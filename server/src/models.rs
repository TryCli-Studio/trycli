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
    pub embed_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AnalyticsProjectSummary {
    pub slug: String,
    pub image_tag: String,
    pub view_count: i64,
    pub avg_session_duration: f64,
    pub error_count: i64,
}

#[derive(Deserialize)]
pub struct WhitelistRequest {
    pub allowed_url: String,
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