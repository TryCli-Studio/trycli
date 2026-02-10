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
    // FIX: Map error instead of unwrap
    let user: Option<User> = session.get("user")
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Session Error: {}", e)))?;
        
    if user.is_none() {
        return Err((StatusCode::UNAUTHORIZED, "Please login first".to_string()));
    }
    let session_id = Uuid::new_v4().to_string();

    if let (Some(username), Some(slug)) = (view_query.username, view_query.slug) {
        let project_id: Option<i64> = sqlx::query_scalar(
            "SELECT id FROM projects WHERE LOWER(owner_username) = LOWER($1) AND LOWER(slug) = LOWER($2)"
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
