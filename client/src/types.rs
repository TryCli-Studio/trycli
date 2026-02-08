use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)] 
pub struct ProjectSummary {
    pub slug: String,
    pub image_tag: String,
    #[serde(default)]
    pub view_count: i64,
    #[serde(default)]
    pub owner_username: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LiveSessionMetric {
    pub slug: String,
    pub uptime_seconds: u64,
    pub container_name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnalyticsDashboardData {
    pub total_lifetime_views: i64,
    pub project_breakdown: Vec<ProjectSummary>,
    pub active_sessions: Vec<LiveSessionMetric>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub login: String,
    pub avatar_url: String,
}