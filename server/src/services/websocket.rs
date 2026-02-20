use axum::{
    extract::{Path, State, ws::{Message, WebSocket, WebSocketUpgrade}},
    response::Response,
    http::StatusCode,
};
use bollard::container::{CreateContainerOptions, Config};
use bollard::models::{HostConfig, Mount, MountTypeEnum, MountTmpfsOptions};
use bollard::image::CreateImageOptions;
// 1. IMPORT ADDED HERE: ResizeExecOptions
use bollard::exec::{CreateExecOptions, StartExecResults, ResizeExecOptions};
use futures::{stream::StreamExt, SinkExt};
use std::collections::HashMap;
use tokio::io::AsyncWriteExt;
use tokio::time::Duration;
use uuid::Uuid;
use tower_sessions::Session; 
use crate::state::{AppState, SessionContext}; 
use crate::models::{User, AnalyticsEventType, log_analytics_event}; 

pub async fn ws_handler(
    ws: WebSocketUpgrade, 
    Path(session_id): Path<String>, 
    State(state): State<AppState>,
    session: Session, 
) -> Result<Response, StatusCode> {
    let user: Option<User> = session.get("user").await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let user_id = user.map(|u| u.id);

    {
        let map = state.lock_sessions();
        
        match map.get(&session_id) {
            Some(ctx) => {
                if let Some(owner) = ctx.owner_id {
                    if Some(owner) != user_id {
                        return Err(StatusCode::FORBIDDEN);
                    }
                }
            },
            None => {
                if user_id.is_none() {
                    return Err(StatusCode::UNAUTHORIZED);
                }
            }
        }
    }

    Ok(ws.on_upgrade(move |socket| handle_socket(socket, state, session_id, user_id)))
}

