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
    let db = PgPoolOptions::new()
        .max_connections(20) // Keeps you safely under the 22 limit
        .min_connections(2)  // Keeps a couple open so the first users don't have to wait
        .connect(&database_url)
        .await?;

    // 3. Conditional Migrations (Crucial for Horizontal Scaling)
    if std::env::var("RUN_MIGRATIONS").unwrap_or_default() == "true" {
        println!("Running database migrations...");
        sqlx::migrate!("./migrations")
            .run(&db)
            .await?;
    } else {
        println!("Skipping migrations (RUN_MIGRATIONS != true)");
    }

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
