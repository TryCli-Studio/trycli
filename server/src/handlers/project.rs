use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};

use bollard::container::{Config, CreateContainerOptions};
use bollard::image::CommitContainerOptions;
use bollard::models::HostConfig;
use tower_sessions::Session;
use uuid::Uuid;
use std::collections::HashMap;
use serde::Deserialize;
use crate::state::{AppState, SessionContext};
use crate::models::{User, ProjectSummary, PublishRequest};

#[derive(Deserialize)]
pub struct SearchQuery {
    q: String,
}

#[derive(Deserialize)]
pub struct EmbedQuery {
    key: Option<String>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/my-projects", get(list_user_projects))
        .route("/api/project/:username/:slug", get(get_project))
        .route("/api/search-projects", get(search_projects))
        .route("/api/publish", post(publish_handler))
}

pub async fn list_user_projects(
    State(state): State<AppState>,
    session: Session,
) -> Result<Json<Vec<ProjectSummary>>, (StatusCode, String)> {
    // FIX: Safely handle session retrieval instead of unwrap()
    let user: Option<User> = session.get("user")
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Session Error: {}", e)))?;
        
    let user = user.ok_or((StatusCode::UNAUTHORIZED, "Unauthorized".to_string()))?;

    let projects = sqlx::query_as::<_, ProjectSummary>(
        "SELECT slug, image_tag FROM projects WHERE owner_id = $1 ORDER BY slug ASC"
    )
    .bind(user.id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        eprintln!("Database error fetching projects: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "Failed to fetch projects".to_string())
    })?;

    Ok(Json(projects))
}

pub async fn search_projects(
    State(state): State<AppState>,
    session: Session,
    Query(search): Query<SearchQuery>,
) -> Result<Json<Vec<ProjectSummary>>, (StatusCode, String)> {
    let user: Option<User> = session.get("user")
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Session Error: {}", e)))?;
        
    let user = user.ok_or((StatusCode::UNAUTHORIZED, "Unauthorized".to_string()))?;

    let query_term = format!("%{}%", search.q);
    
    // Use PostgreSQL ILIKE for case-insensitive fuzzy search
    let projects = sqlx::query_as::<_, ProjectSummary>(
        "SELECT slug, image_tag FROM projects WHERE owner_id = $1 AND slug ILIKE $2 ORDER BY slug ASC LIMIT 10"
    )
    .bind(user.id)
    .bind(query_term)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        eprintln!("Database error searching projects: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "Failed to search projects".to_string())
    })?;

    Ok(Json(projects))
}

