use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, serde::Deserialize, PartialEq)]
pub struct ProjectSummary {
    pub slug: String,
    pub image_tag: String,
    pub is_protected: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub login: String,
    pub avatar_url: String,
}
