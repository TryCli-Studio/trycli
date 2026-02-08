use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProjectSummary {
    pub slug: String,
    pub image_tag: String,
    // Add this to existing struct or ensure query maps correctly
    #[serde(default)] 
    pub view_count: i64, 
}

#[derive(Serialize)]
pub struct LiveSessionMetric {
    pub slug: String,
    pub uptime_seconds: u64,
    pub container_name: String,
}

#[derive(Serialize)]
pub struct AnalyticsDashboardData {
    pub total_lifetime_views: i64,
    pub project_breakdown: Vec<ProjectSummary>,
    pub active_sessions: Vec<LiveSessionMetric>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    pub id: i64,
    pub login: String,
    pub avatar_url: String,
}

#[derive(Deserialize)]
pub struct PublishRequest {
    pub container_id: String,
    pub slug: String,
    pub markdown: String,
}