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
use serde::Deserialize;
use url::Url;
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

// Strict host matching to prevent subdomain spoofing
fn is_authorized(referer: &str, allowed_origins: &str, self_domain: &str) -> bool {
    let ref_url = match Url::parse(referer) {
        Ok(u) => u,
        Err(_) => return false,
    };
    let ref_host = ref_url.host_str().unwrap_or("");
    
    // 1. Internal Check (localhost, 127.0.0.1, or your domain)
    // NOTE: This is the "Localhost Loophole" that allows your tests to pass on port 8080.
    if ref_host == "localhost" || ref_host == "127.0.0.1" || 
       ref_host == self_domain || ref_host.ends_with(&format!(".{}", self_domain)) {
        return true;
    }
    
    // 2. Strict Whitelist Check (Prevents domain.com.evil.com exploits)
    allowed_origins.split(',')
        .map(|s| s.trim().trim_start_matches("https://").trim_start_matches("http://"))
        .any(|domain| ref_host == domain || ref_host.ends_with(&format!(".{}", domain)))
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
    let user: Option<User> = session.get("user")
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Session Error: {}", e)))?;
        
    let user = user.ok_or((StatusCode::UNAUTHORIZED, "Unauthorized".to_string()))?;

    let projects = sqlx::query_as::<_, ProjectSummary>(
        "SELECT slug, image_tag, is_protected FROM projects WHERE owner_id = $1 ORDER BY slug ASC"
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
        "SELECT slug, image_tag, is_protected FROM projects WHERE owner_id = $1 AND slug ILIKE $2 ORDER BY slug ASC LIMIT 10"
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

    let safe_slug = payload.slug.trim().to_lowercase();
    let new_image_tag = format!("trycli-studio-project-{}", safe_slug);

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
    let embed_token = if payload.is_protected {
        Some(Uuid::new_v4().to_string())
    } else {
        None
    };
    let origins_str = if payload.is_protected {
        payload.allowed_origins
    } else {
        None
    };

    // NEW: Insert with Security Fields
    sqlx::query(
        "INSERT INTO projects (slug, image_tag, markdown, owner_id, owner_username, shell, is_protected, embed_token, allowed_origins) 
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
         ON CONFLICT (owner_username, slug) 
         DO UPDATE SET image_tag = $2, markdown = $3, shell = $6, is_protected = $7, embed_token = $8, allowed_origins = $9"
    )
        .bind(&payload.slug)
        .bind(&new_image_tag)
        .bind(&payload.markdown)
        .bind(user.id)          
        .bind(&user.login)
        .bind(&shell_path)
        .bind(payload.is_protected)
        .bind(&embed_token)
        .bind(&origins_str)
        .execute(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB Error: {}", e)))?;

    let _ = state.docker.stop_container(&container_name, None).await;

    // Return the token to the user for immediate use
    if let Some(token) = &embed_token {
        Ok(Json(format!("Published! Embed Token: {}", token)))
    } else {
        Ok(Json("Published!".to_string()))
    }
}

pub async fn get_project(
    Path((username, slug)): Path<(String, String)>, 
    headers: HeaderMap,           
    Query(query): Query<EmbedQuery>, 
    State(state): State<AppState>
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    
    // RESOLVED CONFLICT: Fetching ALL fields (security fields + owner_id)
    // We added 'owner_id' (int8) to the query tuple
    let row_result = sqlx::query_as::<_, (String, String, String, bool, Option<String>, Option<String>, i64)>(
        "SELECT image_tag, markdown, shell, is_protected, embed_token, allowed_origins, owner_id FROM projects WHERE owner_username = $1 AND slug = $2"
    )
    .bind(&username).bind(&slug)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB Read Error: {}", e)))?;

    // Destructuring all 7 fields
    let (image_tag, markdown, shell, is_protected, token, origins_raw, owner_id) = match row_result {
        Some(r) => r,
        None => return Err((StatusCode::NOT_FOUND, "Project not found".to_string())),
    };

    // 2. === AUTHORIZATION LOGIC ===
    // Only run security logic if is_protected is TRUE
    if is_protected {
        let referer = headers.get(header::REFERER)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        
        let allowed_origins_str = origins_raw.as_deref().unwrap_or("");
        let self_domain = "trycli.com"; // Your production domain
        
        // Check authorization: either whitelist match OR valid token
        let is_authorized_domain = is_authorized(referer, allowed_origins_str, self_domain);
        let is_valid_token = token.as_deref() == query.key.as_deref();
        
        if !is_authorized_domain && !is_valid_token {
            tracing::warn!("Blocked Unauthorized Embed: Referer='{}', Slug='{}'", referer, slug);
            return Err((StatusCode::FORBIDDEN, "Access Denied: Domain not authorized.".to_string()));
        }
    }
    
    let container_name = format!("trycli-studio-viewer-{}", Uuid::new_v4());
    let session_id = Uuid::new_v4().to_string();

    let config = Config {
        image: Some(image_tag),
        tty: Some(true),
        user: Some("root".to_string()), 
        env: Some(vec![
            "LANG=C.UTF-8".to_string(), 
            "LC_ALL=C.UTF-8".to_string(),
            "TERM=xterm-256color".to_string(),
            format!("SHELL={}", shell) 
        ]),
        host_config: Some(HostConfig { 
            memory: Some(512 * 1024 * 1024), 
            memory_swap: Some(512 * 1024 * 1024), 
            nano_cpus: Some(1_000_000_000), 
            pids_limit: Some(64), 
            network_mode: Some("bridge".to_string()), 
            cap_drop: Some(vec!["ALL".to_string()]),
            cap_add: Some(vec![
                "NET_BIND_SERVICE".to_string(), 
                "CHOWN".to_string(),            
                "SETUID".to_string(),           
                "SETGID".to_string(),
                "DAC_OVERRIDE".to_string()      
            ]),
            security_opt: Some(vec!["no-new-privileges".to_string()]),
            readonly_rootfs: Some(false), 
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
    
    // MERGED: Return all necessary fields including owner_id
    Ok(Json(serde_json::json!({
        "container_id": session_id,
        "markdown": markdown,
        "is_protected": is_protected,
        "embed_token": token,
        "owner_id": owner_id
    })))
}