mod models;
mod state;
mod error;
mod config;
mod router;
mod services {
    pub mod docker;
    pub mod websocket;
}
mod handlers {
    pub mod auth;
    pub mod project;
    pub mod spawn;
    pub mod analytics;
    pub mod admin;
    pub mod oembed;
}

// Re-export so your handlers can use `crate::Result` and `crate::AppError`
pub use error::{AppError, Result};
use services::docker::start_background_reaper;

#[tokio::main]
// FIX: Use anyhow::Result<()> here so it doesn't clash with your custom crate::Result
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok(); 
    
    // Setup database and Docker
    let state = config::setup_database_and_docker().await?;

    // Spawn background reaper
    let docker_reaper = state.docker.clone();
    let sessions_reaper = state.sessions.clone();
    let db_reaper = state.db.clone();
    tokio::spawn(async move {
        start_background_reaper(docker_reaper, sessions_reaper, db_reaper).await;
    });

    // Create router with all routes
    let app = router::create_router(state)?;

    // Start server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    println!("Server listening on port 3000...");
    axum::serve(listener, app).await?;
    
    Ok(())
}