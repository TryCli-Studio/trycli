use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)] 
pub struct ProjectSummary {
    pub slug: String,
    pub image_tag: String,
    pub is_protected: bool,
    #[serde(default)]
    pub view_count: i64,
    #[serde(default)]
    pub owner_username: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct AnalyticsProjectSummary {
    pub slug: String,
    pub image_tag: String,
    #[serde(default)]
    pub view_count: i64,
    #[serde(default)]
    pub avg_session_duration: f64,
    #[serde(default)]
    pub error_count: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LiveSessionMetric {
    pub slug: String,
    pub uptime_seconds: u64,
    pub container_name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnalyticsDashboardData {
    #[serde(default)]
    pub total_lifetime_views: i64,
    #[serde(default)]
    pub views_24h: i64,
    #[serde(default)]
    pub views_7d: i64,
    #[serde(default)]
    pub views_30d: i64,
    #[serde(default)]
    pub views_lifetime: i64,
    #[serde(default)]
    pub avg_session_duration: f64,
    #[serde(default)]
    pub error_count: i64,
    pub project_breakdown: Vec<AnalyticsProjectSummary>,
    pub active_sessions: Vec<LiveSessionMetric>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub login: String,
    pub avatar_url: String,
}