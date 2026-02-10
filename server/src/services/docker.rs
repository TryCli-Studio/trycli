use bollard::Docker;
use bollard::container::{ListContainersOptions, RemoveContainerOptions, StatsOptions};
use std::sync::Arc;
use std::collections::{HashMap, HashSet}; // Import HashSet
use futures::StreamExt; 
use crate::state::{SessionMap, SessionContext};
use std::time::{SystemTime, UNIX_EPOCH};
use crate::models::AnalyticsEventType;

pub async fn start_background_reaper(docker: Arc<Docker>, sessions: SessionMap, db: sqlx::PgPool) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(30)); 
    
    loop {
        interval.tick().await;
        
        // --- 1. CLEANUP ZOMBIE CONTAINERS ---
        let session_snapshot: HashMap<String, (String, SessionContext)> = match sessions.lock() {
            Ok(guard) => guard.iter().map(|(session_id, ctx)| {
                (ctx.container_name.clone(), (session_id.clone(), ctx.clone()))
            }).collect(),
            Err(e) => {
                eprintln!("!! Reaper Mutex Poisoned: {}", e);
                continue; 
            }
        };

        // --- 2. FETCH ALL CONTAINERS FROM DOCKER ---
        let filters = HashMap::from([
            ("label".to_string(), vec!["managed_by=TryCli Studio".to_string()])
        ]);
        
        let opts = ListContainersOptions {
            all: true, 
            filters,
            ..Default::default()
        };

        // Get the "Ground Truth" from Docker
        let docker_containers = match docker.list_containers(Some(opts)).await {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Reaper failed to list containers: {}", e);
                continue;
            }
        };

        // Create a HashSet of actual running/existing container names
        let mut actual_container_names: HashSet<String> = HashSet::new();
        for c in &docker_containers {
            if let Some(names) = &c.names {
                for name in names {
                    actual_container_names.insert(name.trim_start_matches('/').to_string());
                }
            }
        }

        // --- 3. CLEANUP GHOST SESSIONS  ---
        {
            let mut map = match sessions.lock() {
                Ok(g) => g,
                Err(e) => {
                    eprintln!("Reaper Lock Error: {}", e);
                    continue; 
                }
            };

            // Remove sessions from memory if they don't exist in Docker
            map.retain(|_session_id, ctx| {
                // Always keep initializing sessions (give them 30s grace period)
                if ctx.container_name == "INITIALIZING" {
                    return true;
                }
                
                let exists = actual_container_names.contains(&ctx.container_name);
                
                if !exists {
                    println!("Reaper: Removing Ghost Session (Memory leak) for {}", ctx.container_name);
                }
                
                exists
            });
        }

        // --- 4. CLEANUP ZOMBIE CONTAINERS ---
        // Build a fresh list of valid names from the now-cleaned map
        let valid_session_names: HashSet<String> = sessions.lock().unwrap().values()
            .map(|c| c.container_name.clone())
            .collect();

        let now_ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;

        for container in docker_containers {
            let is_known = container.names.as_ref().map_or(false, |names| {
                names.iter().any(|n| valid_session_names.contains(n.trim_start_matches('/')))
            });

            if !is_known {
                // --- FIX: Grace Period Check ---
                let created = container.created.unwrap_or(0);
                let age = now_ts - created;

                if age > 30 {
                    if let Some(id) = container.id.clone() {
                        println!("Reaper: Killing Zombie Container {} (Age: {}s)", id, age);
                        let _ = docker.remove_container(&id, Some(RemoveContainerOptions {
                            force: true, 
                            ..Default::default()
                        })).await;
                    }
                } 
            } else {
                // --- MINING DETECTION / CPU MONITORING ---
                if let Some(id) = container.id {
                    let container_name = container.names.as_ref()
                        .and_then(|names| names.first())
                        .map(|n| n.trim_start_matches('/').to_string())
                        .unwrap_or_default();

                    if let Some((session_id, ctx)) = session_snapshot.get(&container_name) {
                        let abuse_killed = check_resource_usage(&docker, &id).await;

                        if abuse_killed {
                            log_abuse_and_session_end(&db, ctx).await;

                            let mut map = sessions.lock().unwrap_or_else(|p| p.into_inner());
                            map.remove(session_id);
                        }
                    } else {
                        check_resource_usage(&docker, &id).await;
                    }
                }
            }
        }

        // --- 5. CLEANUP ABANDONED SESSIONS ---
        let abandoned_sessions: Vec<(String, String)> = {
            let map = sessions.lock().unwrap();
            map.iter()
                .filter(|(_, ctx)| {
                    // If it's not connected AND it's older than 45 seconds
                    !ctx.is_ws_connected && ctx.created_at.elapsed().as_secs() > 45
                })
                .map(|(id, ctx)| (id.clone(), ctx.container_name.clone()))
                .collect()
        };

        for (session_id, container_name) in abandoned_sessions {
            println!("Reaper: Killing Abandoned Session {} (Never connected)", session_id);
            
            // 1. Remove from Map
            {
                let mut map = sessions.lock().unwrap();
                map.remove(&session_id);
            }

            // 2. Kill Container
            let _ = docker.remove_container(&container_name, Some(RemoveContainerOptions {
                force: true, 
                ..Default::default()
            })).await;
        }
    }
}

