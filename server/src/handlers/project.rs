use axum::{
    extract::{Path, Query, State},
    routing::{get, post, delete},
    Router, Json,
    http::StatusCode,
    body::Bytes, // FIX: Import Bytes
};
use bollard::container::{CreateContainerOptions, Config};
use bollard::models::{HostConfig, Mount, MountTypeEnum, MountTmpfsOptions};
use bollard::exec::{CreateExecOptions, StartExecResults};
use bollard::image::{CreateImageOptions, RemoveImageOptions}; 
use tower_sessions::Session;
use uuid::Uuid;
use serde::Deserialize;
use futures::StreamExt; // FIX: Import StreamExt
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
        .route("/api/project/:slug", delete(delete_project))
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
        "SELECT slug, image_tag, view_count, owner_username FROM projects WHERE owner_id = $1 ORDER BY slug ASC"
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
    
    let projects = sqlx::query_as::<_, ProjectSummary>(
        "SELECT slug, image_tag, view_count, owner_username FROM projects WHERE owner_id = $1 AND slug ILIKE $2 ORDER BY slug ASC LIMIT 10"
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
        let mut map = state.lock_sessions();
        if let Some(ctx) = map.get_mut(&payload.container_id) {
            if ctx.owner_id != Some(user.id) {
                 return Err((StatusCode::FORBIDDEN, "You do not own this session".to_string()));
            }
            // Set flag to prevent WebSocket from deleting it
            ctx.is_publishing = true;
            (ctx.container_name.clone(), ctx.shell.clone())
        } else {
            return Err((StatusCode::BAD_REQUEST, "Session expired".to_string()));
        }
    };

    let safe_slug = payload.slug.trim().to_lowercase();
    let new_image_tag = format!("trycli-studio-project-{}:latest", safe_slug);

    // 2. Prepare Internal Tar Command
    let tar_cmd = vec![
        "tar", "-cf", "-", "-C", "/", 
        "bin", "etc", "home", "lib", "media", "mnt", "opt", "root", "sbin", "srv", "usr", "var"
    ];

    let exec_config = CreateExecOptions {
        attach_stdout: Some(true),
        attach_stderr: Some(true), 
        cmd: Some(tar_cmd),
        ..Default::default()
    };

    // 3. Create Exec Instance
    let exec = state.docker.create_exec(&container_name, exec_config)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Exec Create Error: {}", e)))?;

    // 4. Start Exec and Capture Stream
    let mut tar_buffer = Vec::new();
    
    if let Ok(StartExecResults::Attached { mut output, .. }) = state.docker.start_exec(&exec.id, None).await {
        while let Some(msg_result) = output.next().await {
            match msg_result {
                Ok(bollard::container::LogOutput::StdOut { message }) => {
                    tar_buffer.extend_from_slice(&message);
                },
                Ok(bollard::container::LogOutput::StdErr { message }) => {
                    println!("Tar Warning: {}", String::from_utf8_lossy(&message));
                },
                Ok(_) => {}, 
                Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Stream Error: {}", e))),
            }
        }
    } else {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to attach to tar process".to_string()));
    }

    if tar_buffer.is_empty() {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, "Snapshot failed: Tar archive was empty".to_string()));
    }

    // 5. IMPORT the captured tarball
    let create_opts = CreateImageOptions {
        from_src: "-".to_string(), 
        repo: new_image_tag.clone(),
        ..Default::default()
    };

    let mut create_image_stream = state.docker.create_image(
        Some(create_opts),
        Some(Bytes::from(tar_buffer)), 
        None
    );

    while let Some(result) = create_image_stream.next().await {
        if let Err(e) = result {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Import Error: {}", e)));
        }
    }

    // 6. Update Database
    sqlx::query("INSERT INTO projects (slug, image_tag, markdown, owner_id, owner_username, shell) VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT (owner_username, slug) DO UPDATE SET image_tag = $2, markdown = $3, shell = $6")
        .bind(&safe_slug)
        .bind(&new_image_tag)
        .bind(&payload.markdown)
        .bind(user.id)          
        .bind(&user.login)
        .bind(&shell_path) 
        .execute(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB Error: {}", e)))?;

    // 7. Cleanup
    {
        let mut map = state.lock_sessions();
        map.remove(&payload.container_id);
    }
    
    let _ = state.docker.remove_container(&container_name, Some(
        bollard::container::RemoveContainerOptions { force: true, ..Default::default() }
    )).await;

    Ok(Json("Published!".to_string()))
}

