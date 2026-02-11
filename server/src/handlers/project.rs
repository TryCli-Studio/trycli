use axum::{
    body::Bytes,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::Html,
    routing::{delete, get, post},
    Json, Router,
};
use bollard::container::{CreateContainerOptions, Config};
use bollard::models::{HostConfig, Mount, MountTypeEnum, MountTmpfsOptions};
use bollard::exec::{CreateExecOptions, StartExecResults};
use bollard::image::{CreateImageOptions, RemoveImageOptions}; 
use tower_sessions::Session;
use uuid::Uuid;
use serde::Deserialize;
use futures::StreamExt;
use crate::state::{AppState, SessionContext};
use crate::models::{User, ProjectSummary, PublishRequest, WhitelistRequest};
use std::collections::HashMap;
use url::Url;

#[derive(Deserialize)]
pub struct SearchQuery {
    q: String,
}

/// Normalizes a URL by extracting scheme + host + path (without query params or fragments).
/// 
/// This prevents bypass attempts using query parameters or fragments. For example:
/// - `https://example.com/path?bypass=1` -> `https://example.com/path`
/// - `https://example.com/path#fragment` -> `https://example.com/path`
/// - `https://example.com/path/` -> `https://example.com/path`
/// 
/// # Security
/// Only http and https URLs with valid hosts are accepted. URLs without hosts or
/// with other schemes are rejected to prevent security issues.
/// 
/// # Returns
/// - `Some(normalized_url)` if the URL is valid and has http/https scheme with a host
/// - `None` if the URL cannot be parsed, lacks a host, or uses a non-http(s) scheme
fn normalize_url(url_str: &str) -> Option<String> {
    let url = Url::parse(url_str).ok()?;
    
    // Only accept http and https schemes for security
    let scheme = url.scheme();
    if scheme != "http" && scheme != "https" {
        return None;
    }
    
    // Require a valid host for http(s) URLs
    let host = url.host_str()?;
    let path = url.path();
    
    // Normalize trailing slashes for consistency
    let normalized_path = if path == "/" || path.is_empty() {
        "/"
    } else {
        path.trim_end_matches('/')
    };
    
    Some(format!("{}://{}{}", scheme, host, normalized_path))
}

