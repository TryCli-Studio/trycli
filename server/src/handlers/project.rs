use axum::{
    extract::{Path, Query, State},
    routing::{get, post,delete},
    Router, Json,
    http::StatusCode,
};
use bollard::container::{CreateContainerOptions, Config};
use bollard::models::HostConfig;
use bollard::image::{CommitContainerOptions, RemoveImageOptions};
use tower_sessions::Session;
use uuid::Uuid;
use serde::Deserialize;
use crate::state::{AppState, SessionContext};
use crate::models::{User, ProjectSummary, PublishRequest};

#[derive(Deserialize)]
pub struct SearchQuery {
    q: String,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/my-projects", get(list_user_projects))
        .route("/api/project/:username/:slug", get(get_project))
        .route("/api/project/:slug", delete(delete_project)) // <--- New Route
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
    // FIX: Map error instead of unwrap
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

    // 5. Save to Database
    sqlx::query("INSERT INTO projects (slug, image_tag, markdown, owner_id, owner_username, shell) VALUES ($1, $2, $3, $4, $5, $6)")
        .bind(&payload.slug)
        .bind(&new_image_tag)
        .bind(&payload.markdown)
        .bind(user.id)          
        .bind(&user.login)
        .bind(&shell_path) 
        .execute(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB Error: {}", e)))?;

    let _ = state.docker.stop_container(&container_name, None).await;

    Ok(Json("Published!".to_string()))
}

pub async fn get_project(
    Path((username, slug)): Path<(String, String)>, 
    State(state): State<AppState>
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    
    // FIX: Handle DB errors properly
    let row_result = sqlx::query_as::<_, (String, String, String, i64)>(
        "SELECT image_tag, markdown, shell, owner_id FROM projects WHERE owner_username = $1 AND slug = $2"
    )
    .bind(username).bind(slug)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB Read Error: {}", e)))?;

    let (image_tag, markdown, shell, owner_id) = match row_result {
        Some(r) => r,
        None => return Err((StatusCode::NOT_FOUND, "Project not found".to_string())),
    };

    let container_name = format!("trycli-studio-viewer-{}", Uuid::new_v4());
    let session_id = Uuid::new_v4().to_string();

    let config = Config {
        image: Some(image_tag),
        tty: Some(true),
        // 1. Run as a non-root user if possible (requires image support), 
        // otherwise rely on CapDrop (below).
        user: Some("root".to_string()), 
        env: Some(vec![
            "LANG=C.UTF-8".to_string(), 
            "LC_ALL=C.UTF-8".to_string(),
            "TERM=xterm-256color".to_string(),
            format!("SHELL={}", shell) 
        ]),
        host_config: Some(HostConfig { 
            // 2. RESOURCE LIMITS
            memory: Some(512 * 1024 * 1024), // 512 MB RAM
            memory_swap: Some(512 * 1024 * 1024), // No extra swap
            nano_cpus: Some(1_000_000_000), // 1.0 CPU Core
            pids_limit: Some(64), // Prevent Fork Bombs (max 64 processes)
            
            // 3. NETWORK SECURITY
            // "bridge" is default, but ensures they can't access host networking
            network_mode: Some("bridge".to_string()), 
            
            // 4. KERNEL SECURITY (The most important part)
            // Drop ALL capabilities first
            cap_drop: Some(vec!["ALL".to_string()]),
            // Add back ONLY what is strictly needed for standard tools
            cap_add: Some(vec![
                "NET_BIND_SERVICE".to_string(), // Allow binding ports
                "CHOWN".to_string(),            // File permissions
                "SETUID".to_string(),           // Sudo/su support
                "SETGID".to_string(),
                "DAC_OVERRIDE".to_string()      // File permission overrides
            ]),
            
            // 5. SECURITY OPT
            // Prevents processes from gaining more privileges than they started with
            // (e.g., prevents some buffer overflow exploits)
            security_opt: Some(vec!["no-new-privileges".to_string()]),
            
            // 6. FILESYSTEM
            // For now, we leave it writable for temp files, but we could mount a tmpfs.
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
        // Viewers are public (owner_id: None)
        map.insert(session_id.clone(), SessionContext {
            container_name: container_name.clone(), 
            shell,
            owner_id: None 
        }); 
    }
    
    Ok(Json(serde_json::json!({
        "container_id": session_id,
        "markdown": markdown,
        "owner_id": owner_id
    })))
}

pub async fn delete_project(
    State(state): State<AppState>,
    session: Session,
    Path(slug): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    // 1. Authenticate User
    let user: Option<User> = session.get("user")
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Session Error: {}", e)))?;
        
    let user = user.ok_or((StatusCode::UNAUTHORIZED, "Unauthorized".to_string()))?;

    // 2. Fetch Image Tag & Verify Ownership (Before Deletion)
    // We need the image tag to clean up Docker, and we need to verify ownership strictly.
    let record: Option<(String,)> = sqlx::query_as(
        "SELECT image_tag FROM projects WHERE slug = $1 AND owner_id = $2"
    )
    .bind(&slug)
    .bind(user.id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB Error: {}", e)))?;

    let image_tag = match record {
        Some(r) => r.0,
        None => return Err((StatusCode::NOT_FOUND, "Project not found or access denied".to_string())),
    };

    // 3. Delete from Database (Source of Truth)
    let db_result = sqlx::query(
        "DELETE FROM projects WHERE slug = $1 AND owner_id = $2"
    )
    .bind(&slug)
    .bind(user.id)
    .execute(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB Delete Error: {}", e)))?;

    if db_result.rows_affected() == 0 {
        // This is unlikely given step 2, but handles race conditions
        return Err((StatusCode::NOT_FOUND, "Project not found".to_string()));
    }

    // 4. Purge Docker Image
    // We use force=true to kill any active containers using this image.
    // We map errors but do NOT fail the request if Docker fails (e.g., image already manualy deleted).
    let remove_opts = RemoveImageOptions {
        force: true, // Force removal even if containers are running
        noprune: false,
    };

    if let Err(e) = state.docker.remove_image(&image_tag, Some(remove_opts), None).await {
        // Log it, but don't fail the request to the client, as the DB entry is already gone.
        eprintln!("Warning: Failed to remove docker image {}: {}", image_tag, e);
    } else {
        println!("Cleaned up image: {}", image_tag);
    }

    Ok(StatusCode::OK)
}