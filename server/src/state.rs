use bollard::Docker;
use std::sync::{Arc, Mutex, MutexGuard};
use std::collections::HashMap;

// New Struct to track container details AND ownership
#[derive(Clone, Debug)]
pub struct SessionContext {
    pub container_name: String,
    pub shell: String,
    // If Some(id), only that user can access (Private/Builder). 
    // If None, it's public (Viewer).
    pub owner_id: Option<i64>, 
    // NEW: The ID of the user who PUBLISHED this project (or is building it).
    // This allows us to count total active slots per publisher.
    pub project_owner_id: Option<i64>,
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