use bollard::Docker;
use sqlx::postgres::PgPoolOptions;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use crate::state::AppState;

pub async fn setup_database_and_docker() -> Result<AppState, Box<dyn std::error::Error>> {
    // 1. Docker (Propagate error instead of unwrap)
    let docker = Arc::new(Docker::connect_with_local_defaults()?);
    
    // 2. DB (Propagate error instead of unwrap)
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let db = PgPoolOptions::new().connect(&database_url).await?;

    // 3. Schema (Propagate error)
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS projects (
            owner_username TEXT NOT NULL, 
            slug TEXT NOT NULL,
            image_tag TEXT NOT NULL, 
            markdown TEXT NOT NULL,
            shell TEXT NOT NULL DEFAULT '/bin/bash', 
            owner_id BIGINT, 
            
            -- NEW: Security Fields
            embed_token TEXT,
            allowed_origins TEXT, -- Stored as comma-separated values
            
            PRIMARY KEY (owner_username, slug)
        )"#
    ).execute(&db).await?;

    let state = AppState { 
        docker, 
        db,
        github_id: std::env::var("GITHUB_CLIENT_ID").expect("Missing GITHUB_CLIENT_ID"),
        github_secret: std::env::var("GITHUB_CLIENT_SECRET").expect("Missing GITHUB_CLIENT_SECRET"),
        sessions: Arc::new(Mutex::new(HashMap::new())),
    };

    Ok(state)
}
