use bollard::Docker;
use bollard::container::{ListContainersOptions, RemoveContainerOptions, StatsOptions};
use std::sync::Arc;
use std::collections::HashMap;
use futures::StreamExt; // <--- CRITICAL FIX: This enables .next() on streams
use crate::state::SessionMap;

pub async fn start_background_reaper(docker: Arc<Docker>, sessions: SessionMap) {
    // Check every 30 seconds
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(30)); 
    
    loop {
        interval.tick().await;
        
        // --- 1. CLEANUP ZOMBIE CONTAINERS ---
        
        let active_container_names: Vec<String> = match sessions.lock() {
            Ok(guard) => guard.values().map(|ctx| ctx.container_name.clone()).collect(),
            Err(e) => {
                eprintln!("!! Reaper Mutex Poisoned: {}", e);
                continue; 
            }
        };
        
        let filters = HashMap::from([
            ("label".to_string(), vec!["managed_by=TryCli Studio".to_string()])
        ]);
        
        let opts = ListContainersOptions {
            all: true, 
            filters,
            ..Default::default()
        };

        if let Ok(containers) = docker.list_containers(Some(opts)).await {
            for container in containers {
                // Check if this container is known to our SessionMap
                let is_active = container.names.as_ref().map_or(false, |names| {
                    names.iter().any(|n| {
                        let clean = n.trim_start_matches('/'); 
                        active_container_names.contains(&clean.to_string())
                    })
                });

                // If not active in our map, kill it
                if !is_active {
                    if let Some(id) = container.id {
                        println!("Reaper: Killing Zombie Container {}", id);
                        let _ = docker.remove_container(&id, Some(RemoveContainerOptions {
                            force: true, 
                            ..Default::default()
                        })).await;
                    }
                } 
                // --- 2. MINING DETECTION (CPU MONITORING) ---
                else {
                    // It is active, but is it misbehaving?
                    if let Some(id) = container.id {
                       check_cpu_usage(&docker, &id).await;
                    }
                }
            }
        }
    }
}

// Helper function to check CPU usage
async fn check_cpu_usage(docker: &Docker, container_id: &str) {
    let options = StatsOptions {
        stream: false, // We only want one snapshot, not a continuous stream
        ..Default::default()
    };

    let mut stats_stream = docker.stats(container_id, Some(options));

    // FIX: .next() is now available because of `use futures::StreamExt;`
    if let Some(Ok(stats)) = stats_stream.next().await {
        // Calculate CPU Usage Percentage
        // Formula: (total_delta / system_delta) * number_of_cpus * 100.0
        let cpu_delta = stats.cpu_stats.cpu_usage.total_usage as f64 - stats.precpu_stats.cpu_usage.total_usage as f64;
        
        // Protect against 0 division or missing system usage data
        let system_cpu_usage = stats.cpu_stats.system_cpu_usage.unwrap_or(0);
        let pre_system_cpu_usage = stats.precpu_stats.system_cpu_usage.unwrap_or(0);
        let system_delta = system_cpu_usage as f64 - pre_system_cpu_usage as f64;

        if system_delta > 0.0 && cpu_delta > 0.0 {
            // Usually docker stats include the number of CPUs in the calculation, 
            // but for a simple "hog" check, raw % relative to system delta is often enough.
            // If you allocated 1.0 CPU, 100% usage means they are using all of it.
            let cpu_percent = (cpu_delta / system_delta) * 100.0;

            // THRESHOLD: If they are using > 90% of the CPU allocated to them
            if cpu_percent > 90.0 {
                println!("ABUSE DETECTED: Container {} using {:.2}% CPU. Killing.", container_id, cpu_percent);
                
                let _ = docker.remove_container(container_id, Some(RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                })).await;
            }
        }
    }
}