use axum::{
    extract::{Query, State},
    response::{Redirect, IntoResponse},
    http::StatusCode,
    routing::get,
    Router,
};
use oauth2::{
    basic::BasicClient, AuthUrl, ClientId, ClientSecret, CsrfToken, 
    RedirectUrl, Scope, TokenUrl, TokenResponse,
};
use serde::Deserialize;
use tower_sessions::Session;
use crate::state::AppState;
use crate::models::User;

pub const AUTH_URL: &str = "https://github.com/login/oauth/authorize";
pub const TOKEN_URL: &str = "https://github.com/login/oauth/access_token";

// Routes for Auth
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/auth/github", get(github_login))
        .route("/auth/callback", get(github_callback))
        .route("/auth/logout", get(logout))
        .route("/api/me", get(get_me))
}

// 1. Redirect user to GitHub
async fn github_login(State(state): State<AppState>) -> Result<impl IntoResponse, (StatusCode, String)> {
    // FIX: Handle config errors gracefully
    let client = make_client(&state).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let (auth_url, _csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("read:user".to_string()))
        .url();
    Ok(Redirect::to(auth_url.as_str()))
}

// 2. Handle callback from GitHub
#[derive(Deserialize)]
struct AuthRequest { code: String }

async fn github_callback(
    Query(query): Query<AuthRequest>,
    State(state): State<AppState>,
    session: Session,
) -> Result<Redirect, (StatusCode, String)> { 
    // FIX: Handle config errors gracefully
    let client = make_client(&state).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    // 1. Exchange Code
    let token = client
        .exchange_code(oauth2::AuthorizationCode::new(query.code))
        .request_async(oauth2::reqwest::async_http_client)
        .await
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Token Error: {}", e))
        })?;

    // 2. Fetch Profile
    let http_client = reqwest::Client::new();
    let user_data: User = http_client
        .get("https://api.github.com/user")
        .header("User-Agent", "TryCli Studio")
        .bearer_auth(token.access_token().secret())
        .send()
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Reqwest Error".into()))?
        .json()
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "JSON Error".into()))?;

    // 3. Save Session
    session.insert("user", &user_data)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Session Insert Error".into()))?;
    
    // 4. Redirect
    Ok(Redirect::to("http://localhost:8080/dashboard"))
}

// 3. Helper to check session
async fn get_me(session: Session) -> Result<impl IntoResponse, (StatusCode, String)> {
    let user: Option<User> = session.get("user")
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Session Read Error: {}", e)))?;
    
    Ok(axum::Json(user))
}

// 4. Helper to create OAuth client (Now returns Result)
fn make_client(state: &AppState) -> Result<BasicClient, String> {
    let auth_url = AuthUrl::new(AUTH_URL.to_string())
        .map_err(|e| format!("Invalid Auth URL: {}", e))?;
    
    let token_url = TokenUrl::new(TOKEN_URL.to_string())
        .map_err(|e| format!("Invalid Token URL: {}", e))?;

    let api_url = std::env::var("API_URL")
        .unwrap_or_else(|_| "http://localhost:3000".to_string());
        
    let redirect_url = RedirectUrl::new(format!("{}/auth/callback", api_url))
        .map_err(|e| format!("Invalid Redirect URL: {}", e))?;

    Ok(BasicClient::new(
        ClientId::new(state.github_id.clone()),
        Some(ClientSecret::new(state.github_secret.clone())),
        auth_url,
        Some(token_url),
    )
    .set_redirect_uri(redirect_url))
}

async fn logout(session: Session) -> Result<impl IntoResponse, (StatusCode, String)> {
    session.delete().await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Redirect::to("http://localhost:8080/")) 
}