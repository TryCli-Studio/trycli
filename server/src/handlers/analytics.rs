use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use tower_sessions::Session;
use crate::state::AppState;
use crate::models::{User, AnalyticsDashboardData, ProjectSummary, LiveSessionMetric};

pub async fn get_analytics(
    State(state): State<AppState>,
    session: Session,
) -> Result<Json<AnalyticsDashboardData>, (StatusCode, String)> {
    // 1. Auth Check
    let user: Option<User> = session.get("user")
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Session Error: {}", e)))?;
    let user = user.ok_or((StatusCode::UNAUTHORIZED, "Unauthorized".to_string()))?;

    // 2. Fetch Projects & Lifetime Views
    // FIX: Added 'owner_username' to the SELECT statement
    let projects = sqlx::query_as::<_, ProjectSummary>(
        "SELECT slug, image_tag, view_count, owner_username FROM projects WHERE owner_id = $1 ORDER BY view_count DESC"
    )
    .bind(user.id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let total_lifetime_views: i64 = projects.iter().map(|p| p.view_count).sum();

    // 3. Calculate Live Sessions & Uptime
    let active_sessions = {
        let sessions = state.lock_sessions();
        sessions.values()
            .filter(|ctx| ctx.project_owner_id == Some(user.id) && ctx.project_slug.is_some())
            .map(|ctx| {
                LiveSessionMetric {
                    slug: ctx.project_slug.clone().unwrap_or_default(),
                    container_name: ctx.container_name.clone(),
                    uptime_seconds: std::time::Instant::now().duration_since(ctx.created_at).as_secs(),
                }
            })
            .collect()
    };

    Ok(Json(AnalyticsDashboardData {
        total_lifetime_views,
        project_breakdown: projects,
        active_sessions,
    }))
}