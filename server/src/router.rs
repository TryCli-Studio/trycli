use axum::{
    routing::get,
    Router,
    http::Method,
};
use tower_sessions::{Expiry, MemoryStore, SessionManagerLayer};
use axum::http::header::{CONTENT_TYPE, AUTHORIZATION};
use crate::state::AppState;
use crate::handlers::{auth, project, spawn, analytics, admin, oembed};
use crate::services::websocket;

pub fn create_router(state: AppState) -> Result<Router, Box<dyn std::error::Error>> {
    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false) 
        .with_same_site(tower_sessions::cookie::SameSite::Strict)
        .with_expiry(Expiry::OnInactivity(time::Duration::minutes(60)));

    // 1. DYNAMIC ORIGIN: Reads from env, defaults to localhost for dev
    let frontend_url = std::env::var("FRONTEND_URL")
        .unwrap_or_else(|_| "http://localhost:8080".to_string());

    let app = Router::new()
        .merge(auth::routes())
        .merge(spawn::routes())
        .merge(project::routes())
        .merge(admin::routes())
        .route("/ws/{session_id}", get(websocket::ws_handler))
        .route("/api/analytics", get(analytics::get_analytics))
        .route("/api/oembed", get(oembed::oembed_handler))
        .layer(tower_http::cors::CorsLayer::new()
            .allow_origin(frontend_url.parse::<axum::http::HeaderValue>()?)
            // 2. ALLOW DELETE METHOD
            .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
            .allow_headers([CONTENT_TYPE, AUTHORIZATION])
            .allow_credentials(true) 
        )
        .layer(session_layer) 
        .with_state(state);

    Ok(app)
}