async fn handle_socket(mut socket: WebSocket, state: AppState, session_id: String, user_id: Option<i64>) {

    // Track if this is a first-time connection for view counting
    let was_previously_connected = {
        let mut map = state.lock_sessions();
        if let Some(ctx) = map.get_mut(&session_id) {
            let was_connected = ctx.is_ws_connected;
            ctx.is_ws_connected = true;
            was_connected
        } else {
            false
        }
    };

    // Count view ONLY on first WebSocket connection (not on HTTP GET)
    if !was_previously_connected {
        let session_ctx = {
            let map = state.lock_sessions();
            map.get(&session_id).cloned()
        };

        // Only count views for viewer sessions (not builders)
        if let Some(ctx) = session_ctx {
            if let (Some(owner_id), Some(slug)) = (ctx.project_owner_id, &ctx.project_slug) {
                let db_clone = state.db.clone();
                let slug_clone = slug.clone();
                tokio::spawn(async move {
                    // Increment view counter
                    let _ = sqlx::query("UPDATE projects SET view_count = view_count + 1 WHERE owner_id = $1 AND LOWER(slug) = LOWER($2)")
                        .bind(owner_id)
                        .bind(&slug_clone)
                        .execute(&db_clone)
                        .await;
                    
                    // Log View event for analytics
                    if let Ok(Some(project_id)) = sqlx::query_scalar::<_, i64>(
                        "SELECT id FROM projects WHERE owner_id = $1 AND LOWER(slug) = LOWER($2)"
                    )
                    .bind(owner_id)
                    .bind(&slug_clone)
                    .fetch_optional(&db_clone)
                    .await
                    {
                        log_analytics_event(&db_clone, project_id, AnalyticsEventType::View, None, None).await;
                    }
                });
            }
        }
    }

    // 1. Check if we need to Spawn a Viewer Container (Lazy Loading)
    let pending_spawn = {
        let map = state.lock_sessions();
        if let Some(ctx) = map.get(&session_id) {
            // If we have an image tag but no container name, it's a viewer waiting to start
            if ctx.container_name.is_empty() && ctx.pending_image_tag.is_some() {
                Some((ctx.pending_image_tag.clone().unwrap(), ctx.shell.clone()))
            } else {
                None
            }
        } else {
            None
        }
    };

    if let Some((image_tag, shell)) = pending_spawn {
        // Perform the spawn that used to be in get_project
        let container_name = format!("trycli-studio-viewer-{}", Uuid::new_v4());
        
        let config = Config {
            image: Some(image_tag),
            labels: Some(HashMap::from([
                ("managed_by".to_string(), "TryCli Studio".to_string())
            ])),
            tty: Some(true),
            user: Some("root".to_string()), 
            // FIX: Run sleep infinity as PID 1. This uses almost 0 CPU/RAM.
            // The actual shell will be run via exec later.
            cmd: Some(vec!["sleep".to_string(), "infinity".to_string()]), 
            env: Some(vec![
                "LANG=C.UTF-8".to_string(), 
                "LC_ALL=C.UTF-8".to_string(),
                "TERM=xterm-256color".to_string(),
                format!("SHELL={}", shell) 
            ]),
            host_config: Some(HostConfig { 
                runtime: Some("kata-clh".to_string()),
                memory: Some(512 * 1024 * 1024), 
                nano_cpus: Some(500_000_000),
                pids_limit: Some(128),
                network_mode: Some("bridge".to_string()), 
                cap_drop: Some(vec!["ALL".to_string()]),
                cap_add: Some(vec![
                    "NET_BIND_SERVICE".to_string(),
                    "CHOWN".to_string(),
                    "SETUID".to_string(),
                    "SETGID".to_string(),
                    "DAC_OVERRIDE".to_string()
                ]),
                security_opt: Some(vec!["no-new-privileges".to_string()]),
                mounts: Some(vec![
                    Mount {
                        target: Some("/tmp".to_string()), 
                        typ: Some(MountTypeEnum::TMPFS), 
                        tmpfs_options: Some(MountTmpfsOptions {
                            size_bytes: Some(50 * 1024 * 1024),
                            mode: Some(0o1777),
                        }),
                        ..Default::default()
                    }
                ]),
                auto_remove: Some(true), 
                ..Default::default() 
            }),
            ..Default::default()
        };

        // Create & Start
        let create_res = state.docker.create_container(
            Some(CreateContainerOptions { name: container_name.clone(), platform: None }), 
            config
        ).await;

        if create_res.is_ok() {
            if state.docker.start_container::<String>(&container_name, None).await.is_ok() {
                // Update SessionContext with the real container name
                let mut map = state.lock_sessions();
                if let Some(ctx) = map.get_mut(&session_id) {
                    ctx.container_name = container_name.clone();
                    ctx.pending_image_tag = None; // clear pending
                }
            } else {
                let _ = socket.send(Message::Text("\r\n\x1b[31m[!] Failed to start container.\x1b[0m\r\n".to_string())).await;
                return;
            }
        } else {
            let _ = socket.send(Message::Text("\r\n\x1b[31m[!] Failed to create container.\x1b[0m\r\n".to_string())).await;
            return;
        }
    }

    let is_claimed_by_us = {
        let mut map = state.lock_sessions();
        if map.contains_key(&session_id) {
            false
        } else {
            map.insert(session_id.clone(), SessionContext {
                container_name: "INITIALIZING".to_string(),
                shell: "".to_string(),
                pending_image_tag: None,
                owner_id: user_id,
                project_owner_id: user_id,
                is_publishing: false,
                project_slug: None, // Builder sessions don't have a specific slug yet
                created_at: std::time::Instant::now(), // Start timer
                is_ws_connected: true,
            });
            true
        }
    };

    if is_claimed_by_us {
        run_setup_wizard(socket, state, session_id, user_id).await;
    } else {
        let existing_session = {
            let map = state.lock_sessions();
            map.get(&session_id).cloned()
        };

        if let Some(ctx) = existing_session {
            if ctx.container_name == "INITIALIZING" {
                let _ = socket.close().await;
                return;
            }
            attach_to_container(socket, state, session_id, ctx.container_name, ctx.shell, None).await;
        }
    }
}

