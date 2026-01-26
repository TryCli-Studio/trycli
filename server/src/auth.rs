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
use serde::{Deserialize, Serialize};
use tower_sessions::Session;
use crate::AppState;

pub const AUTH_URL: &str = "https://github.com/login/oauth/authorize";
pub const TOKEN_URL: &str = "https://github.com/login/oauth/access_token";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    pub id: i64,
    pub login: String,
    pub avatar_url: String,
}

// Routes for Auth
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/auth/github", get(github_login))
        .route("/auth/callback", get(github_callback))
        .route("/auth/logout", get(logout))
        .route("/api/me", get(get_me))
}

// 1. Redirect user to GitHub
async fn github_login(State(state): State<AppState>) -> impl IntoResponse {
    let client = make_client(&state);
    let (auth_url, _csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("read:user".to_string()))
        .url();

    Redirect::to(auth_url.as_str())
}

// 2. Handle callback from GitHub
#[derive(Deserialize)]
struct AuthRequest { code: String }

async fn github_callback(
    Query(query): Query<AuthRequest>,
    State(state): State<AppState>,
    session: Session,
) -> Result<Redirect, (StatusCode, String)> { // Change return type to Result
    
    println!(">> Callback hit! Code: {}", query.code); // DEBUG LOG

    let client = make_client(&state);
    
    // 1. Exchange Code (Handle error)
    let token = client
        .exchange_code(oauth2::AuthorizationCode::new(query.code))
        .request_async(oauth2::reqwest::async_http_client)
        .await
        .map_err(|e| {
            println!("!! Token Exchange Failed: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Token Error: {}", e))
        })?;

    println!(">> Token received. Fetching User Profile..."); // DEBUG LOG

    // 2. Fetch Profile (Handle error)
    let http_client = reqwest::Client::new();
    let user_data: User = http_client
        .get("https://api.github.com/user")
        .header("User-Agent", "TryCLI")
        .bearer_auth(token.access_token().secret())
        .send()
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Reqwest Error".into()))?
        .json()
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "JSON Error".into()))?;

    println!(">> User fetched: {}", user_data.login); // DEBUG LOG

    // 3. Save Session
    if let Err(e) = session.insert("user", &user_data).await {
         println!("!! Session Insert Failed: {:?}", e);
         return Err((StatusCode::INTERNAL_SERVER_ERROR, "Session Error".into()));
    }

    println!(">> Redirecting to Frontend..."); // DEBUG LOG

    // 4. Redirect
    Ok(Redirect::to("http://localhost:8080/new"))
}

// 3. Helper to check session
async fn get_me(session: Session) -> impl IntoResponse {
    let user: Option<User> = session.get("user").await.unwrap();
    axum::Json(user)
}

fn make_client(state: &AppState) -> BasicClient {
    BasicClient::new(
        ClientId::new(state.github_id.clone()),
        Some(ClientSecret::new(state.github_secret.clone())),
        AuthUrl::new(AUTH_URL.to_string()).unwrap(),
        Some(TokenUrl::new(TOKEN_URL.to_string()).unwrap()),
    )
    .set_redirect_uri(RedirectUrl::new("http://localhost:3000/auth/callback".to_string()).unwrap())
}

async fn logout(session: Session) -> impl IntoResponse {
    session.delete().await.unwrap();
    Redirect::to("http://localhost:8080/") // Redirect to home
}