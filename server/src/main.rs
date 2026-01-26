use axum::{
    extract::{Path, State, ws::{Message, WebSocket, WebSocketUpgrade}},
    routing::{get, post},
    Router, Json, response::IntoResponse,
    http::StatusCode, // Import StatusCode for error handling
};
use bollard::Docker;
// BOLLARD 0.15 IMPORTS
use bollard::container::{CreateContainerOptions, Config}; 
use bollard::image::{CommitContainerOptions}; 
use bollard::exec::{CreateExecOptions, StartExecResults};

use futures::{stream::StreamExt, SinkExt};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use sqlx::postgres::PgPoolOptions;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Shared State
#[derive(Clone)]
struct AppState {
    docker: Arc<Docker>,
    db: sqlx::PgPool,
}

#[tokio::main]
async fn main() {
    let docker = Arc::new(Docker::connect_with_local_defaults().unwrap());
    
    // Connect to Postgres on port 5433 (as per previous fix)
    let db = PgPoolOptions::new()
        .connect("postgres://postgres:password@localhost:5433/postgres").await.unwrap();

    // Init DB
    sqlx::query("CREATE TABLE IF NOT EXISTS projects (slug TEXT PRIMARY KEY, image_tag TEXT, markdown TEXT)")
        .execute(&db).await.unwrap();

    let state = AppState { docker, db };

    // Define Routes
    let app = Router::new()
        .route("/api/spawn", post(spawn_handler))      
        .route("/api/publish", post(publish_handler))  
        .route("/api/project/:slug", get(get_project)) 
        .route("/ws/:container_id", get(ws_handler))   
        .layer(tower_http::cors::CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Server listening on port 3000...");
    axum::serve(listener, app).await.unwrap();
}

// --- API HANDLERS ---

async fn spawn_handler(State(state): State<AppState>) -> Json<String> {
    let container_name = format!("trycli-session-{}", Uuid::new_v4());
    
    let config = Config {
        image: Some("ubuntu:latest"), 
        tty: Some(true),
        ..Default::default()
    };
    
    let _ = state.docker.create_container(
        Some(CreateContainerOptions { name: container_name.clone(), platform: None }), 
        config
    ).await.unwrap();
    
    state.docker.start_container::<String>(&container_name, None).await.unwrap();
    
    Json(container_name) 
}

#[derive(Deserialize)]
struct PublishRequest {
    container_id: String,
    slug: String,
    markdown: String,
}

// Change Return Type to Result so we can return Errors instead of crashing
async fn publish_handler(
    State(state): State<AppState>, 
    Json(payload): Json<PublishRequest>
) -> Result<Json<String>, (StatusCode, String)> {
    
    // GUARD 1: Check if container_id is empty (Prevents Panic #1)
    if payload.container_id.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Container not ready yet. Please wait.".to_string()));
    }

    let new_image_tag = format!("trycli-project-{}", payload.slug);
    
    let commit_opts = CommitContainerOptions {
        container: payload.container_id.clone(),
        repo: new_image_tag.clone(),
        ..Default::default()
    };
    
    // GUARD 2: Handle Docker errors gracefully
    state.docker.commit_container::<String, String>(commit_opts, Default::default())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Docker Error: {}", e)))?;

    sqlx::query("INSERT INTO projects (slug, image_tag, markdown) VALUES ($1, $2, $3)")
        .bind(&payload.slug)
        .bind(&new_image_tag)
        .bind(&payload.markdown)
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

async fn get_project(
    Path(slug): Path<String>, 
    State(state): State<AppState>
) -> Result<Json<ProjectResponse>, (StatusCode, String)> {
    
    // GUARD 3: Use fetch_optional to handle missing projects (Prevents Panic #2)
    let row: Option<(String, String)> = sqlx::query_as("SELECT image_tag, markdown FROM projects WHERE slug = $1")
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
    
    // GUARD 4: Don't crash if container doesn't exist for WS
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