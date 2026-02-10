use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProjectSummary {
    pub slug: String,
    pub image_tag: String,
    #[serde(default)] 
    pub view_count: i64, 
    #[serde(default)] 
    pub owner_username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AnalyticsProjectSummary {
    pub slug: String,
    pub image_tag: String,
    pub view_count: i64,
    pub avg_session_duration: f64,
    pub error_count: i64,
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
    pub views_24h: i64,
    pub views_7d: i64,
    pub views_30d: i64,
    pub views_lifetime: i64,
    pub avg_session_duration: f64,
    pub error_count: i64,
    pub project_breakdown: Vec<AnalyticsProjectSummary>,
    pub active_sessions: Vec<LiveSessionMetric>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[sqlx(type_name = "analytics_event_type", rename_all = "snake_case")]
pub enum AnalyticsEventType {
    View,
    SessionEnd,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AnalyticsEvent {
    pub id: i64,
    pub project_id: i64,
    pub event_type: AnalyticsEventType,
    pub duration_seconds: Option<i64>,
    pub error_type: Option<String>,
    pub created_at: time::OffsetDateTime,
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
    pub is_protected: bool,
    pub allowed_origins: Option<String>,
}

#[derive(Serialize)]
#[serde(tag = "type")]
pub enum OEmbedResponse {
    #[serde(rename = "rich")]
    Rich {
        version: String,
        title: String,
        author_name: String,
        author_url: String,
        provider_name: String,
        provider_url: String,
        html: String,
        width: u32,
        height: u32,
    },
    #[serde(rename = "link")]
    Link {
        version: String,
        title: String,
        author_name: String,
        author_url: String,
        provider_name: String,
        provider_url: String,
        thumbnail_url: Option<String>,
    }
}