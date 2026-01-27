mod auth;

use axum::{
    extract::{Path, State, ws::{Message, WebSocket, WebSocketUpgrade}},
    routing::{get, post},
    Router, Json, response::IntoResponse,
    http::{StatusCode, Method}, 
};
use bollard::Docker;
use bollard::container::{CreateContainerOptions, Config}; 
use bollard::models::HostConfig;
use bollard::image::{CommitContainerOptions, CreateImageOptions}; 
use bollard::exec::{CreateExecOptions, StartExecResults};

use futures::{stream::StreamExt, SinkExt}; 
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use tokio::io::AsyncWriteExt;
use sqlx::postgres::PgPoolOptions;
use serde::Deserialize; 
use uuid::Uuid;
use tower_sessions::{Expiry, MemoryStore, Session, SessionManagerLayer};
use axum::http::header::{CONTENT_TYPE, AUTHORIZATION};

// Store (ContainerID, ShellPath)
type SessionMap = Arc<Mutex<HashMap<String, (String, String)>>>;

#[derive(Clone)]
pub struct AppState {
    docker: Arc<Docker>,
    db: sqlx::PgPool,
    github_id: String,
    github_secret: String,
    sessions: SessionMap,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok(); 

    let docker = Arc::new(Docker::connect_with_local_defaults().unwrap());
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let db = PgPoolOptions::new().connect(&database_url).await.unwrap();

    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS projects (
            owner_username TEXT NOT NULL, slug TEXT NOT NULL,
            image_tag TEXT NOT NULL, markdown TEXT NOT NULL,
            owner_id BIGINT, PRIMARY KEY (owner_username, slug)
        )"#
    ).execute(&db).await.unwrap();

    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false) 
        .with_same_site(tower_sessions::cookie::SameSite::Lax)
        .with_expiry(Expiry::OnInactivity(time::Duration::minutes(60)));

    let state = AppState { 
        docker, 
        db,
        github_id: std::env::var("GITHUB_CLIENT_ID").expect("Missing GITHUB_CLIENT_ID"),
        github_secret: std::env::var("GITHUB_CLIENT_SECRET").expect("Missing GITHUB_CLIENT_SECRET"),
        sessions: Arc::new(Mutex::new(HashMap::new())),
    };

    let app = Router::new()
        .merge(auth::routes()) 
        .route("/api/spawn", post(spawn_handler))      
        .route("/api/publish", post(publish_handler))  
        .route("/api/project/:username/:slug", get(get_project)) 
        .route("/ws/:session_id", get(ws_handler))   
        .layer(tower_http::cors::CorsLayer::new()
            .allow_origin("http://localhost:8080".parse::<axum::http::HeaderValue>().unwrap())
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
    session: Session, 
) -> Result<Json<String>, (StatusCode, String)> {
    let user: Option<auth::User> = session.get("user").await.unwrap();
    if user.is_none() {
        return Err((StatusCode::UNAUTHORIZED, "Please login first".to_string()));
    }
    Ok(Json(Uuid::new_v4().to_string()))
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

    let container_id = {
        let map = state.sessions.lock().unwrap();
        map.get(&payload.container_id).map(|val| val.0.clone())
    };

    let container_id = container_id.ok_or((StatusCode::BAD_REQUEST, "Session expired or container not found".to_string()))?;

    let new_image_tag = format!("trycli-project-{}", payload.slug);
    
    let commit_opts = CommitContainerOptions {
        container: container_id.clone(),
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

    let _ = state.docker.stop_container(&container_id, None).await;

    Ok(Json("Published!".to_string()))
}

async fn get_project(
    Path((username, slug)): Path<(String, String)>, 
    State(state): State<AppState>
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    
    let row: Option<(String, String)> = sqlx::query_as(
        "SELECT image_tag, markdown FROM projects WHERE owner_username = $1 AND slug = $2"
    )
    .bind(username).bind(slug)
    .fetch_optional(&state.db).await.unwrap_or(None);

    let (image_tag, markdown) = match row {
        Some(r) => r,
        None => return Err((StatusCode::NOT_FOUND, "Project not found".to_string())),
    };

    let container_name = format!("trycli-viewer-{}", Uuid::new_v4());
    
    let config = Config {
        image: Some(image_tag),
        tty: Some(true),
        host_config: Some(HostConfig { auto_remove: Some(true), ..Default::default() }),
        ..Default::default()
    };

    state.docker.create_container(
        Some(CreateContainerOptions { name: container_name.clone(), platform: None }), 
        config
    ).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    state.docker.start_container::<String>(&container_name, None)
        .await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let session_id = Uuid::new_v4().to_string();
    {
        let mut map = state.sessions.lock().unwrap();
        map.insert(session_id.clone(), (container_name.clone(), "/bin/bash".to_string()));
    }

    Ok(Json(serde_json::json!({
        "container_id": session_id,
        "markdown": markdown
    })))
}

// --- WEBSOCKET WIZARD ---

async fn ws_handler(
    ws: WebSocketUpgrade, 
    Path(session_id): Path<String>, 
    State(state): State<AppState>
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state, session_id))
}