async fn run_setup_wizard(mut socket: WebSocket, state: AppState, session_id: String, _user_id: Option<i64>) {
    async fn send_txt(ws: &mut WebSocket, txt: &str) {
        let _ = ws.send(Message::Text(txt.to_string())).await;
    }
    
    let green = "\x1b[32m";
    let cyan = "\x1b[36m";
    let reset = "\x1b[0m";
    let red = "\x1b[31m";
    let clear = "\x1b[2J\x1b[1;1H";

    send_txt(&mut socket, clear).await;
    send_txt(&mut socket, &format!("{}Welcome to TryCli Studio Setup!{}\r\n\r\n", green, reset)).await;
    
    send_txt(&mut socket, &format!("{}Select Base Image:{}\r\n", cyan, reset)).await;
    send_txt(&mut socket, "1. Ubuntu 22.04 (Heavy, Full-featured)\r\n").await;
    send_txt(&mut socket, "2. Alpine Linux (Lightweight, Fast)\r\n").await;
    send_txt(&mut socket, "3. Debian Bookworm (Stable)\r\n").await;
    send_txt(&mut socket, "Choice [1-3]: ").await;

    let mut distro_choice = 0;
    while let Some(Ok(Message::Text(txt))) = socket.recv().await {
        let input = txt.trim();
        if input == "1" { distro_choice = 1; break; }
        if input == "2" { distro_choice = 2; break; }
        if input == "3" { distro_choice = 3; break; }
    }

    send_txt(&mut socket, "\r\n\r\n").await;
    send_txt(&mut socket, &format!("{}Select Shell (will be installed):{}\r\n", cyan, reset)).await;
    send_txt(&mut socket, "1. Bash (Default)\r\n").await;
    send_txt(&mut socket, "2. Zsh (Oh-My-Zsh ready)\r\n").await;
    send_txt(&mut socket, "3. Fish (Friendly Interactive Shell)\r\n").await;
    send_txt(&mut socket, "Choice [1-3]: ").await;

    let mut shell_choice = 0;
    while let Some(Ok(Message::Text(txt))) = socket.recv().await {
        let input = txt.trim();
        if input == "1" { shell_choice = 1; break; }
        if input == "2" { shell_choice = 2; break; }
        if input == "3" { shell_choice = 3; break; }
    }
    
    send_txt(&mut socket, &format!("\r\n\r\n{}Provisioning Container... Please wait...{}\r\n", green, reset)).await;

    let (image, install_script, final_shell) = match (distro_choice, shell_choice) {
        (2, 1) => ("alpine:latest", "apk add --no-cache bash", "/bin/bash"),
        (2, 2) => ("alpine:latest", "apk add --no-cache zsh", "/bin/zsh"),
        (2, 3) => ("alpine:latest", "apk add --no-cache fish", "/usr/bin/fish"),
        (1, 2) | (3, 2) => (if distro_choice == 1 { "ubuntu:22.04" } else { "debian:bookworm-slim" }, "export DEBIAN_FRONTEND=noninteractive; apt-get update && apt-get install -y zsh", "/usr/bin/zsh"),
        (1, 3) | (3, 3) => (if distro_choice == 1 { "ubuntu:22.04" } else { "debian:bookworm-slim" }, "export DEBIAN_FRONTEND=noninteractive; apt-get update && apt-get install -y fish", "/usr/bin/fish"),
        (1, _) => ("ubuntu:22.04", "true", "/bin/bash"),
        (3, _) => ("debian:bookworm-slim", "true", "/bin/bash"),
        _ => ("debian:bookworm-slim", "true", "/bin/bash"),
    };

    {
        let map = state.lock_sessions();
        if !map.contains_key(&session_id) {
            println!("Wizard: Session {} disconnected early. Aborting spawn.", session_id);
            return;
        }
    }

    let _ = state.docker.create_image(Some(CreateImageOptions { from_image: image, ..Default::default() }), None, None).collect::<Vec<_>>().await;

    {
        let map = state.lock_sessions();
        if !map.contains_key(&session_id) {
            println!("Wizard: Session {} disconnected during pull. Aborting spawn.", session_id);
            return;
        }
    }

    let container_name = format!("trycli-studio-session-{}", Uuid::new_v4());
    let config = Config {
        image: Some(image.to_string()),
        tty: Some(true),
        open_stdin: Some(true),
        // FIX: Use shell instead of tail to keep it alive reliably
        cmd: Some(vec!["sleep".to_string(), "infinity".to_string()]),
        env: Some(vec![
            "LANG=C.UTF-8".to_string(),
            "LC_ALL=C.UTF-8".to_string(),
            "TERM=xterm-256color".to_string()
        ]),
        labels: Some(HashMap::from([
            ("managed_by".to_string(), "TryCli Studio".to_string())
        ])),
        host_config: Some(HostConfig {
            runtime: Some("kata-clh".to_string()),
            memory: Some(512 * 1024 * 1024), 
            memory_swap: Some(1024 * 1024 * 1024), 
            nano_cpus: Some(500_000_000),   

            ulimits: Some(vec![
            bollard::models::ResourcesUlimits {
                name: Some("fsize".to_string()),
                soft: Some(100 * 1024 * 1024), 
                hard: Some(100 * 1024 * 1024), 
            }
            ]),

            mounts: Some(vec![
            Mount {
                target: Some("/tmp".to_string()),
                typ: Some(MountTypeEnum::TMPFS),
                tmpfs_options: Some(MountTmpfsOptions {
                    size_bytes: Some(256 * 1024 * 1024), 
                    mode: Some(0o1777),
                }),
                ..Default::default()
            }
            ]),
            
            pids_limit: Some(128), 

            cap_drop: Some(vec![
                "SYS_ADMIN".to_string(),   
                "NET_RAW".to_string(),     
                "SYS_MODULE".to_string(),  
                "SYS_PTRACE".to_string(),  
                "AUDIT_CONTROL".to_string(), 
                "MAC_ADMIN".to_string(),     
                "SYS_TIME".to_string(),      
            ]),

            security_opt: Some(vec!["no-new-privileges".to_string()]),
            network_mode: Some("bridge".to_string()),

            // FIX: Must be false so we can export stopped containers
            auto_remove: Some(false),
            ..Default::default()
        }),
        ..Default::default()
    };

    match state.docker.create_container(
        Some(CreateContainerOptions { name: container_name.clone(), platform: None }), config
    ).await {
        Ok(_) => {
            // 1. Attempt to start the container
            if let Err(e) = state.docker.start_container::<String>(&container_name, None).await {
                // If start fails, cleanup map and notify client (if still connected)
                {
                    let mut map = state.lock_sessions();
                    map.remove(&session_id);
                }
                send_txt(&mut socket, &format!("{}Fatal Error: Could not start container: {}{}", red, e, reset)).await;
                // Attempt to remove the dead container artifacts
                let _ = state.docker.remove_container(&container_name, Some(
                    bollard::container::RemoveContainerOptions { force: true, ..Default::default() }
                )).await;
                return;
            }

            // 2. CHECK: Is the client still here?
            // We lock the map to check if the session key still exists.
            // If the WS disconnected during 'create_container' or 'start_container', 
            // the 'ws_handler' would have removed the key.
            let session_active = {
                let mut map = state.lock_sessions();
                if map.contains_key(&session_id) {
                    // Update the existing placeholder session with the real container details
                    if let Some(ctx) = map.get_mut(&session_id) {
                        ctx.container_name = container_name.clone();
                        ctx.shell = final_shell.to_string();
                        // owner_id and other fields remain as initialized
                    }
                    true
                } else {
                    false
                }
            };

            // 3. HANDLE ABANDONMENT
            if !session_active {
                println!("Wizard: Session {} abandoned after spawn. Cleaning up immediately.", session_id);
                let _ = state.docker.remove_container(&container_name, Some(
                    bollard::container::RemoveContainerOptions { force: true, ..Default::default() }
                )).await;
                return;
            }

            // 4. PREPARE ENVIRONMENT (Rate limits & Auto-install)
            let limit_config = "Acquire::http::Dl-Limit \"500\"; Acquire::https::Dl-Limit \"500\";";
            let inject_limit_cmd = format!(
                "echo '{}' > /etc/apt/apt.conf.d/99limit", 
                limit_config
            );

            // Chain commands:
            // 1. Disable local echo
            // 2. Create apt config dir
            // 3. Inject apt rate limiting
            // 4. Run the distro-specific install script (e.g. install fish/zsh)
            // 5. Clean the terminal after setup completion
            // 6. Exec into the final requested shell
            let auto_type_cmd = format!(
                "stty -echo; mkdir -p /etc/apt/apt.conf.d && {} && {} && printf '\\033[2J\\033[3J\\033[H' && exec {}\n", 
                inject_limit_cmd,
                install_script, 
                final_shell
            );

            // 5. ATTACH
            // We connect to /bin/sh initially to run the setup script, which then execs into the final shell.
            attach_to_container(socket, state, session_id, container_name, "/bin/sh".to_string(), Some(auto_type_cmd)).await;
        },
        Err(e) => {
            // Container creation failed entirely
            {
                let mut map = state.lock_sessions();
                map.remove(&session_id);
            }
            send_txt(&mut socket, &format!("Error creating container: {}", e)).await;
        }
    }
}

