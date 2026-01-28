use axum::{
    routing::post,
    Router, Json,
    http::StatusCode,
};
use tower_sessions::Session;
use uuid::Uuid;
use crate::state::AppState;
use crate::models::User;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/spawn", post(spawn_handler))
}

pub async fn spawn_handler(
    session: Session, 
) -> Result<Json<String>, (StatusCode, String)> {
    // FIX: Map error instead of unwrap
    let user: Option<User> = session.get("user")
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Session Error: {}", e)))?;
        
    if user.is_none() {
        return Err((StatusCode::UNAUTHORIZED, "Please login first".to_string()));
    }
    Ok(Json(Uuid::new_v4().to_string()))
}