async fn handle_socket(socket: WebSocket, state: AppState, session_id: String) {
    let existing_session = {
        let map = state.sessions.lock().unwrap();
        map.get(&session_id).cloned()
    };

    if let Some((container_id, shell_path)) = existing_session {
        // Resume session: No auto-type script needed
        attach_to_container(socket, state.docker, container_id, shell_path, None).await;
    } else {
        run_setup_wizard(socket, state, session_id).await;
    }
}

async fn run_setup_wizard(mut socket: WebSocket, state: AppState, session_id: String) {
    async fn send_txt(ws: &mut WebSocket, txt: &str) {
        let _ = ws.send(Message::Text(txt.to_string())).await;
    }

    let green = "\x1b[32m";
    let cyan = "\x1b[36m";
    let reset = "\x1b[0m";
    let clear = "\x1b[2J\x1b[1;1H";

    send_txt(&mut socket, clear).await;
    send_txt(&mut socket, &format!("{}Welcome to TryCLI Setup!{}\r\n\r\n", green, reset)).await;
    
    send_txt(&mut socket, &format!("{}Select Base Image:{}\r\n", cyan, reset)).await;
    send_txt(&mut socket, "1. Ubuntu 22.04 (Heavy, Full-featured)\r\n").await;
    send_txt(&mut socket, "2. Alpine Linux (Lightweight, Fast)\r\n").await;
    send_txt(&mut socket, "3. Debian Bookworm (Stable)\r\n").await;
    send_txt(&mut socket, "Choice [1-3]: ").await;

    let mut distro_choice = 0;
    while let Some(Ok(Message::Text(txt))) = socket.recv().await {
        let input = txt.trim();
        if input == "1" { distro_choice = 1; break; }
        if input == "2" { distro_choice = 2; break; }
        if input == "3" { distro_choice = 3; break; }
    }

    send_txt(&mut socket, "\r\n\r\n").await;

    send_txt(&mut socket, &format!("{}Select Shell (will be installed):{}\r\n", cyan, reset)).await;
    send_txt(&mut socket, "1. Bash (Default)\r\n").await;
    send_txt(&mut socket, "2. Zsh (Oh-My-Zsh ready)\r\n").await;
    send_txt(&mut socket, "3. Fish (Friendly Interactive Shell)\r\n").await;
    send_txt(&mut socket, "Choice [1-3]: ").await;

    let mut shell_choice = 0;
    while let Some(Ok(Message::Text(txt))) = socket.recv().await {
        let input = txt.trim();
        if input == "1" { shell_choice = 1; break; }
        if input == "2" { shell_choice = 2; break; }
        if input == "3" { shell_choice = 3; break; }
    }

    send_txt(&mut socket, &format!("\r\n\r\n{}Provisioning Container... Please wait...{}\r\n", green, reset)).await;

    // --- LOGIC CHANGE 1: Define Image + Install Script ---
    let (image, install_script, final_shell) = match (distro_choice, shell_choice) {
        // Alpine Logic (apk)
        (2, 1) => ("alpine:latest", "apk add --no-cache bash", "/bin/bash"),
        (2, 2) => ("alpine:latest", "apk add --no-cache zsh", "/bin/zsh"),
        (2, 3) => ("alpine:latest", "apk add --no-cache fish", "/usr/bin/fish"),
        // Ubuntu/Debian Logic (apt-get)
        (1, 2) | (3, 2) => (if distro_choice == 1 { "ubuntu:22.04" } else { "debian:bookworm-slim" }, "export DEBIAN_FRONTEND=noninteractive; apt-get update && apt-get install -y zsh", "/usr/bin/zsh"),
        (1, 3) | (3, 3) => (if distro_choice == 1 { "ubuntu:22.04" } else { "debian:bookworm-slim" }, "export DEBIAN_FRONTEND=noninteractive; apt-get update && apt-get install -y fish", "/usr/bin/fish"),
        // Defaults (Bash)
        (1, _) => ("ubuntu:22.04", "true", "/bin/bash"),
        (3, _) => ("debian:bookworm-slim", "true", "/bin/bash"),
        _ => ("debian:bookworm-slim", "true", "/bin/bash"),
    };

    // Auto-pull image
    let _ = state.docker.create_image(Some(CreateImageOptions { from_image: image, ..Default::default() }), None, None).collect::<Vec<_>>().await;

    let container_name = format!("trycli-session-{}", Uuid::new_v4());
    
    // --- LOGIC CHANGE 2: Start a SLEEPER Container ---
    // We do NOT run the install here. We just keep it alive.
    let config = Config {
        image: Some(image.to_string()),
        tty: Some(true),
        open_stdin: Some(true), 
        cmd: Some(vec!["tail".to_string(), "-f".to_string(), "/dev/null".to_string()]), // Keep alive
        host_config: Some(HostConfig { 
            memory: Some(512 * 1024 * 1024), 
            auto_remove: Some(true), 
            ..Default::default() 
        }),
        ..Default::default()
    };

    match state.docker.create_container(
        Some(CreateContainerOptions { name: container_name.clone(), platform: None }), config
    ).await {
        Ok(_) => {
            state.docker.start_container::<String>(&container_name, None).await.unwrap();
            
            {
                let mut map = state.sessions.lock().unwrap();
                map.insert(session_id, (container_name.clone(), final_shell.to_string()));
            }

            // --- LOGIC CHANGE 3: Construct the Auto-Type Command ---
            // We tell the shell to: Install -> Echo Done -> Switch to Final Shell
            let auto_type_cmd = format!("{} && echo '\r\n\r\n--- READY ---' && exec {}\n", install_script, final_shell);

            // Connect using /bin/sh (safe), but inject the script!
            attach_to_container(socket, state.docker, container_name, "/bin/sh".to_string(), Some(auto_type_cmd)).await;
        },
        Err(e) => {
            send_txt(&mut socket, &format!("Error creating container: {}", e)).await;
        }
    }
}

