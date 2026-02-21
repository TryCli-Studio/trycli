use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::Html,
    routing::{delete, get, post},
    Json, Router,
};
use bollard::container::Config;
use bollard::image::{CommitContainerOptions, RemoveImageOptions, PushImageOptions};
use tower_sessions::Session;
use uuid::Uuid;
use serde::Deserialize;
use crate::state::{AppState, SessionContext};
use crate::models::{User, ProjectSummary, PublishRequest, WhitelistRequest, TogglePublicRequest};
use std::collections::HashMap;
use url::Url;
use bollard::auth::DockerCredentials;
use futures::StreamExt;

// Maximum number of whitelist entries allowed per project
const MAX_WHITELIST_ENTRIES: i64 = 100;

// Rate limit for whitelist operations (requests per minute per user)
pub const WHITELIST_RATE_LIMIT_PER_MINUTE: u32 = 20;

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
    let port = url.port();
    let path = url.path();
    
    // Normalize trailing slashes for consistency
    let normalized_path = if path == "/" || path.is_empty() {
        "/"
    } else {
        path.trim_end_matches('/')
    };
    
    if let Some(p) = port {
        Some(format!("{}://{}:{}{}", scheme, host, p, normalized_path))
    } else {
        Some(format!("{}://{}{}", scheme, host, normalized_path))
    }
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
        .route("/api/project/:username/:slug/embed-key", get(get_embed_key))
        .route(
            "/api/project/:slug/whitelist",
            get(get_whitelist).post(add_to_whitelist).delete(remove_from_whitelist),
        )
        .route("/api/project/:slug", delete(delete_project))
        .route("/api/search-projects", get(search_projects))
        .route("/api/publish", post(publish_handler))
        .route("/e/:token", get(resolve_secret_embed)) // Secret Embed Route
        .route("/api/project/:slug/visibility", post(toggle_public_visibility))
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
        "SELECT p.slug, p.image_tag, p.view_count, u.username as owner_username, p.embed_key, p.is_public 
         FROM projects p
         JOIN users u ON p.owner_id = u.id
         WHERE p.owner_id = $1 
         ORDER BY p.slug ASC"
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
        "SELECT p.slug, p.image_tag, p.view_count, u.username as owner_username, p.embed_key, p.is_public
         FROM projects p
         JOIN users u ON p.owner_id = u.id
         WHERE p.owner_id = $1 AND p.slug ILIKE $2 
         ORDER BY p.slug ASC LIMIT 10"
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
            ctx.is_publishing = true;
            (ctx.container_name.clone(), ctx.shell.clone())
        } else {
            return Err((StatusCode::BAD_REQUEST, "Session expired".to_string()));
        }
    };

    let safe_slug = payload.slug.trim().to_lowercase();
    let use_remote = std::env::var("USE_REMOTE_REGISTRY").unwrap_or_default() == "true";
    
    // --- CHANGED: Use a single repo, and make the slug the TAG ---
    let (repo_name, tag_name, new_image_tag) = if use_remote {
        let registry_url = std::env::var("REGISTRY_URL")
            .unwrap_or_else(|_| "registry.digitalocean.com/trycli-registry".to_string());
        // Single repository named 'projects'
        let r_name = format!("{}/projects", registry_url);
        (r_name.clone(), safe_slug.clone(), format!("{}:{}", r_name, safe_slug))
    } else {
        // Local fallback
        let r_name = "trycli-studio-projects".to_string();
        (r_name.clone(), safe_slug.clone(), format!("{}:{}", r_name, safe_slug))
    };

    let commit_opts = CommitContainerOptions {
        container: container_name.clone(),
        repo: repo_name.clone(),
        tag: tag_name.clone(), // Use the slug as the tag
        pause: true,
        ..Default::default()
    };

    let config = Config::<String>::default();

    state.docker.commit_container(commit_opts, config)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Snapshot failed: {}", e)))?;

    // 2. Conditionally Push to Remote Registry
    if use_remote {
        let credentials = DockerCredentials {
            username: Some(std::env::var("REGISTRY_USERNAME").unwrap_or_default()),
            password: Some(std::env::var("REGISTRY_PASSWORD").unwrap_or_default()),
            serveraddress: Some(std::env::var("REGISTRY_URL").unwrap_or_default()),
            ..Default::default()
        };

        let mut push_stream = state.docker.push_image(
            &new_image_tag,
            None::<PushImageOptions<String>>,
            Some(credentials)
        );

        while let Some(push_result) = push_stream.next().await {
            if let Err(e) = push_result {
                return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to push image to registry: {}", e)));
            }
        }
    }
    if use_remote {
    println!("Push successful. Cleaning up local cache for: {}", new_image_tag);
    
    let remove_opts = RemoveImageOptions {
        force: true,
        noprune: false,
    };
    
    // This removes the copy from the droplet's disk.
    // The image remains safe in the DigitalOcean Container Registry.
    let _ = state.docker.remove_image(&new_image_tag, Some(remove_opts), None).await;
    }   

    // 3. Update Database
    sqlx::query("
            INSERT INTO projects (slug, image_tag, markdown, owner_id, shell, embed_token, embed_key) 
            VALUES ($1, $2, $3, $4, $5, gen_random_uuid()::text, encode(gen_random_bytes(24), 'base64')) 
            ON CONFLICT (owner_id, slug) 
            DO UPDATE SET image_tag = $2, markdown = $3, shell = $5
        ")
        .bind(&safe_slug)        // $1
        .bind(&new_image_tag)    // $2
        .bind(&payload.markdown) // $3
        .bind(user.id)           // $4
        .bind(&shell_path)       // $5 (CRITICAL: This must be shell path, not username)
        .execute(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB Error: {}", e)))?;

    // 4. Cleanup Memory and Docker Container
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
    session: Session, 
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> axum::response::Result<impl axum::response::IntoResponse> { 
    
    // 1. Load project + security metadata (case-insensitive for username/slug)
    let row_result = sqlx::query_as::<_, (String, String, String, i64, Option<String>, i64, bool)>(
        "SELECT p.image_tag, p.markdown, p.shell, p.owner_id, p.embed_key, p.id, p.is_public 
         FROM projects p
         JOIN users u ON p.owner_id = u.id
         WHERE LOWER(u.username) = LOWER($1) AND LOWER(p.slug) = LOWER($2)"
    )
    .bind(&username).bind(&slug)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| axum::response::ErrorResponse::from((StatusCode::INTERNAL_SERVER_ERROR, format!("DB Read Error: {}", e))))?; 

    // Explicitly destructure so types are known
    let (image_tag, markdown, shell, owner_id, mut embed_key, project_id, is_public) = match row_result {
        Some(r) => (r.0, r.1, r.2, r.3, r.4, r.5, r.6),
        None => return Err(axum::response::ErrorResponse::from((StatusCode::NOT_FOUND, "Project not found".to_string()))),
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
            axum::response::ErrorResponse::from((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to generate VIP key: {}", e),
            ))
        })?;

        if let Some((k,)) = new_key_row {
            embed_key = Some(k);
        }
    }

    // 4. Dual-layer security for embeds (VIP key + Guest List)
    if !is_owner {
        if !is_public {
        // VIP Pass: compare URL ?key with stored embed_key
        let vip_allowed = match (params.get("key"), embed_key.as_ref()) {
            (Some(request_key), Some(stored_key)) => {
                let trimmed_request: &str = request_key.trim();
                let normalized_request = trimmed_request.replace(' ', "+");
                let trimmed_stored: &str = stored_key.trim();

                let matches = trimmed_request == trimmed_stored
                    || normalized_request == trimmed_stored;

                tracing::debug!(
                    "VIP key comparison for project {}: request='{}' normalized='{}' stored='{}' match={}",
                    project_id,
                    trimmed_request,
                    normalized_request,
                    trimmed_stored,
                    matches
                );

                matches
            }
            _ => {
                tracing::debug!("VIP key check failed for project {}: request_key={:?} stored_key={:?}", 
                    project_id, params.get("key"), embed_key.as_ref());
                false
            }
        };

        if !vip_allowed {
            // Guest List: validate parent page URL against project_whitelists.
            let referer = headers
                .get("x-embed-referer")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string())
                .or_else(|| {
                    headers
                        .get("referer")
                        .and_then(|v| v.to_str().ok())
                        .map(|s| s.to_string())
                });

            // Capture the exact blocked url for the error message
            let blocked_origin = referer.clone().unwrap_or_else(|| "Unknown Domain".to_string());

            let whitelist_allowed: Option<bool> = if let Some(referer_url) = referer {
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
                        axum::response::ErrorResponse::from((
                            StatusCode::INTERNAL_SERVER_ERROR,
                            format!("Whitelist DB Error: {}", e),
                        ))
                    })?;

                    Some(exists_row.is_some())
                } else {
                    tracing::warn!("Referer normalization failed for project {}: {}", project_id, referer_url);
                    Some(false)
                }
            } else {
                None
            };

            let is_allowed = whitelist_allowed.unwrap_or(false);

            if !is_allowed {
                // CHANGE: Return JSON with the specific origin instead of just a string
                return Err(axum::response::ErrorResponse::from((
                    StatusCode::FORBIDDEN, 
                    Json(serde_json::json!({
                        "error": "Unauthorized Embed Location",
                        "origin": blocked_origin
                    }))
                )));
            }}
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
            return Err(axum::response::ErrorResponse::from((StatusCode::TOO_MANY_REQUESTS, "Publisher limit reached".to_string())));
        }
    }

    // 7. Construct JSON response
    let session_id = Uuid::new_v4().to_string();

    {
        let mut map = state.lock_sessions();
        map.insert(session_id.clone(), SessionContext {
            container_name: String::new(), 
            pending_image_tag: Some(image_tag),
            shell,
            owner_id: None,
            project_owner_id: Some(owner_id),
            is_publishing: false,
            project_slug: Some(slug), 
            created_at: std::time::Instant::now(),
            is_ws_connected: false,
        }); 
    }
    
    let mut response_json = serde_json::json!({
        "markdown": markdown,
        "owner_id": owner_id,
        "container_id": session_id, // Frontend connects to this UUID
        "is_public": is_public
    });

    if is_owner {
        if let Some(token) = sqlx::query_scalar::<_, String>("SELECT embed_token FROM projects WHERE id = $1").bind(project_id).fetch_optional(&state.db).await.unwrap_or(None) {
            response_json["embed_token"] = serde_json::Value::String(token);
        }
    }

    Ok(Json(response_json))
}