async fn check_resource_usage(docker: &Docker, container_id: &str) -> bool {
    let options = StatsOptions {
        stream: false,
        ..Default::default()
    };

    let mut stats_stream = docker.stats(container_id, Some(options));

    if let Some(Ok(stats)) = stats_stream.next().await {
        // --- CPU CHECK (From feat) ---
        let cpu_delta = stats.cpu_stats.cpu_usage.total_usage as f64 - stats.precpu_stats.cpu_usage.total_usage as f64;
        let system_cpu_usage = stats.cpu_stats.system_cpu_usage.unwrap_or(0);
        let pre_system_cpu_usage = stats.precpu_stats.system_cpu_usage.unwrap_or(0);
        let system_delta = system_cpu_usage as f64 - pre_system_cpu_usage as f64;

        if system_delta > 0.0 && cpu_delta > 0.0 {
            let cpu_percent = (cpu_delta / system_delta) * 100.0;
            if cpu_percent > 90.0 {
                println!("ABUSE: CPU Hog {} ({:.2}%). Killing.", container_id, cpu_percent);
                let _ = docker.remove_container(container_id, Some(RemoveContainerOptions { force: true, ..Default::default() })).await;
                return true;
            }
        }

        // --- NETWORK CHECK ---
        let mut total_network_bytes = 0;
        if let Some(networks) = stats.networks {
            for (_, net_stats) in networks {
                total_network_bytes += net_stats.rx_bytes + net_stats.tx_bytes;
            }
        }

        const NETWORK_LIMIT_BYTES: u64 = 1024 * 1024 * 1024; 

        if total_network_bytes > NETWORK_LIMIT_BYTES {
            println!("ABUSE: Network Limit Exceeded {} ({} bytes). Killing.", container_id, total_network_bytes);
            let _ = docker.remove_container(container_id, Some(RemoveContainerOptions {
                force: true,
                ..Default::default()
            })).await;
            return true;
        }
    }

    false
}

async fn log_abuse_and_session_end(db: &sqlx::PgPool, ctx: &SessionContext) {
    let project_id = match (ctx.project_owner_id, ctx.project_slug.as_deref()) {
        (Some(owner_id), Some(slug)) => {
            sqlx::query_scalar::<_, i64>(
                "SELECT id FROM projects WHERE owner_id = $1 AND LOWER(slug) = LOWER($2)"
            )
            .bind(owner_id)
            .bind(slug)
            .fetch_optional(db)
            .await
            .ok()
            .flatten()
        }
        _ => None,
    };

    let Some(project_id) = project_id else {
        return;
    };

    let duration_seconds = std::time::Instant::now()
        .duration_since(ctx.created_at)
        .as_secs() as i64;

    let _ = sqlx::query(
        "INSERT INTO analytics_events (project_id, event_type, duration_seconds) VALUES ($1, $2, $3)"
    )
    .bind(project_id)
    .bind(AnalyticsEventType::SessionEnd)
    .bind(duration_seconds)
    .execute(db)
    .await;

    let _ = sqlx::query(
        "INSERT INTO analytics_events (project_id, event_type, error_type) VALUES ($1, $2, $3)"
    )
    .bind(project_id)
    .bind(AnalyticsEventType::Error)
    .bind("ABUSE")
    .execute(db)
    .await;
}