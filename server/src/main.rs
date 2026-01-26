mod auth; // Register auth module

use axum::{
    extract::{Path, State, ws::{Message, WebSocket, WebSocketUpgrade}},
    routing::{get, post},
    Router, Json, response::IntoResponse,
    http::{StatusCode, Method}, 
};
use bollard::Docker;
// Import HostConfig for Resource Limits
use bollard::container::{CreateContainerOptions, Config}; 
use bollard::models::HostConfig;
use bollard::image::{CommitContainerOptions, CreateImageOptions}; // <--- ADDED CreateImageOptions
use bollard::exec::{CreateExecOptions, StartExecResults};

use futures::{stream::StreamExt, SinkExt, TryStreamExt}; // <--- ADDED TryStreamExt for image pull
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use sqlx::postgres::PgPoolOptions;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use tower_sessions::{Expiry, MemoryStore, Session, SessionManagerLayer};
use axum::http::header::{CONTENT_TYPE, AUTHORIZATION};

#[derive(Clone)]
pub struct AppState {
    docker: Arc<Docker>,
    db: sqlx::PgPool,
    github_id: String,
    github_secret: String,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok(); 

    let docker = Arc::new(Docker::connect_with_local_defaults().unwrap());
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set in .env");

    let db = PgPoolOptions::new()
        .connect(&database_url).await.unwrap();

    // --- FIX 1: CORRECT TABLE SCHEMA ---
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS projects (
            owner_username TEXT NOT NULL,
            slug TEXT NOT NULL,
            image_tag TEXT NOT NULL,
            markdown TEXT NOT NULL,
            owner_id BIGINT,
            PRIMARY KEY (owner_username, slug)
        )
        "#
    ).execute(&db).await.unwrap();

    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false) 
        .with_same_site(tower_sessions::cookie::SameSite::Lax) // Ensure Lax for redirects
        .with_expiry(Expiry::OnInactivity(time::Duration::minutes(60)));

    let state = AppState { 
        docker, 
        db,
        github_id: std::env::var("GITHUB_CLIENT_ID").expect("Missing GITHUB_CLIENT_ID"),
        github_secret: std::env::var("GITHUB_CLIENT_SECRET").expect("Missing GITHUB_CLIENT_SECRET"),
    };

    let app = Router::new()
        .merge(auth::routes()) 
        .route("/api/spawn", post(spawn_handler))      
        .route("/api/publish", post(publish_handler))  
        // --- FIX 2: Updated Route for Username ---
        .route("/api/project/:username/:slug", get(get_project)) 
        .route("/ws/:container_id", get(ws_handler))   
        .layer(tower_http::cors::CorsLayer::new()
            .allow_origin("http://localhost:8080".parse::<axum::http::HeaderValue>().unwrap())
            // --- FIX 3: Add OPTIONS for Browser Preflight ---
            .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
            .allow_headers([CONTENT_TYPE, AUTHORIZATION])
            .allow_credentials(true) 
        )
        .layer(session_layer) 
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Server listening on port 3000...");
    axum::serve(listener, app).await.unwrap();
}

// --- HANDLERS ---

async fn spawn_handler(
    State(state): State<AppState>,
    session: Session, 
) -> Result<Json<String>, (StatusCode, String)> {
    
    let user: Option<auth::User> = session.get("user").await.unwrap();
    if user.is_none() {
        return Err((StatusCode::UNAUTHORIZED, "Please login first".to_string()));
    }

    let container_name = format!("trycli-session-{}", Uuid::new_v4());
    let image_name = "debian:bookworm-slim";

    // --- FIX 4: AUTO-PULL IMAGE (Prevent 500 Error) ---
    // If image is missing, this downloads it.
    let pull_options = CreateImageOptions {
        from_image: image_name,
        ..Default::default()
    };
    let _ = state.docker.create_image(Some(pull_options), None, None)
        .map_err(|e| println!("Pulling status: {:?}", e))
        .collect::<Vec<_>>() 
        .await;

    // OPTIMIZATION: Host Config (Restored)
    let host_config = HostConfig {
        memory: Some(512 * 1024 * 1024), 
        nano_cpus: Some(500_000_000), 
        auto_remove: Some(true), 
        ..Default::default()
    };

    let config = Config {
        image: Some(image_name), 
        tty: Some(true),
        host_config: Some(host_config),
        ..Default::default()
    };
    
    state.docker.create_container(
        Some(CreateContainerOptions { name: container_name.clone(), platform: None }), 
        config
    ).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    state.docker.start_container::<String>(&container_name, None).await.unwrap();
    
    Ok(Json(container_name))
}

