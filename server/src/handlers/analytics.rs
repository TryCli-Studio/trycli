use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use tower_sessions::Session;
use crate::state::AppState;
use crate::models::{User, AnalyticsDashboardData, AnalyticsProjectSummary, LiveSessionMetric};

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
    let projects = sqlx::query_as::<_, AnalyticsProjectSummary>(
        "SELECT \
            p.slug, \
            p.image_tag, \
            COALESCE(COUNT(*) FILTER (WHERE ae.event_type = 'view'), 0) AS view_count, \
            COALESCE(AVG(ae.duration_seconds) FILTER (WHERE ae.event_type = 'session_end'), 0)::FLOAT8 AS avg_session_duration, \
            COALESCE(COUNT(*) FILTER (WHERE ae.event_type = 'error'), 0) AS error_count \
        FROM projects p \
        LEFT JOIN analytics_events ae ON ae.project_id = p.id \
        WHERE p.owner_id = $1 \
        GROUP BY p.slug, p.image_tag \
        ORDER BY view_count DESC"
    )
    .bind(user.id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    #[derive(sqlx::FromRow)]
    struct AnalyticsAggregates {
        views_24h: i64,
        views_7d: i64,
        views_30d: i64,
        views_lifetime: i64,
        avg_session_duration: f64,
        error_count: i64,
    }

    let aggregates = sqlx::query_as::<_, AnalyticsAggregates>(
        "SELECT \
            COALESCE(COUNT(*) FILTER (WHERE ae.event_type = 'view' AND ae.created_at > NOW() - INTERVAL '24 hours'), 0) AS views_24h, \
            COALESCE(COUNT(*) FILTER (WHERE ae.event_type = 'view' AND ae.created_at > NOW() - INTERVAL '7 days'), 0) AS views_7d, \
            COALESCE(COUNT(*) FILTER (WHERE ae.event_type = 'view' AND ae.created_at > NOW() - INTERVAL '30 days'), 0) AS views_30d, \
            COALESCE(COUNT(*) FILTER (WHERE ae.event_type = 'view'), 0) AS views_lifetime, \
            COALESCE(AVG(ae.duration_seconds) FILTER (WHERE ae.event_type = 'session_end'), 0)::FLOAT8 AS avg_session_duration, \
            COALESCE(COUNT(*) FILTER (WHERE ae.event_type = 'error'), 0) AS error_count \
        FROM analytics_events ae \
        JOIN projects p ON p.id = ae.project_id \
        WHERE p.owner_id = $1"
    )
    .bind(user.id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

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
        total_lifetime_views: aggregates.views_lifetime,
        views_24h: aggregates.views_24h,
        views_7d: aggregates.views_7d,
        views_30d: aggregates.views_30d,
        views_lifetime: aggregates.views_lifetime,
        avg_session_duration: aggregates.avg_session_duration,
        error_count: aggregates.error_count,
        project_breakdown: projects,
        active_sessions,
    }))
}