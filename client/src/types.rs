use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, serde::Deserialize, PartialEq)]
pub struct ProjectSummary {
    pub slug: String,
    pub image_tag: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct User {
    pub login: String,
    pub avatar_url: String,
}