// --- LOGIC CHANGE 4: Add `initial_input` argument ---
async fn attach_to_container(
    socket: WebSocket, 
    docker: Arc<Docker>, 
    container_name: String, 
    shell_path: String,
    initial_input: Option<String>
) {
    let config = CreateExecOptions {
        attach_stdout: Some(true), attach_stderr: Some(true), attach_stdin: Some(true),
        tty: Some(true), 
        cmd: Some(vec![shell_path]), 
        ..Default::default()
    };
    
    let exec = match docker.create_exec(&container_name, config).await {
        Ok(e) => e,
        Err(_) => return,
    };
    
    if let StartExecResults::Attached { mut output, mut input } = docker.start_exec(&exec.id, None).await.unwrap() {
        let (mut sender, mut receiver) = socket.split();

        // --- LOGIC CHANGE 5: Inject the Install Script ---
        if let Some(script) = initial_input {
            let _ = input.write_all(script.as_bytes()).await;
        }
        
        let mut send_task = tokio::spawn(async move {
            while let Some(Ok(Message::Text(text))) = receiver.next().await {
                let _ = input.write_all(text.as_bytes()).await;
            }
        });

        let mut recv_task = tokio::spawn(async move {
            while let Some(Ok(msg)) = output.next().await {
                let _ = sender.send(Message::Text(msg.to_string().into())).await;
            }
        });

        let _ = tokio::select! {
            _ = &mut send_task => {},
            _ = &mut recv_task => {},
        };
    }
}