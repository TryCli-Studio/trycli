use bollard::Docker;
use std::sync::{Arc, Mutex, MutexGuard};
use std::collections::HashMap;
use std::time::Instant;
use dashmap::DashMap;
use governor::{Quota, RateLimiter, clock::DefaultClock, state::{InMemoryState, NotKeyed}};
use std::num::NonZeroU32;
use crate::handlers::project::WHITELIST_RATE_LIMIT_PER_MINUTE;

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
    // Rate limiter for whitelist operations: per-user tracking
    pub whitelist_rate_limiters: Arc<DashMap<i64, Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>>>,
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

    /// Get or create a rate limiter for a user's whitelist operations
    pub fn get_or_create_whitelist_rate_limiter(&self, user_id: i64) -> Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>> {
        self.whitelist_rate_limiters
            .entry(user_id)
            .or_insert_with(|| {
                let quota = Quota::per_minute(
                    NonZeroU32::new(WHITELIST_RATE_LIMIT_PER_MINUTE)
                        .expect("WHITELIST_RATE_LIMIT_PER_MINUTE must be non-zero")
                );
                Arc::new(RateLimiter::direct(quota))
            })
            .clone()
    }
}