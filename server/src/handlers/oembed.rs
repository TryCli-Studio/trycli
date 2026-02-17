use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use reqwest::Url;
use serde::Deserialize;
use crate::state::AppState;
use crate::models::OEmbedResponse;

#[derive(Deserialize)]
pub struct OEmbedRequest {
    pub url: String,
}

pub async fn oembed_handler(
    State(state): State<AppState>,
    Query(req): Query<OEmbedRequest>,
) -> Result<Json<OEmbedResponse>, (StatusCode, String)> {
    let url_obj = Url::parse(&req.url)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid URL".to_string()))?;

    if let Some(segments) = url_obj.path_segments() {
        let parts: Vec<&str> = segments.collect();
        
        // CASE 1: Secret Token URL -> Interactive Iframe
        if parts.len() >= 2 && parts[0] == "e" {
            let token = parts[1];
            
            let project = sqlx::query!(
                "SELECT slug, owner_username FROM projects WHERE embed_token = $1", 
                token
            )
            .fetch_optional(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            if let Some(p) = project {
                let origin = std::env::var("FRONTEND_URL").unwrap_or("https://trycli.com".to_string());
                let embed_src = format!("{}/embed/{}/{}", origin, p.owner_username, p.slug);
                
                return Ok(Json(OEmbedResponse::Rich {
                    version: "1.0".to_string(),
                    title: format!("Interactive Demo: {}", p.slug),
                    // FIX: Clone the username here so it doesn't get moved
                    author_name: p.owner_username.clone(), 
                    author_url: format!("{}/{}", origin, p.owner_username),
                    provider_name: "TryCLI Studio".to_string(),
                    provider_url: origin.clone(),
                    html: format!(
                        r#"<iframe src="{}" width="800" height="500" frameborder="0" allowtransparency="true" allow="clipboard-read; clipboard-write"></iframe>"#, 
                        embed_src
                    ),
                    width: 800,
                    height: 500,
                }));
            }
        }
        
        // CASE 2: Public Profile URL -> Static Link Card
        if parts.len() >= 2 {
            let username = parts[0];
            let slug = parts[1];
            
            let exists = sqlx::query!(
                "SELECT 1 as exists FROM projects WHERE owner_username = $1 AND slug = $2",
                username, slug
            )
            .fetch_optional(&state.db)
            .await
            .unwrap_or(None);

            if exists.is_some() {
                let origin = std::env::var("FRONTEND_URL").unwrap_or("https://trycli.com".to_string());
                return Ok(Json(OEmbedResponse::Link {
                    version: "1.0".to_string(),
                    title: format!("Demo: {}", slug),
                    author_name: username.to_string(),
                    author_url: format!("{}/{}", origin, username),
                    provider_name: "TryCLI Studio".to_string(),
                    provider_url: origin,
                    thumbnail_url: Some("https://trycli.com/logo_black.png".to_string()), 
                }));
            }
        }
    }

    Err((StatusCode::NOT_FOUND, "No embeddable project found".to_string()))
}