use axum::{
    extract::{Path, State},
    routing::{get, post},
    Router, Json,
    http::StatusCode,
};
use bollard::container::{CreateContainerOptions, Config};
use bollard::models::HostConfig;
use bollard::image::CommitContainerOptions;
use tower_sessions::Session;
use uuid::Uuid;
use std::collections::HashMap;
use crate::state::AppState;
use crate::models::{User, ProjectSummary, PublishRequest};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/my-projects", get(list_user_projects))
        .route("/api/project/:username/:slug", get(get_project))
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

    // 1. Get Session Info (Using Safe Lock)
    let (container_id, shell_path) = {
        let map = state.lock_sessions();
        match map.get(&payload.container_id) {
            Some((cid, sh)) => (cid.clone(), sh.clone()),
            None => return Err((StatusCode::BAD_REQUEST, "Session expired".to_string())),
        }
    };

    let new_image_tag = format!("trycli-project-{}", payload.slug);

    // 2. Prepare Commit Options
    let commit_opts = CommitContainerOptions {
        container: container_id.clone(),
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

    let _ = state.docker.stop_container(&container_id, None).await;

    Ok(Json("Published!".to_string()))
}

pub async fn get_project(
    Path((username, slug)): Path<(String, String)>, 
    State(state): State<AppState>
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    
    // FIX: Handle DB errors properly
    let row_result = sqlx::query_as::<_, (String, String, String)>(
        "SELECT image_tag, markdown, shell FROM projects WHERE owner_username = $1 AND slug = $2"
    )
    .bind(username).bind(slug)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB Read Error: {}", e)))?;

    let (image_tag, markdown, shell) = match row_result {
        Some(r) => r,
        None => return Err((StatusCode::NOT_FOUND, "Project not found".to_string())),
    };

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
        // FIX: Safe lock helper
        let mut map = state.lock_sessions();
        map.insert(session_id.clone(), (container_name.clone(), shell)); 
    }

    Ok(Json(serde_json::json!({
        "container_id": session_id,
        "markdown": markdown
    })))
}
