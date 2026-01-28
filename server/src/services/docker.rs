use bollard::Docker;
use bollard::container::{ListContainersOptions, RemoveContainerOptions};
use std::sync::Arc;
use std::collections::HashMap;
use crate::state::SessionMap;

pub async fn start_background_reaper(docker: Arc<Docker>, sessions: SessionMap) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(60)); 
    loop {
        interval.tick().await;
        
        // Map the struct to the name string
        let active_container_names: Vec<String> = match sessions.lock() {
            Ok(guard) => guard.values().map(|ctx| ctx.container_name.clone()).collect(),
            Err(e) => {
                eprintln!("!! Reaper Mutex Poisoned: {}", e);
                continue; 
            }
        };
        
        let filters = HashMap::from([
            ("label".to_string(), vec!["managed_by=trycli".to_string()])
        ]);
        
        let opts = ListContainersOptions {
            all: true, 
            filters,
            ..Default::default()
        };

        if let Ok(containers) = docker.list_containers(Some(opts)).await {
            for container in containers {
                let is_active = container.names.as_ref().map_or(false, |names| {
                    names.iter().any(|n| {
                        let clean = n.trim_start_matches('/'); 
                        active_container_names.contains(&clean.to_string())
                    })
                });

                if !is_active {
                    if let Some(id) = container.id {
                        println!("Reaper: Killing Zombie Container {}", id);
                        let _ = docker.remove_container(&id, Some(RemoveContainerOptions {
                            force: true, 
                            ..Default::default()
                        })).await;
                    }
                }
            }
        }
    }
}