pub async fn toggle_public_visibility(
    State(state): State<AppState>,
    session: Session,
    Path(slug): Path<String>,
    Json(payload): Json<TogglePublicRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let user: Option<User> = session.get("user")
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Session Error: {}", e)))?;
    let user = user.ok_or((StatusCode::UNAUTHORIZED, "Unauthorized".to_string()))?;

    let result = sqlx::query(
        "UPDATE projects SET is_public = $1 WHERE owner_id = $2 AND LOWER(slug) = LOWER($3)"
    )
    .bind(payload.is_public)
    .bind(user.id)
    .bind(&slug)
    .execute(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB Error: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err((StatusCode::NOT_FOUND, "Project not found".to_string()));
    }

    Ok(StatusCode::OK)
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

    // Apply rate limiting: 20 requests per minute per user
    let rate_limiter = state.get_or_create_whitelist_rate_limiter(user.id);
    if rate_limiter.check().is_err() {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            "Rate limit exceeded. Please try again later.".to_string()
        ));
    }

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

    // Apply rate limiting: 20 requests per minute per user
    let rate_limiter = state.get_or_create_whitelist_rate_limiter(user.id);
    if rate_limiter.check().is_err() {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            "Rate limit exceeded. Please try again later.".to_string()
        ));
    }

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

    // Use a database transaction with table-level advisory lock to prevent race conditions
    let mut tx = state.db.begin()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Transaction Error: {}", e)))?;

    // Get an advisory lock for this project's whitelist to prevent concurrent modifications
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(project_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Lock Error: {}", e)))?;

    // Check if entry already exists (using normalized URL)
    let exists: (bool,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM project_whitelists WHERE project_id = $1 AND allowed_url = $2)",
    )
    .bind(project_id)
    .bind(&normalized_url)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB Error: {}", e)))?;

    if exists.0 {
        // Entry already exists, commit transaction and return success
        tx.commit().await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Commit Error: {}", e)))?;
        return Ok(StatusCode::OK); // 200 OK - idempotent operation, entry already exists
    }

    // Check current count
    let count_row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM project_whitelists WHERE project_id = $1",
    )
    .bind(project_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB Error: {}", e)))?;

    if count_row.0 >= MAX_WHITELIST_ENTRIES {
        // Transaction will automatically rollback when dropped
        return Err((
            StatusCode::FORBIDDEN,
            format!("Maximum whitelist entries ({}) reached for this project", MAX_WHITELIST_ENTRIES)
        ));
    }

    // Insert the new entry (normalized)
    sqlx::query(
        "INSERT INTO project_whitelists (project_id, allowed_url) VALUES ($1, $2)",
    )
    .bind(project_id)
    .bind(normalized_url)
    .execute(&mut *tx)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Insert Error: {}", e)))?;

    // Commit transaction
    tx.commit()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Commit Error: {}", e)))?;

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

    // Apply rate limiting: 20 requests per minute per user
    let rate_limiter = state.get_or_create_whitelist_rate_limiter(user.id);
    if rate_limiter.check().is_err() {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            "Rate limit exceeded. Please try again later.".to_string()
        ));
    }

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

