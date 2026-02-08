use axum::{
    extract::{Path, State, ws::{Message, WebSocket, WebSocketUpgrade}},
    response::Response,
    http::StatusCode,
};
use bollard::container::{CreateContainerOptions, Config};
use bollard::models::{HostConfig, Mount, MountTypeEnum, MountTmpfsOptions};
use bollard::image::CreateImageOptions;
use bollard::exec::{CreateExecOptions, StartExecResults};
use futures::{stream::StreamExt, SinkExt};
use std::collections::HashMap;
use tokio::io::AsyncWriteExt;
use tokio::time::Duration;
use uuid::Uuid;
use tower_sessions::Session; 
use crate::state::{AppState, SessionContext}; 
use crate::models::User; 

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

async fn handle_socket(socket: WebSocket, state: AppState, session_id: String, user_id: Option<i64>) {
    let is_claimed_by_us = {
        let mut map = state.lock_sessions();
        if map.contains_key(&session_id) {
            false
        } else {
            map.insert(session_id.clone(), SessionContext {
                container_name: "INITIALIZING".to_string(),
                shell: "".to_string(),
                owner_id: user_id,
                project_owner_id: user_id,
                is_publishing: false,
                project_slug: None, // Builder sessions don't have a specific slug yet
                created_at: std::time::Instant::now(), // Start timer
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

async fn run_setup_wizard(mut socket: WebSocket, state: AppState, session_id: String, user_id: Option<i64>) {
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

    let _ = state.docker.create_image(Some(CreateImageOptions { from_image: image, ..Default::default() }), None, None).collect::<Vec<_>>().await;

    let container_name = format!("trycli-studio-session-{}", Uuid::new_v4());
    let config = Config {
        image: Some(image.to_string()),
        tty: Some(true),
        open_stdin: Some(true),
        // FIX: Use shell instead of tail to keep it alive reliably
        cmd: Some(vec!["/bin/sh".to_string()]),
        env: Some(vec![
            "LANG=C.UTF-8".to_string(),
            "LC_ALL=C.UTF-8".to_string(),
            "TERM=xterm-256color".to_string()
        ]),
        labels: Some(HashMap::from([
            ("managed_by".to_string(), "TryCli Studio".to_string())
        ])),
        host_config: Some(HostConfig {
            runtime: Some("runsc".to_string()),
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
            if let Err(e) = state.docker.start_container::<String>(&container_name, None).await {
                {
                    let mut map = state.lock_sessions();
                    map.remove(&session_id);
                }
                send_txt(&mut socket, &format!("{}Fatal Error: Could not start container: {}{}", red, e, reset)).await;
                return;
            }

            {
                let mut map = state.lock_sessions();
                map.insert(session_id.clone(), SessionContext {
                    container_name: container_name.clone(),
                    shell: final_shell.to_string(),
                    owner_id: user_id,
                    project_owner_id: user_id,
                    is_publishing: false, 
                    project_slug: None,
                    created_at: std::time::Instant::now(), 
                });
            }
            let limit_config = "Acquire::http::Dl-Limit \"500\"; Acquire::https::Dl-Limit \"500\";";
            let inject_limit_cmd = format!(
            "echo '{}' > /etc/apt/apt.conf.d/99limit", 
            limit_config
            );

            let auto_type_cmd = format!(
                "mkdir -p /etc/apt/apt.conf.d && {} && {} && echo '\r\n\r\n READY ' && exec {}\n", 
                inject_limit_cmd,
                install_script, 
                final_shell
            );
            attach_to_container(socket, state, session_id, container_name, "/bin/sh".to_string(), Some(auto_type_cmd)).await;
        },
        Err(e) => {
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
        
        let mut recv_task = tokio::spawn(async move {
            // Guardrail: 20 Minute Idle Timeout
            const IDLE_TIMEOUT: Duration = Duration::from_secs(60 * 20);

            loop {
                // We wrap the receiver.next() in a timeout
                match tokio::time::timeout(IDLE_TIMEOUT, receiver.next()).await {
                    // Case A: Received a message within time limit
                    Ok(Some(Ok(msg))) => {
                        match msg {
                            Message::Text(text) => {
                                if input.write_all(text.as_bytes()).await.is_err() {
                                    break; // Container stdin closed
                                }
                            },
                            Message::Binary(bin) => {
                                if input.write_all(&bin).await.is_err() {
                                    break;
                                }
                            },
                            Message::Close(_) => break, // Client closed tab
                            _ => {} // Ignore Pings/Pongs (handled by Axum)
                        }
                    },
                    // Case B: Stream ended normally (client disconnected)
                    Ok(None) | Ok(Some(Err(_))) => break,
                    
                    // Case C: IDLE TIMEOUT HIT
                    Err(_) => {
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
            let _ = state.docker.remove_container(&container_name, Some(
                bollard::container::RemoveContainerOptions { force: true, ..Default::default() }
            )).await;
        } else {
            println!("Preserving session {} for publishing...", session_id);
        }
    }
}