use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProjectSummary {
    pub slug: String,
    pub image_tag: String,
    pub is_protected: bool,
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
