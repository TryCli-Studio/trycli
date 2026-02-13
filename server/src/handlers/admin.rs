use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, delete},
    Router, Json,
};
use tower_sessions::Session;
use bollard::container::{ListContainersOptions, RemoveContainerOptions};
use bollard::image::RemoveImageOptions;
use serde::Serialize;
use crate::state::AppState;
use crate::models::{User, ProjectSummary};

// 🔒 SECURITY: Add GitHub usernames here (lowercase)
// Example: &["karthikey", "alice", "bob"]
const ADMIN_ALLOWLIST: &[&str] = &["karthikeyjoshi", "yashb404", "rakshat28"];

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/admin/stats", get(get_system_stats))
        .route("/api/admin/projects", get(get_all_projects))
        .route("/api/admin/container/{id}", delete(kill_container))
        .route("/api/admin/project/{slug}", delete(delete_project_admin))
}

// Updated middleware to check the list
async fn check_admin(session: &Session) -> Result<(), (StatusCode, String)> {
    let user: Option<User> = session.get("user").await.map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Session Error".into()))?;
    
    match user {
        Some(u) => {
            // Check if the logged-in user is in our allowlist
            if ADMIN_ALLOWLIST.contains(&u.login.to_lowercase().as_str()) {
                Ok(())
            } else {
                Err((StatusCode::FORBIDDEN, "Admin access only".into()))
            }
        },
        None => Err((StatusCode::UNAUTHORIZED, "Please login first".into())),
    }
}

#[derive(Serialize)]
pub struct SystemStats {
    pub total_projects: i64,
    pub total_views: i64,
    pub active_containers: usize,
    pub container_list: Vec<ContainerInfo>,
}

#[derive(Serialize)]
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    pub state: String,
    pub image: String,
    pub is_managed: bool,
}

pub async fn get_system_stats(
    State(state): State<AppState>,
    session: Session,
) -> Result<Json<SystemStats>, (StatusCode, String)> {
    check_admin(&session).await?;

    // 1. DB Stats
    let project_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM projects")
        .fetch_one(&state.db).await.unwrap_or(0);
    
    let view_count: i64 = sqlx::query_scalar("SELECT SUM(view_count) FROM projects")
        .fetch_one(&state.db).await.unwrap_or(0);

    // 2. Docker Stats
    let opts = ListContainersOptions::<String> {
        all: true, 
        ..Default::default()
    };

    let containers = state.docker.list_containers(Some(opts)).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let container_list = containers.into_iter().map(|c| {
        let name = c.names.unwrap_or_default().get(0).cloned().unwrap_or_default();
        let is_managed = name.contains("trycli") || c.labels.as_ref().map_or(false, |l| l.contains_key("managed_by"));

        ContainerInfo {
            id: c.id.unwrap_or_default(),
            name,
            state: c.state.unwrap_or_default(),
            image: c.image.unwrap_or_default(),
            is_managed,
        }
    }).collect();

    // 3. Active Sessions
    let active_sessions_count = state.lock_sessions().len();

    Ok(Json(SystemStats {
        total_projects: project_count,
        total_views: view_count,
        active_containers: active_sessions_count,
        container_list,
    }))
}

pub async fn get_all_projects(
    State(state): State<AppState>,
    session: Session,
) -> Result<Json<Vec<ProjectSummary>>, (StatusCode, String)> {
    check_admin(&session).await?;

    let projects = sqlx::query_as::<_, ProjectSummary>(
        "SELECT slug, image_tag, view_count, owner_username FROM projects ORDER BY view_count DESC LIMIT 100"
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB Error: {}", e)))?;

    Ok(Json(projects))
}

pub async fn kill_container(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<String>, // Note: 'id' here might be the container ID or Name
) -> Result<Json<String>, (StatusCode, String)> {
    check_admin(&session).await?;

    state.docker.remove_container(&id, Some(RemoveContainerOptions {
        force: true,
        ..Default::default()
    })).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // 2. Remove from SessionMap (The Fix)
    {
        let mut map = state.lock_sessions();
        // Remove the entry where the container name matches the ID passed
        // Note: The UI passes the container ID usually, but check if your UI passes Name or ID. 
        // Ideally, check against both or ensure consistency.
        map.retain(|_, ctx| ctx.container_name != id);
    }

    Ok(Json("Container killed and session cleared".to_string()))
}

pub async fn delete_project_admin(
    State(state): State<AppState>,
    session: Session,
    Path(slug): Path<String>,
) -> Result<Json<String>, (StatusCode, String)> {
    check_admin(&session).await?;

    let record: Option<(String,)> = sqlx::query_as(
        "SELECT image_tag FROM projects WHERE slug = $1"
    )
    .bind(&slug)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if let Some((image_tag,)) = record {
        sqlx::query("DELETE FROM projects WHERE slug = $1")
            .bind(&slug)
            .execute(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let _ = state.docker.remove_image(&image_tag, Some(RemoveImageOptions {
            force: true,
            ..Default::default()
        }), None).await;
        
        Ok(Json(format!("Deleted project and image: {}", image_tag)))
    } else {
        Err((StatusCode::NOT_FOUND, "Project not found".to_string()))
    }
}