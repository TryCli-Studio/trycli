use axum::{
    routing::post,
    Router, Json,
    extract::State, // Add State
    http::StatusCode,
};
use tower_sessions::Session;
use uuid::Uuid;
use crate::state::AppState;
use crate::models::User;
use bollard::container::RemoveContainerOptions;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/spawn", post(spawn_handler))
}

pub async fn spawn_handler(
    State(state): State<AppState>,
    session: Session, 
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
    // --------------------------------

    Ok(Json(Uuid::new_v4().to_string()))
}