/// Get the embed_key for a project (owner-only endpoint).
/// 
/// This endpoint requires authentication and only returns the embed_key to the project owner.
/// Separating this from the main project response prevents accidental exposure via browser
/// dev tools or network inspection when viewing the project normally.
pub async fn get_embed_key(
    State(state): State<AppState>,
    session: Session,
    Path((username, slug)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    // 1. Verify user is authenticated
    let user: Option<User> = session
        .get("user")
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Session Error: {}", e)))?;
    let user = user.ok_or((StatusCode::UNAUTHORIZED, "Unauthorized".to_string()))?;

    // 2. Fetch project and verify ownership (case-insensitive for username/slug)
    let row_result = sqlx::query_as::<_, (i64, i64, Option<String>)>(
        "SELECT p.id, p.owner_id, p.embed_key 
         FROM projects p
         JOIN users u ON p.owner_id = u.id
         WHERE LOWER(u.username) = LOWER($1) AND LOWER(p.slug) = LOWER($2)"
    )
    .bind(&username)
    .bind(&slug)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB Read Error: {}", e)))?;

    let (project_id, owner_id, mut embed_key) = match row_result {
        Some(r) => r,
        None => return Err((StatusCode::NOT_FOUND, "Project not found".to_string())),
    };

    // 3. Verify the user is the owner (return NOT_FOUND to avoid leaking project existence)
    if user.id != owner_id {
        return Err((StatusCode::NOT_FOUND, "Project not found".to_string()));
    }

    // 4. Generate embed_key if it doesn't exist (lazy generation)
    if embed_key.is_none() {
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
                format!("Failed to generate embed key: {}", e),
            )
        })?;

        embed_key = new_key_row.map(|(k,)| k);
    }

    // 5. Return the embed_key (should always be present after step 4)
    let key = embed_key.ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "Failed to retrieve or generate embed key".to_string(),
    ))?;
    
    let response = serde_json::json!({
        "embed_key": key
    });

    Ok(Json(response))
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

    // 1. Delete Local Image Cache (if it exists on this specific node)
    let remove_opts = RemoveImageOptions {
        force: true,
        noprune: false,
    };
    let _ = state.docker.remove_image(&image_tag, Some(remove_opts), None).await;

    // 2. Conditionally Delete from Remote Registry (DigitalOcean API)
    let use_remote = std::env::var("USE_REMOTE_REGISTRY").unwrap_or_default() == "true";
    
    if use_remote {
        let registry_name = std::env::var("REGISTRY_NAME").unwrap_or_else(|_| "trycli-registry".to_string());
        // Since you used the DO API token as the password, we can reuse it here
        let do_token = std::env::var("REGISTRY_PASSWORD").unwrap_or_default(); 
        
        // DigitalOcean's specific endpoint for deleting a tag
        let url = format!(
            "https://api.digitalocean.com/v2/registry/{}/repositories/projects/tags/{}",
            registry_name, slug
        );

        let client = reqwest::Client::new();
        match client.delete(&url).bearer_auth(&do_token).send().await {
            Ok(resp) => {
                if resp.status().is_success() || resp.status() == 404 {
                    println!("Successfully deleted remote image tag: {}", slug);
                } else {
                    let err_text = resp.text().await.unwrap_or_default();
                    eprintln!("Warning: Failed to delete remote image tag. DO API returned: {}", err_text);
                }
            }
            Err(e) => {
                eprintln!("Warning: Network error calling DO Registry API: {}", e);
            }
        }
    }

    Ok(StatusCode::OK)
}

