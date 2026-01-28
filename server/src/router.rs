use axum::{
    routing::get,
    Router,
    http::Method,
};
use tower_sessions::{Expiry, MemoryStore, SessionManagerLayer};
use axum::http::header::{CONTENT_TYPE, AUTHORIZATION};
use crate::state::AppState;
use crate::handlers::{auth, project, spawn};
use crate::services::websocket;

pub fn create_router(state: AppState) -> Result<Router, Box<dyn std::error::Error>> {
    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false) 
        .with_same_site(tower_sessions::cookie::SameSite::Lax)
        .with_expiry(Expiry::OnInactivity(time::Duration::minutes(60)));

    let app = Router::new()
        .merge(auth::routes())
        .merge(spawn::routes())
        .merge(project::routes())
        .route("/ws/:session_id", get(websocket::ws_handler))
        .layer(tower_http::cors::CorsLayer::new()
            .allow_origin("http://localhost:8080".parse::<axum::http::HeaderValue>()?)
            .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
            .allow_headers([CONTENT_TYPE, AUTHORIZATION])
            .allow_credentials(true) 
        )
        .layer(session_layer) 
        .with_state(state);

    Ok(app)
}
