use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
    routing::get,
    Router,
};
use oauth2::{
    basic::BasicClient, AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenResponse, TokenUrl,
};
use serde::Deserialize;
use tower_sessions::Session;
use crate::models::User;
use crate::state::AppState;

pub const AUTH_URL: &str = "https://github.com/login/oauth/authorize";
pub const TOKEN_URL: &str = "https://github.com/login/oauth/access_token";

pub const DEMO_USER_ID: i64 = 1;
pub const DEMO_USER_LOGIN: &str = "yashb404";
pub const DEMO_USER_AVATAR: &str = "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 32 32'%3E%3Crect width='32' height='32' fill='%2327272a'/%3E%3Ccircle cx='16' cy='11' r='6' fill='%2371717a'/%3E%3Cpath d='M6 28 c0-6 4-9 10-9 s10 3 10 9 Z' fill='%2371717a'/%3E%3C/svg%3E";

/// Ensures that a demo user exists in the database and is placed into the session
/// if no user is already logged in.
pub async fn get_current_user_or_demo(
    session: &Session,
    db: &sqlx::PgPool,
) -> Result<User, (StatusCode, String)> {
    if let Ok(Some(user)) = session.get::<User>("user").await {
        if user.login == DEMO_USER_LOGIN {
            return Ok(user);
        }
    }

    // Ensure demo user exists in the database
    sqlx::query(
        "INSERT INTO users (id, username, avatar_url, last_login) 
         VALUES ($1, $2, $3, NOW())
         ON CONFLICT (id) DO UPDATE 
         SET username = $2, avatar_url = $3, last_login = NOW()",
    )
    .bind(DEMO_USER_ID)
    .bind(DEMO_USER_LOGIN)
    .bind(DEMO_USER_AVATAR)
    .execute(db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB Error: {e}")))?;

    let demo_user = User {
        id: DEMO_USER_ID,
        login: DEMO_USER_LOGIN.to_string(),
        avatar_url: DEMO_USER_AVATAR.to_string(),
    };

    session
        .insert("user", &demo_user)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Session Error: {e}")))?;

    Ok(demo_user)
}

// Routes for Auth
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/auth/github", get(github_login))
        .route("/auth/callback", get(github_callback))
        .route("/auth/logout", get(logout))
        .route("/api/me", get(get_me))
}

// 1. Skip login and redirect straight to dashboard with demo user in session
async fn github_login(
    State(state): State<AppState>,
    session: Session,
) -> Result<Redirect, (StatusCode, String)> {
    let _ = get_current_user_or_demo(&session, &state.db).await?;

    let frontend_url = std::env::var("FRONTEND_URL")
        .unwrap_or_else(|_| "http://localhost:8080".to_string());
    let dashboard_url = format!("{}/dashboard", frontend_url.trim_end_matches('/'));

    Ok(Redirect::to(&dashboard_url))
}

// 2. Handle callback from GitHub (preserved in case OAuth is ever re-enabled)
#[derive(Deserialize)]
struct AuthRequest {
    code: String,
}

async fn github_callback(
    Query(query): Query<AuthRequest>,
    State(state): State<AppState>,
    session: Session,
) -> Result<Redirect, (StatusCode, String)> {
    let client = make_client(&state).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    // Exchange Code
    let token = client
        .exchange_code(oauth2::AuthorizationCode::new(query.code))
        .request_async(oauth2::reqwest::async_http_client)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Token Error: {e}")))?;

    // Fetch Profile
    let http_client = reqwest::Client::new();
    let user_data: User = http_client
        .get("https://api.github.com/user")
        .header("User-Agent", "TryCli Studio")
        .bearer_auth(token.access_token().secret())
        .send()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Reqwest Error: {e}")))?
        .json()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("JSON Error: {e}")))?;

    sqlx::query(
        "INSERT INTO users (id, username, avatar_url, last_login) 
         VALUES ($1, $2, $3, NOW())
         ON CONFLICT (id) DO UPDATE 
         SET username = $2, avatar_url = $3, last_login = NOW()",
    )
    .bind(user_data.id)
    .bind(&user_data.login)
    .bind(&user_data.avatar_url)
    .execute(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB Error: {e}")))?;

    // Save Session
    session
        .insert("user", &user_data)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Session Insert Error: {e}")))?;

    let frontend_url = std::env::var("FRONTEND_URL")
        .unwrap_or_else(|_| "http://localhost:8080".to_string());
    let dashboard_url = format!("{}/dashboard", frontend_url.trim_end_matches('/'));

    Ok(Redirect::to(&dashboard_url))
}

// 3. Helper to check session or auto-initialize demo user
async fn get_me(
    State(state): State<AppState>,
    session: Session,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let user = get_current_user_or_demo(&session, &state.db).await?;
    Ok(axum::Json(Some(user)))
}

// 4. Helper to create OAuth client
fn make_client(state: &AppState) -> Result<BasicClient, String> {
    let auth_url =
        AuthUrl::new(AUTH_URL.to_string()).map_err(|e| format!("Invalid Auth URL: {e}"))?;

    let token_url =
        TokenUrl::new(TOKEN_URL.to_string()).map_err(|e| format!("Invalid Token URL: {e}"))?;

    let api_url =
        std::env::var("API_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

    let redirect_url =
        RedirectUrl::new(format!("{}/auth/callback", api_url.trim_end_matches('/')))
            .map_err(|e| format!("Invalid Redirect URL: {e}"))?;

    Ok(BasicClient::new(
        ClientId::new(state.github_id.clone()),
        Some(ClientSecret::new(state.github_secret.clone())),
        auth_url,
        Some(token_url),
    )
    .set_redirect_uri(redirect_url))
}

async fn logout(session: Session) -> Result<impl IntoResponse, (StatusCode, String)> {
    session
        .delete()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let redirect_url = std::env::var("FRONTEND_URL")
        .unwrap_or_else(|_| "http://localhost:8080".to_string());

    Ok(Redirect::to(&redirect_url))
}