/// Validates CSRF protection by checking Origin/Referer headers against expected frontend URL.
/// 
/// This function provides defense-in-depth against CSRF attacks by:
/// 1. Checking the Origin header (sent by browsers on cross-origin requests)
/// 2. Falling back to Referer header validation if Origin is not present
/// 3. Requiring the request to come from the configured FRONTEND_URL
/// 
/// # Security
/// This works in conjunction with SameSite=Strict cookies to prevent CSRF attacks.
/// An attacker on a different domain cannot forge these headers in a way that would
/// pass validation.
/// 
/// # Returns
/// - `Ok(())` if the request passes CSRF validation
/// - `Err((StatusCode, String))` if the request fails validation
fn validate_csrf_protection(headers: &HeaderMap) -> Result<(), (StatusCode, String)> {
    let frontend_url = std::env::var("FRONTEND_URL")
        .unwrap_or_else(|_| "http://localhost:8080".to_string());
    
    // Parse the expected origin from FRONTEND_URL
    let expected_origin = Url::parse(&frontend_url)
        .ok()
        .and_then(|u| {
            let scheme = u.scheme();
            let host = u.host_str()?;
            let port = u.port();
            if let Some(p) = port {
                Some(format!("{}://{}:{}", scheme, host, p))
            } else {
                Some(format!("{}://{}", scheme, host))
            }
        })
        .ok_or_else(|| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Invalid FRONTEND_URL configuration".to_string(),
            )
        })?;
    
    // Check Origin header first (preferred for CORS requests)
    if let Some(origin) = headers.get("origin") {
        let origin_str = origin
            .to_str()
            .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid Origin header".to_string()))?;
        
        if origin_str == expected_origin {
            return Ok(());
        } else {
            return Err((
                StatusCode::FORBIDDEN,
                "CSRF validation failed: Origin mismatch".to_string(),
            ));
        }
    }
    
    // Fallback to Referer header
    if let Some(referer) = headers.get("referer") {
        let referer_str = referer
            .to_str()
            .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid Referer header".to_string()))?;
        
        // Check if referer starts with the expected origin
        if referer_str.starts_with(&expected_origin) {
            return Ok(());
        } else {
            return Err((
                StatusCode::FORBIDDEN,
                "CSRF validation failed: Referer mismatch".to_string(),
            ));
        }
    }
    
    // No Origin or Referer header found
    Err((
        StatusCode::FORBIDDEN,
        "CSRF validation failed: Missing Origin or Referer header".to_string(),
    ))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/my-projects", get(list_user_projects))
        .route("/api/project/:username/:slug", get(get_project))
        .route("/api/project/:slug", delete(delete_project))
        .route(
            "/api/project/:slug/whitelist",
            get(get_whitelist).post(add_to_whitelist).delete(remove_from_whitelist),
        )
        .route("/api/search-projects", get(search_projects))
        .route("/api/publish", post(publish_handler))
        .route("/e/:token", get(resolve_secret_embed)) // Secret Embed Route
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
        "SELECT slug, image_tag, view_count, owner_username, embed_key \
         FROM projects WHERE owner_id = $1 ORDER BY slug ASC"
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
        "SELECT slug, image_tag, view_count, owner_username, embed_key \
         FROM projects WHERE owner_id = $1 AND slug ILIKE $2 ORDER BY slug ASC LIMIT 10"
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

    // 6. Update Database with new embed_token logic
    // gen_random_uuid() requires Postgres 13+. If older, ensure pgcrypto extension is enabled.
    // Generate embed_key at creation time to ensure VIP links work immediately
    sqlx::query("
            INSERT INTO projects (slug, image_tag, markdown, owner_id, owner_username, shell, embed_token, embed_key) 
            VALUES ($1, $2, $3, $4, $5, $6, gen_random_uuid()::text, encode(gen_random_bytes(24), 'base64')) 
            ON CONFLICT (owner_username, slug) 
            DO UPDATE SET image_tag = $2, markdown = $3, shell = $6
        ")
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
    State(state): State<AppState>,
    session: Session, // Session is used to detect owner (bypass checks), lazily create embed_key for owners, and return embed_token/embed_key to them; non-owner secure embeds use VIP key + Referer whitelist
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    
    // 1. Load project + security metadata (case-insensitive for username/slug)
    let row_result = sqlx::query_as::<_, (i64, String, String, String, i64, Option<String>)>(
        "SELECT id, image_tag, markdown, shell, owner_id, embed_key \
         FROM projects \
         WHERE LOWER(owner_username) = LOWER($1) AND LOWER(slug) = LOWER($2)"
    )
    .bind(&username).bind(&slug)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB Read Error: {}", e)))?;

    let (project_id, image_tag, markdown, shell, owner_id, mut embed_key) = match row_result {
        Some(r) => r,
        None => return Err((StatusCode::NOT_FOUND, "Project not found".to_string())),
    };

    // 2. Determine current user & ownership (owners bypass embed security)
    let current_user: Option<User> = session.get("user").await.ok().flatten();
    let is_owner = current_user.as_ref().map(|u| u.id) == Some(owner_id);

    // 3. Ensure owners always have a VIP key (embed_key); generate one lazily if missing
    if is_owner && embed_key.is_none() {
        let new_key_row: Option<(String,)> = sqlx::query_as(
            "UPDATE projects \
             SET embed_key = encode(gen_random_bytes(24), 'base64') \
             WHERE id = $1 \
             RETURNING embed_key",
        )
        .bind(project_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to generate VIP key: {}", e),
            )
        })?;

        if let Some((k,)) = new_key_row {
            embed_key = Some(k);
        }
    }

    // 4. Dual-layer security for embeds (VIP key + Guest List)
    if !is_owner {
        // VIP Pass: compare URL ?key with stored embed_key using a match on Options
        let vip_allowed = match (params.get("key"), embed_key.as_ref()) {
            (Some(request_key), Some(stored_key)) if request_key == stored_key => true,
            _ => false,
        };

        if !vip_allowed {
            // Guest List: validate Referer header against project_whitelists
            let referer = headers
                .get("Referer")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());

            // Optional boolean, safely unwrapped later
            let whitelist_allowed: Option<bool> = if let Some(referer_url) = referer {
                // Normalize the Referer URL to prevent bypasses via query params or fragments
                let normalized_referer = normalize_url(&referer_url);
                
                if let Some(normalized) = normalized_referer {
                    let exists_row: Option<(bool,)> = sqlx::query_as(
                        "SELECT TRUE FROM project_whitelists \
                         WHERE project_id = $1 AND allowed_url = $2 \
                         LIMIT 1",
                    )
                    .bind(project_id)
                    .bind(&normalized)
                    .fetch_optional(&state.db)
                    .await
                    .map_err(|e| {
                        (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            format!("Whitelist DB Error: {}", e),
                        )
                    })?;

                    // TRUE row exists => allowed; otherwise false
                    Some(exists_row.is_some())
                } else {
                    // If URL parsing fails, deny access and log for security monitoring
                    tracing::warn!("Referer normalization failed for project {}: {}", project_id, referer_url);
                    Some(false)
                }
            } else {
                None
            };

            let is_allowed = whitelist_allowed.unwrap_or(false);

            if !is_allowed {
                return Err((
                    StatusCode::FORBIDDEN,
                    "This terminal is restricted to authorized websites.".to_string(),
                ));
            }
        }
    }

    // 5. Increment View Count asynchronously (only after passing security)
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

    // 6. Publisher concurrency limit (protect compute)
    {
        let sessions = state.lock_sessions();
        let active_viewers = sessions.values()
            .filter(|ctx| ctx.project_owner_id == Some(owner_id))
            .count();
        
        if active_viewers >= 5 {
            return Err((StatusCode::TOO_MANY_REQUESTS, "Publisher limit reached".to_string()));
        }
    }

    // 7. Construct JSON response
    let mut response_json = serde_json::json!({
        "markdown": markdown,
        "owner_id": owner_id,
        // We will insert container_id below
    });

    // If owner, fetch and attach the secret token + VIP embed_key for the Share / Embed modal
    if is_owner {
        let token_record: Option<(String,)> = sqlx::query_as(
            "SELECT embed_token FROM projects WHERE owner_id = $1 AND LOWER(slug) = LOWER($2)"
        )
        .bind(owner_id)
        .bind(&slug)
        .fetch_optional(&state.db)
        .await
        .ok()
        .flatten();

        if let Some((token,)) = token_record {
            response_json["embed_token"] = serde_json::Value::String(token);
        }

        if let Some(key) = embed_key {
            response_json["embed_key"] = serde_json::Value::String(key);
        }
    }

    // Spin up container
    let container_name = format!("trycli-studio-viewer-{}", Uuid::new_v4());
    let session_id = Uuid::new_v4().to_string();

    let config = Config {
        image: Some(image_tag),
        labels: Some(HashMap::from([
        ("managed_by".to_string(), "TryCli Studio".to_string())
        ])),
        tty: Some(true),
        user: Some("root".to_string()), 
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
            is_ws_connected: false,
        }); 
    }
    
    // Add session ID to the response
    response_json["container_id"] = serde_json::Value::String(session_id);

    Ok(Json(response_json))
}