async fn attach_to_container(
    socket: WebSocket, 
    state: AppState,
    session_id: String,
    container_name: String, 
    shell_path: String,
    initial_input: Option<String>
) {
    let config = CreateExecOptions {
        attach_stdout: Some(true), attach_stderr: Some(true), attach_stdin: Some(true),
        tty: Some(true), 
        cmd: Some(vec![shell_path]), 
        ..Default::default()
    };

    let exec = match state.docker.create_exec(&container_name, config).await {
        Ok(e) => e,
        Err(e) => {
            println!("Exec Create Error: {}", e);
            return;
        }
    };

    if let Ok(StartExecResults::Attached { mut output, mut input }) = state.docker.start_exec(&exec.id, None).await {
        let (mut sender, mut receiver) = socket.split();

        if let Some(script) = initial_input {
            let _ = input.write_all(script.as_bytes()).await;
        }

        // TASK 1: Docker Output -> WebSocket Client
        let mut send_task = tokio::spawn(async move {
            while let Some(Ok(msg)) = output.next().await {
                // If the pipe breaks, we just stop
                if sender.send(Message::Text(msg.to_string().into())).await.is_err() {
                    break;
                }
            }
        });

        // TASK 2: WebSocket Client -> Docker Input (WITH IDLE TIMEOUT)
        // Clone session_id for logging within the task
        let session_id_log = session_id.clone();
        
        // 2. FIX: Clone state for the resize task so we don't move the original 'state'
        let state_resize = state.clone();

        let mut recv_task = tokio::spawn(async move {
            const IDLE_TIMEOUT: Duration = Duration::from_secs(60 * 20);

            loop {
                match tokio::time::timeout(IDLE_TIMEOUT, receiver.next()).await {
                    Ok(Some(Ok(msg))) => {
                        match msg {
                            Message::Text(text) => {
                                // --- RESIZE LOGIC START ---
                                if text.starts_with("RESIZE:") {
                                    let parts: Vec<&str> = text.split(':').collect();
                                    if parts.len() == 3 {
                                        if let (Ok(w), Ok(h)) = (parts[1].parse::<u16>(), parts[2].parse::<u16>()) {
                                            // USE state_resize HERE (Arc is cloned)
                                            let _ = state_resize.docker.resize_exec(&exec.id, ResizeExecOptions {
                                                width: w,
                                                height: h
                                            }).await;
                                        }
                                    }
                                } 
                                // --- RESIZE LOGIC END ---
                                else {
                                    if input.write_all(text.as_bytes()).await.is_err() {
                                        break; 
                                    }
                                }
                            },
                            Message::Binary(bin) => {
                                if input.write_all(&bin).await.is_err() {
                                    break;
                                }
                            },
                            Message::Close(_) => break,
                            _ => {} 
                        }
                    },
                    Ok(None) | Ok(Some(Err(_))) => break,
                    Err(_) => {
                        // Timeout
                        println!("Session {} timed out due to inactivity (20m). Closing.", session_id_log);
                        break; 
                    }
                }
            }
        });

        // Wait for EITHER task to finish.
        // If recv_task times out, this select completes, dropping send_task, 
        // and proceeding to the cleanup block below.
        let max_session_duration = Duration::from_secs(60 * 60); // 1 hour

        let _ = tokio::time::timeout(max_session_duration, async {
            tokio::select! {
                _ = &mut send_task => {},
                _ = &mut recv_task => {},
            }
        }).await;

        // --- HANDOFF PROTOCOL: Only delete if NOT publishing ---
        // Capture session context for analytics before removing
        let session_ctx = {
            let map = state.lock_sessions();
            map.get(&session_id).cloned()
        };

        let should_delete = {
            let mut map = state.lock_sessions();
            if let Some(ctx) = map.get(&session_id) {
                if ctx.is_publishing {
                    false 
                } else {
                    map.remove(&session_id); 
                    true 
                }
            } else {
                false
            }
        };

        if should_delete {
            println!("Cleaning up session: {}", session_id);
            
            // Log session end event only for viewer sessions that actually connected
            if let Some(ctx) = session_ctx {
                if ctx.is_ws_connected && ctx.project_slug.is_some() && ctx.project_owner_id.is_some() {
                    let duration = ctx.created_at.elapsed().as_secs() as i64;
                    
                    // Lookup project_id from slug and owner
                    if let (Some(owner_id), Some(slug)) = (ctx.project_owner_id, &ctx.project_slug) {
                        let db_clone = state.db.clone();
                        let slug_clone = slug.clone();
                        tokio::spawn(async move {
                            if let Ok(Some(project_id)) = sqlx::query_scalar::<_, i64>(
                                "SELECT id FROM projects WHERE owner_id = $1 AND LOWER(slug) = LOWER($2)"
                            )
                            .bind(owner_id)
                            .bind(&slug_clone)
                            .fetch_optional(&db_clone)
                            .await
                            {
                                log_analytics_event(&db_clone, project_id, AnalyticsEventType::SessionEnd, Some(duration), None).await;
                            }
                        });
                    }
                }
            }
            
            // USE original 'state' here (it was not moved because we only moved 'state_resize' into the closure)
            let _ = state.docker.remove_container(&container_name, Some(
                bollard::container::RemoveContainerOptions { force: true, ..Default::default() }
            )).await;
        } else {
            println!("Preserving session {} for publishing...", session_id);
        }
    }
}