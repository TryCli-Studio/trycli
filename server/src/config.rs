use bollard::Docker;
use sqlx::postgres::PgPoolOptions;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use crate::state::AppState;
use dashmap::DashMap;

pub async fn setup_database_and_docker() -> anyhow::Result<AppState> {
    // 1. Docker setup 
    let docker = Arc::new(Docker::connect_with_local_defaults()?);
    
    // 2. DB Connection 
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let db = PgPoolOptions::new().connect(&database_url).await?;

    sqlx::migrate!("./migrations")
        .run(&db)
        .await?;

    let state = AppState { 
        docker, 
        db,
        github_id: std::env::var("GITHUB_CLIENT_ID").expect("Missing GITHUB_CLIENT_ID"),
        github_secret: std::env::var("GITHUB_CLIENT_SECRET").expect("Missing GITHUB_CLIENT_SECRET"),
        sessions: Arc::new(Mutex::new(HashMap::new())),
        whitelist_rate_limiters: Arc::new(DashMap::new()),
    };

    Ok(state)
}