pub async fn publish_handler(
    State(state): State<AppState>,
    session: Session, 
    Json(payload): Json<PublishRequest>
) -> Result<Json<String>, (StatusCode, String)> {
    let user: Option<User> = session.get("user")
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Session Error: {}", e)))?;
        
    let user = user.ok_or((StatusCode::UNAUTHORIZED, "Unauthorized".to_string()))?;

    // 1. Get Session Info & Verify Ownership
    let (container_name, shell_path) = {
        let map = state.lock_sessions();
        match map.get(&payload.container_id) {
            Some(ctx) => {
                // STRICT CHECK: Does the user publishing own the container?
                if ctx.owner_id != Some(user.id) {
                     return Err((StatusCode::FORBIDDEN, "You do not own this session".to_string()));
                }
                // Minimize clone: only clone the strings needed for the commit options
                (ctx.container_name.clone(), ctx.shell.clone())
            },
            None => return Err((StatusCode::BAD_REQUEST, "Session expired".to_string())),
        }
    };

    let new_image_tag = format!("trycli-project-{}", payload.slug);

    // 2. Prepare Commit Options
    let commit_opts = CommitContainerOptions {
        container: container_name.clone(),
        repo: new_image_tag.clone(),
        ..Default::default()
    };

    // 3. Prepare Config
    let config = Config {
        cmd: Some(vec![shell_path.clone()]),
        env: Some(vec![format!("SHELL={}", shell_path)]),
        ..Default::default()
    };

    // 4. Commit
    state.docker.commit_container(commit_opts, config)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Docker Commit Error: {}", e)))?;

    // NEW: Generate Token and Process Origins
    let embed_token = Uuid::new_v4().to_string();
    let origins_str = payload.allowed_origins.unwrap_or_default(); // Store as provided

    // NEW: Insert with Security Fields
    sqlx::query(
        "INSERT INTO projects (slug, image_tag, markdown, owner_id, owner_username, shell, embed_token, allowed_origins) 
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         ON CONFLICT (owner_username, slug) 
         DO UPDATE SET image_tag = $2, markdown = $3, shell = $6, embed_token = $7, allowed_origins = $8"
    )
        .bind(&payload.slug)
        .bind(&new_image_tag)
        .bind(&payload.markdown)
        .bind(user.id)          
        .bind(&user.login)
        .bind(&shell_path)
        .bind(&embed_token)
        .bind(&origins_str)
        .execute(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB Error: {}", e)))?;

    let _ = state.docker.stop_container(&container_name, None).await;

    // Return the token to the user for immediate use
    Ok(Json(format!("Published! Embed Token: {}", embed_token)))
}

pub async fn get_project(
    Path((username, slug)): Path<(String, String)>, 
    headers: HeaderMap,           // NEW: Capture Headers
    Query(query): Query<EmbedQuery>, // NEW: Capture Query Params
    State(state): State<AppState>
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    
    // 1. Fetch Project Data + Security Fields
    let row_result = sqlx::query_as::<_, (String, String, String, Option<String>, Option<String>)>(
        "SELECT image_tag, markdown, shell, embed_token, allowed_origins FROM projects WHERE owner_username = $1 AND slug = $2"
    )
    .bind(&username).bind(&slug)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB Read Error: {}", e)))?;

    let (image_tag, markdown, shell, token, origins_raw) = match row_result {
        Some(r) => r,
        None => return Err((StatusCode::NOT_FOUND, "Project not found".to_string())),
    };

    // 2. === AUTHORIZATION LOGIC ===
    let referer = headers.get(header::REFERER)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    
    // Parse the allowed list
    let allowed_list: Vec<&str> = origins_raw.as_deref().unwrap_or("")
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    // Check 1: Is it our own frontend? (Always allow)
    // In production, change "localhost" to your actual domain
    let is_self = referer.contains("localhost") || referer.contains("127.0.0.1") || referer.contains("trycli.com");

    // Check 2: Is the Referer in the whitelist?
    // We check if the referer *starts with* the allowed origin to handle paths
    let is_whitelisted = allowed_list.iter().any(|&origin| {
        // Handle cases where user might enter "example.com" or "https://example.com"
        let clean_origin = origin.trim_start_matches("https://").trim_start_matches("http://");
        referer.contains(clean_origin)
    });

    // Check 3: Is the provided token correct?
    let is_valid_token = token.as_deref() == query.key.as_deref();

    // DECISION MATRIX
    if !is_self {
        // If we have security rules set up (either a whitelist or a token exists)
        if !allowed_list.is_empty() || token.is_some() {
            // We need EITHER a whitelist match OR a valid token
            if !is_whitelisted && !is_valid_token {
                tracing::warn!("Blocked Unauthorized Embed: Referer='{}', Slug='{}'", referer, slug);
                return Err((StatusCode::FORBIDDEN, "Access Denied: Domain not authorized.".to_string()));
            }
        }
    }
    

    let container_name = format!("trycli-viewer-{}", Uuid::new_v4());
    let session_id = Uuid::new_v4().to_string();

    let config = Config {
        image: Some(image_tag),
        tty: Some(true),
        env: Some(vec![
            "LANG=C.UTF-8".to_string(), 
            "LC_ALL=C.UTF-8".to_string(),
            "TERM=xterm-256color".to_string(),
            format!("SHELL={}", shell) 
        ]),
        labels: Some(HashMap::from([
            ("managed_by".to_string(), "trycli".to_string()),
            ("type".to_string(), "viewer".to_string())
        ])),
        host_config: Some(HostConfig { 
            memory: Some(512 * 1024 * 1024),  
            auto_remove: Some(true), 
            ..Default::default() 
        }),
        ..Default::default()
    };

    state.docker.create_container(
        Some(CreateContainerOptions { name: container_name.clone(), platform: None }), 
        config
    ).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Docker Create Error: {}", e)))?;

    state.docker.start_container::<String>(&container_name, None)
        .await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Docker Start Error: {}", e)))?;

    {
        let mut map = state.lock_sessions();
        map.insert(session_id.clone(), SessionContext {
            container_name: container_name.clone(), 
            shell,
            owner_id: None 
        }); 
    }
    
    Ok(Json(serde_json::json!({
        "container_id": session_id,
        "markdown": markdown
    })))
}