/// Get the current whitelist (Guest List) for a project owned by the authenticated user.
pub async fn get_whitelist(
    State(state): State<AppState>,
    session: Session,
    Path(slug): Path<String>,
) -> Result<Json<Vec<String>>, (StatusCode, String)> {
    let user: Option<User> = session
        .get("user")
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Session Error: {}", e)))?;
    let user = user.ok_or((StatusCode::UNAUTHORIZED, "Unauthorized".to_string()))?;

    // Resolve project_id for this owner + slug
    let project_row: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM projects WHERE owner_id = $1 AND LOWER(slug) = LOWER($2)",
    )
    .bind(user.id)
    .bind(&slug)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB Error: {}", e)))?;

    let (project_id,) = match project_row {
        Some(row) => row,
        None => return Err((StatusCode::NOT_FOUND, "Project not found".to_string())),
    };

    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT allowed_url FROM project_whitelists WHERE project_id = $1 ORDER BY created_at DESC",
    )
    .bind(project_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB Error: {}", e)))?;

    let urls = rows.into_iter().map(|(url,)| url).collect();

    Ok(Json(urls))
}

/// Add a new URL to a project's whitelist (Guest List).
pub async fn add_to_whitelist(
    State(state): State<AppState>,
    headers: HeaderMap,
    session: Session,
    Path(slug): Path<String>,
    Json(payload): Json<WhitelistRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    // CSRF Protection: Validate Origin/Referer headers
    validate_csrf_protection(&headers)?;
    
    let user: Option<User> = session
        .get("user")
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Session Error: {}", e)))?;
    let user = user.ok_or((StatusCode::UNAUTHORIZED, "Unauthorized".to_string()))?;

    let trimmed_url = payload.allowed_url.trim();
    if trimmed_url.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "allowed_url is required".to_string()));
    }

    let project_row: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM projects WHERE owner_id = $1 AND LOWER(slug) = LOWER($2)",
    )
    .bind(user.id)
    .bind(&slug)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB Error: {}", e)))?;

    let (project_id,) = match project_row {
        Some(row) => row,
        None => return Err((StatusCode::NOT_FOUND, "Project not found".to_string())),
    };

    // Normalize the URL to prevent bypasses via query params or fragments
    let normalized_url = normalize_url(trimmed_url).ok_or((
        StatusCode::BAD_REQUEST,
        "Invalid URL format. URL must use http or https scheme and include a valid host.".to_string(),
    ))?;

    // Unique(project_id, allowed_url) is enforced by the DB; ignore conflicts
    let result = sqlx::query(
        "INSERT INTO project_whitelists (project_id, allowed_url) \
         VALUES ($1, $2) \
         ON CONFLICT (project_id, allowed_url) DO NOTHING",
    )
    .bind(project_id)
    .bind(&normalized_url)
    .execute(&state.db)
    .await;

    if let Err(e) = result {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("DB Error: {}", e)));
    }

    Ok(StatusCode::CREATED)
}