pub async fn get_project(
    Path((username, slug)): Path<(String, String)>, 
    State(state): State<AppState>
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    
    // Case-insensitive lookup (Fixes 404s due to capitalization)
    let row_result = sqlx::query_as::<_, (String, String, String, i64)>(
        "SELECT image_tag, markdown, shell, owner_id FROM projects WHERE LOWER(owner_username) = LOWER($1) AND LOWER(slug) = LOWER($2)"
    )
    .bind(&username).bind(&slug)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB Read Error: {}", e)))?;

    let (image_tag, markdown, shell, owner_id) = match row_result {
        Some(r) => r,
        None => return Err((StatusCode::NOT_FOUND, "Project not found".to_string())),
    };

    // 2. [NEW] Increment View Count asynchronously
    // We don't await strictly for this to ensure speed for the user
    let db_clone = state.db.clone();
    let slug_clone = slug.clone();
    let username_clone = username.clone();
    tokio::spawn(async move {
        let _ = sqlx::query("UPDATE projects SET view_count = view_count + 1 WHERE LOWER(owner_username) = LOWER($1) AND LOWER(slug) = LOWER($2)")
            .bind(username_clone)
            .bind(slug_clone)
            .execute(&db_clone)
            .await;
    });

    {
        let sessions = state.lock_sessions();
        let active_viewers = sessions.values()
            .filter(|ctx| ctx.project_owner_id == Some(owner_id))
            .count();
        
        if active_viewers >= 5 {
            return Err((StatusCode::TOO_MANY_REQUESTS, "Publisher limit reached".to_string()));
        }
    }

    let container_name = format!("trycli-studio-viewer-{}", Uuid::new_v4());
    let session_id = Uuid::new_v4().to_string();

    let config = Config {
        image: Some(image_tag),
        tty: Some(true),
        user: Some("root".to_string()), 
        // FIX: Explicit CMD needed for flattened images
        cmd: Some(vec![shell.clone()]), 
        env: Some(vec![
            "LANG=C.UTF-8".to_string(), 
            "LC_ALL=C.UTF-8".to_string(),
            "TERM=xterm-256color".to_string(),
            format!("SHELL={}", shell) 
        ]),
        host_config: Some(HostConfig { 
            runtime: Some("runsc".to_string()),
            memory: Some(512 * 1024 * 1024), 
            nano_cpus: Some(250_000_000),
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
            readonly_rootfs: Some(true),
            mounts: Some(vec![
                Mount {
                    target: Some("/root".to_string()), 
                    typ: Some(MountTypeEnum::TMPFS), 
                    tmpfs_options: Some(MountTmpfsOptions {
                        size_bytes: Some(50 * 1024 * 1024), 
                        mode: Some(0o1777),
                    }),
                    ..Default::default()
                },
                Mount {
                    target: Some("/tmp".to_string()),
                    typ: Some(MountTypeEnum::TMPFS),
                    tmpfs_options: Some(MountTmpfsOptions {
                        size_bytes: Some(50 * 1024 * 1024),
                        mode: Some(0o1777),
                    }),
                    ..Default::default()
                }
            ]),
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
            owner_id: None,
            project_owner_id: Some(owner_id),
            is_publishing: false,
            project_slug: Some(slug), 
            created_at: std::time::Instant::now(),
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
    let user: Option<User> = session.get("user")
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Session Error: {}", e)))?;
        
    let user = user.ok_or((StatusCode::UNAUTHORIZED, "Unauthorized".to_string()))?;

    let record: Option<(String,)> = sqlx::query_as(
        "SELECT image_tag FROM projects WHERE LOWER(slug) = LOWER($1) AND owner_id = $2"
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

    let db_result = sqlx::query(
        "DELETE FROM projects WHERE LOWER(slug) = LOWER($1) AND owner_id = $2"
    )
    .bind(&slug)
    .bind(user.id)
    .execute(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB Delete Error: {}", e)))?;

    if db_result.rows_affected() == 0 {
        return Err((StatusCode::NOT_FOUND, "Project not found".to_string()));
    }

    let remove_opts = RemoveImageOptions {
        force: true,
        noprune: false,
    };

    if let Err(e) = state.docker.remove_image(&image_tag, Some(remove_opts), None).await {
        eprintln!("Warning: Failed to remove docker image {}: {}", image_tag, e);
    } else {
        println!("Cleaned up image: {}", image_tag);
    }

    Ok(StatusCode::OK)
}