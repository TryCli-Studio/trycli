use axum::{
    extract::{Query, State},
    routing::post,
    Router, Json,
    http::StatusCode,
};
use serde::Deserialize;
use tower_sessions::Session;
use uuid::Uuid;
use crate::state::AppState;
use crate::models::{User, AnalyticsEventType};
use bollard::container::RemoveContainerOptions;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/spawn", post(spawn_handler))
}

#[derive(Deserialize)]
pub struct SpawnViewQuery {
    pub username: Option<String>,
    pub slug: Option<String>,
}

pub async fn spawn_handler(
    State(state): State<AppState>,
    session: Session, 
    Query(view_query): Query<SpawnViewQuery>,
) -> Result<Json<String>, (StatusCode, String)> {
    let user: Option<User> = session.get("user")
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Session Error: {}", e)))?;
        
    let user = user.ok_or((StatusCode::UNAUTHORIZED, "Please login first".to_string()))?;

    // --- FIX: DEDUPLICATION LOGIC ---
    let old_session_to_kill = {
        let mut sessions = state.lock_sessions();
        let mut target_id = None;

        // Find any existing "Builder" session (project_slug is None) for this user
        for (id, ctx) in sessions.iter() {
            if ctx.owner_id == Some(user.id) && ctx.project_slug.is_none() {
                target_id = Some((id.clone(), ctx.container_name.clone()));
                break; 
            }
        }

        // Remove from map immediately to prevent new connections to it
        if let Some((sid, _)) = &target_id {
            sessions.remove(sid);
        }
        target_id
    };

    // If we found an old session, tell Docker to kill it in the background
    if let Some((_, container_name)) = old_session_to_kill {
        if container_name != "INITIALIZING" {
            let docker = state.docker.clone();
            tokio::spawn(async move {
                let _ = docker.remove_container(&container_name, Some(RemoveContainerOptions {
                    force: true, ..Default::default()
                })).await;
            });
        }
    }
    let session_id = Uuid::new_v4().to_string();

    if let (Some(username), Some(slug)) = (view_query.username, view_query.slug) {
        let project_id: Option<i64> = sqlx::query_scalar(
            "SELECT p.id 
             FROM projects p
             JOIN users u ON p.owner_id = u.id
             WHERE LOWER(u.username) = LOWER($1) AND LOWER(p.slug) = LOWER($2)"
        )
        .bind(&username)
        .bind(&slug)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB Error: {}", e)))?;

        if let Some(project_id) = project_id {
            let _ = sqlx::query(
                "INSERT INTO analytics_events (project_id, event_type) VALUES ($1, $2)"
            )
            .bind(project_id)
            .bind(AnalyticsEventType::View)
            .execute(&state.db)
            .await;
        }
    }

    Ok(Json(session_id))
}