#[derive(Deserialize)]
struct PublishRequest {
    container_id: String,
    slug: String,
    markdown: String,
}

async fn publish_handler(
    State(state): State<AppState>,
    session: Session, 
    Json(payload): Json<PublishRequest>
) -> Result<Json<String>, (StatusCode, String)> {

    let user: Option<auth::User> = session.get("user").await.unwrap();
    let user = user.ok_or((StatusCode::UNAUTHORIZED, "Unauthorized".to_string()))?;
    
    if payload.container_id.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Container not ready yet.".to_string()));
    }

    let new_image_tag = format!("trycli-project-{}", payload.slug);
    
    let commit_opts = CommitContainerOptions {
        container: payload.container_id.clone(),
        repo: new_image_tag.clone(),
        ..Default::default()
    };
    
    state.docker.commit_container::<String, String>(commit_opts, Default::default())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Docker Error: {}", e)))?;

    sqlx::query("INSERT INTO projects (slug, image_tag, markdown, owner_id, owner_username) VALUES ($1, $2, $3, $4, $5)")
        .bind(&payload.slug)
        .bind(&new_image_tag)
        .bind(&payload.markdown)
        .bind(user.id)          
        .bind(&user.login)     
        .execute(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB Error: {}", e)))?;

    let _ = state.docker.stop_container(&payload.container_id, None).await;

    Ok(Json("Published!".to_string()))
}

#[derive(Serialize)]
struct ProjectResponse {
    container_id: String,
    markdown: String,
}

// --- FIX 5: Updated Signature for Username ---
async fn get_project(
    Path((username, slug)): Path<(String, String)>, 
    State(state): State<AppState>
) -> Result<Json<ProjectResponse>, (StatusCode, String)> {
    
    let row: Option<(String, String)> = sqlx::query_as(
        "SELECT image_tag, markdown FROM projects WHERE owner_username = $1 AND slug = $2"
    )
    .bind(username)
    .bind(slug)
    .fetch_optional(&state.db)
    .await
    .unwrap_or(None);

    let (image_tag, markdown) = match row {
        Some(r) => r,
        None => return Err((StatusCode::NOT_FOUND, "Project not found".to_string())),
    };

    let container_name = format!("trycli-viewer-{}", Uuid::new_v4());
    
    let config = Config {
        image: Some(image_tag),
        tty: Some(true),
        ..Default::default()
    };

    state.docker.create_container(
        Some(CreateContainerOptions { name: container_name.clone(), platform: None }), 
        config
    ).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    state.docker.start_container::<String>(&container_name, None)
        .await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(ProjectResponse { container_id: container_name, markdown }))
}

async fn ws_handler(ws: WebSocketUpgrade, Path(id): Path<String>, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_terminal(socket, state.docker, id))
}

async fn handle_terminal(socket: WebSocket, docker: Arc<Docker>, id: String) {
    let config = CreateExecOptions {
        attach_stdout: Some(true), attach_stderr: Some(true), attach_stdin: Some(true),
        tty: Some(true), cmd: Some(vec!["/bin/bash"]), ..Default::default()
    };
    
    let exec = match docker.create_exec(&id, config).await {
        Ok(e) => e,
        Err(e) => {
            println!("Failed to create exec for {}: {}", id, e);
            return; 
        }
    };
    
    if let StartExecResults::Attached { mut output, mut input } = docker.start_exec(&exec.id, None).await.unwrap() {
        let (mut sender, mut receiver) = socket.split();
        
        let _send_task = tokio::spawn(async move {
            while let Some(Ok(Message::Text(text))) = receiver.next().await {
                let _ = input.write_all(text.as_bytes()).await;
            }
        });

        while let Some(Ok(msg)) = output.next().await {
             let _ = sender.send(Message::Text(msg.to_string().into())).await;
        }
    }
}