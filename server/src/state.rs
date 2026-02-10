use bollard::Docker;
use std::sync::{Arc, Mutex, MutexGuard};
use std::collections::HashMap;
use std::time::Instant;

// New Struct to track container details AND ownership
#[derive(Clone, Debug)]
pub struct SessionContext {
    pub container_name: String,
    pub shell: String,
    pub owner_id: Option<i64>,
    pub project_owner_id: Option<i64>,
    pub is_publishing: bool,
    pub project_slug: Option<String>, 
    pub created_at: Instant,
    pub is_ws_connected: bool,
}

pub type SessionMap = Arc<Mutex<HashMap<String, SessionContext>>>;

#[derive(Clone)]
pub struct AppState {
    pub docker: Arc<Docker>,
    pub db: sqlx::PgPool,
    pub github_id: String,
    pub github_secret: String,
    pub sessions: SessionMap,
}

impl AppState {
    pub fn lock_sessions(&self) -> MutexGuard<'_, HashMap<String, SessionContext>> {
        match self.sessions.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                tracing::warn!("Session mutex poisoned. Recovering state.");
                poisoned.into_inner()
            }
        }
    }
}