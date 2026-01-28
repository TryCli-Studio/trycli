use bollard::Docker;
use std::sync::{Arc, Mutex, MutexGuard};
use std::collections::HashMap;

// Store (SessionID -> (ContainerName, ShellPath))
pub type SessionMap = Arc<Mutex<HashMap<String, (String, String)>>>;

#[derive(Clone)]
pub struct AppState {
    pub docker: Arc<Docker>,
    pub db: sqlx::PgPool,
    pub github_id: String,
    pub github_secret: String,
    pub sessions: SessionMap,
}

// Helper to handle Mutex Poisoning gracefully without unwrap()
impl AppState {
    pub fn lock_sessions(&self) -> MutexGuard<'_, HashMap<String, (String, String)>> {
        match self.sessions.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                tracing::warn!("Session mutex poisoned. Recovering state.");
                poisoned.into_inner()
            }
        }
    }
}