pub async fn resolve_secret_embed(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<Html<String>, (StatusCode, String)> {
    // 1. Validate Token from DB and fetch embed_key for VIP access
    let row: Option<(String, String, Option<String>)> = sqlx::query_as(
        "SELECT u.username, p.slug, p.embed_key 
         FROM projects p
         JOIN users u ON p.owner_id = u.id
         WHERE p.embed_token = $1"
    )
    .bind(&token)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let (owner, slug, embed_key) = row.ok_or((StatusCode::NOT_FOUND, "Invalid embed token".to_string()))?;

    // 2. Get Configurations
    let api_url = std::env::var("API_URL").unwrap_or("http://localhost:3000".to_string());
    // FIX: Get the Frontend URL so we redirect to the right port (8080 in dev)
    let frontend_url = std::env::var("FRONTEND_URL").unwrap_or("http://localhost:8080".to_string());
    
    // 3. Generate Discovery URL for OEmbed (Points to Backend)
    let oembed_url = format!("{}/api/oembed?url={}/e/{}", api_url, api_url, token);

    // 4. Construct Target URL with VIP key for bypass (Points to Frontend)
    let target_url = if let Some(key) = embed_key {
        // URL encode the key to handle special characters in base64
        let encoded_key = urlencoding::encode(&key);
        format!("{}/embed/{}/{}?key={}", frontend_url, owner, slug, encoded_key)
    } else {
        format!("{}/embed/{}/{}", frontend_url, owner, slug)
    };

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

