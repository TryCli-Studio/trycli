use bollard::Docker;
use bollard::container::{ListContainersOptions, RemoveContainerOptions, StatsOptions};
use std::sync::Arc;
use std::collections::{HashMap, HashSet}; // Import HashSet
use futures::StreamExt; 
use crate::state::SessionMap;

pub async fn start_background_reaper(docker: Arc<Docker>, sessions: SessionMap) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(30)); 
    
    loop {
        interval.tick().await;
        
        // 1. Fetch ALL containers from Docker first
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
                    // Docker names come with a leading slash (e.g., "/keen_eaver")
                    actual_container_names.insert(name.trim_start_matches('/').to_string());
                }
            }
        }

        // --- 2. CLEANUP GHOST SESSIONS (The Fix for your issue) ---
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

        // --- 3. CLEANUP ZOMBIE CONTAINERS (Existing Logic) ---
        // (We can verify map existence again or just use the filtered list logic)
        
        // Build a fresh list of valid names from the now-cleaned map
        let valid_session_names: HashSet<String> = sessions.lock().unwrap().values()
            .map(|c| c.container_name.clone())
            .collect();

        for container in docker_containers {
            let is_known = container.names.as_ref().map_or(false, |names| {
                names.iter().any(|n| valid_session_names.contains(n.trim_start_matches('/')))
            });

            if !is_known {
                if let Some(id) = container.id.clone() {
                    println!("Reaper: Killing Zombie Container {}", id);
                    let _ = docker.remove_container(&id, Some(RemoveContainerOptions {
                        force: true, 
                        ..Default::default()
                    })).await;
                }
            } else {
                // --- 4. RESOURCE MONITORING (Existing Logic) ---
                if let Some(id) = container.id {
                    check_resource_usage(&docker, &id).await;
                }
            }
        }
    }
}

// Helper function to check CPU usage
async fn check_resource_usage(docker: &Docker, container_id: &str) {
    let options = StatsOptions {
        stream: false,
        ..Default::default()
    };

    let mut stats_stream = docker.stats(container_id, Some(options));

    if let Some(Ok(stats)) = stats_stream.next().await {
        // --- EXISTING CPU CHECK ---
        let cpu_delta = stats.cpu_stats.cpu_usage.total_usage as f64 - stats.precpu_stats.cpu_usage.total_usage as f64;
        let system_cpu_usage = stats.cpu_stats.system_cpu_usage.unwrap_or(0);
        let pre_system_cpu_usage = stats.precpu_stats.system_cpu_usage.unwrap_or(0);
        let system_delta = system_cpu_usage as f64 - pre_system_cpu_usage as f64;

        if system_delta > 0.0 && cpu_delta > 0.0 {
            let cpu_percent = (cpu_delta / system_delta) * 100.0;
            if cpu_percent > 90.0 {
                println!("ABUSE: CPU Hog {} ({:.2}%). Killing.", container_id, cpu_percent);
                let _ = docker.remove_container(container_id, Some(RemoveContainerOptions { force: true, ..Default::default() })).await;
                return; // Container killed, exit
            }
        }

        // --- NEW NETWORK CHECK ---
        // Sum up Rx (Received) and Tx (Transmitted) across all interfaces
        let mut total_network_bytes = 0;
        if let Some(networks) = stats.networks {
            for (_, net_stats) in networks {
                total_network_bytes += net_stats.rx_bytes + net_stats.tx_bytes;
            }
        }

        // LIMIT: 1 GB (1024 * 1024 * 1024)
        // If they download/upload more than 1GB total, kill them.
        const NETWORK_LIMIT_BYTES: u64 = 1024 * 1024 * 1024; 

        if total_network_bytes > NETWORK_LIMIT_BYTES {
            println!("ABUSE: Network Limit Exceeded {} ({} bytes). Killing.", container_id, total_network_bytes);
            let _ = docker.remove_container(container_id, Some(RemoveContainerOptions {
                force: true,
                ..Default::default()
            })).await;
        }
    }
}