/// Remove a URL from a project's whitelist (Guest List).
pub async fn remove_from_whitelist(
    State(state): State<AppState>,
    headers: HeaderMap,
    session: Session,
    Path(slug): Path<String>,
    Json(payload): Json<WhitelistRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    // CSRF Protection: Validate Origin/Referer headers
    validate_csrf_protection(&headers)?;
    
    let user: Option<User> = session
        .get("user")
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Session Error: {}", e)))?;
    let user = user.ok_or((StatusCode::UNAUTHORIZED, "Unauthorized".to_string()))?;

    let trimmed_url = payload.allowed_url.trim();
    if trimmed_url.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "allowed_url is required".to_string()));
    }

    // Normalize the URL to match how it was stored
    let normalized_url = normalize_url(trimmed_url).ok_or((
        StatusCode::BAD_REQUEST,
        "Invalid URL format. URL must use http or https scheme and include a valid host.".to_string(),
    ))?;

    let delete_result = sqlx::query(
        "DELETE FROM project_whitelists pw \
         USING projects p \
         WHERE pw.project_id = p.id \
           AND p.owner_id = $1 \
           AND LOWER(p.slug) = LOWER($2) \
           AND pw.allowed_url = $3",
    )
    .bind(user.id)
    .bind(&slug)
    .bind(&normalized_url)
    .execute(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB Error: {}", e)))?;

    if delete_result.rows_affected() == 0 {
        return Err((StatusCode::NOT_FOUND, "Whitelist entry not found".to_string()));
    }

    Ok(StatusCode::NO_CONTENT)
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

pub async fn resolve_secret_embed(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<Html<String>, (StatusCode, String)> {
    // 1. Validate Token from DB
    let row: Option<(String, String)> = sqlx::query_as(
        "SELECT owner_username, slug FROM projects WHERE embed_token = $1"
    )
    .bind(&token)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let (owner, slug) = row.ok_or((StatusCode::NOT_FOUND, "Invalid embed token".to_string()))?;

    // 2. Get Configurations
    let api_url = std::env::var("API_URL").unwrap_or("http://localhost:3000".to_string());
    // FIX: Get the Frontend URL so we redirect to the right port (8080 in dev)
    let frontend_url = std::env::var("FRONTEND_URL").unwrap_or("http://localhost:8080".to_string());
    
    // 3. Generate Discovery URL for OEmbed (Points to Backend)
    let oembed_url = format!("{}/api/oembed?url={}/e/{}", api_url, api_url, token);

    // 4. Construct Target URL (Points to Frontend)
    let target_url = format!("{}/embed/{}/{}", frontend_url, owner, slug);

    // 5. Return HTML with <link> tags + Meta Refresh
    let html = format!(r#"<!DOCTYPE html>
<html>
<head>
    <title>TryCLI Interactive Demo</title>
    <link rel="alternate" type="application/json+oembed" href="{}" title="Interactive Terminal" />
    <meta property="og:title" content="Interactive Terminal Demo" />
    <meta property="og:description" content="Click to launch a live Linux container for this project." />
    <meta property="og:type" content="website" />
    <meta http-equiv="refresh" content="0;url={}" />
</head>
<body>
    <p>Redirecting to interactive terminal...</p>
    <script>window.location.href = "{}";</script>
</body>
</html>"#, oembed_url, target_url, target_url);

    Ok(Html(html))
}