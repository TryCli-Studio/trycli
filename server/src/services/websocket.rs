use axum::{
    extract::{Path, State, ws::{Message, WebSocket, WebSocketUpgrade}},
    response::IntoResponse,
};
use bollard::container::{CreateContainerOptions, Config};
use bollard::models::HostConfig;
use bollard::image::CreateImageOptions;
use bollard::exec::{CreateExecOptions, StartExecResults};
use futures::{stream::StreamExt, SinkExt};
use std::collections::HashMap;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;
use crate::state::AppState;

pub async fn ws_handler(
    ws: WebSocketUpgrade, 
    Path(session_id): Path<String>, 
    State(state): State<AppState>
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state, session_id))
}

async fn handle_socket(socket: WebSocket, state: AppState, session_id: String) {
    // FIX: Safe lock helper
    let existing_session = {
        let map = state.lock_sessions();
        map.get(&session_id).cloned()
    };

    if let Some((container_id, shell_path)) = existing_session {
        attach_to_container(socket, state, session_id, container_id, shell_path, None).await;
    } else {
        run_setup_wizard(socket, state, session_id).await;
    }
}

async fn run_setup_wizard(mut socket: WebSocket, state: AppState, session_id: String) {
    async fn send_txt(ws: &mut WebSocket, txt: &str) {
        let _ = ws.send(Message::Text(txt.to_string())).await;
    }
    
    let green = "\x1b[32m";
    let cyan = "\x1b[36m";
    let reset = "\x1b[0m";
    let red = "\x1b[31m";
    let clear = "\x1b[2J\x1b[1;1H";

    send_txt(&mut socket, clear).await;
    send_txt(&mut socket, &format!("{}Welcome to TryCLI Setup!{}\r\n\r\n", green, reset)).await;
    
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

    let container_name = format!("trycli-session-{}", Uuid::new_v4());
    let config = Config {
        image: Some(image.to_string()),
        tty: Some(true),
        open_stdin: Some(true), 
        cmd: Some(vec!["tail".to_string(), "-f".to_string(), "/dev/null".to_string()]),
        env: Some(vec![
            "LANG=C.UTF-8".to_string(), 
            "LC_ALL=C.UTF-8".to_string(),
            "TERM=xterm-256color".to_string() 
        ]),
        labels: Some(HashMap::from([
            ("managed_by".to_string(), "trycli".to_string())
        ])),
        host_config: Some(HostConfig { 
            memory: Some(512 * 1024 * 1024), 
            auto_remove: Some(true), 
            ..Default::default() 
        }),
        ..Default::default()
    };

    match state.docker.create_container(
        Some(CreateContainerOptions { name: container_name.clone(), platform: None }), config
    ).await {
        Ok(_) => {
            // FIX: Removed dangerous unwrap() on start_container
            if let Err(e) = state.docker.start_container::<String>(&container_name, None).await {
                send_txt(&mut socket, &format!("{}Fatal Error: Could not start container: {}{}", red, e, reset)).await;
                return;
            }

            {
                // FIX: Safe lock helper
                let mut map = state.lock_sessions();
                map.insert(session_id.clone(), (container_name.clone(), final_shell.to_string()));
            }

            let auto_type_cmd = format!("{} && echo '\r\n\r\n READY ' && exec {}\n", install_script, final_shell);
            attach_to_container(socket, state, session_id, container_name, "/bin/sh".to_string(), Some(auto_type_cmd)).await;
        },
        Err(e) => {
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

        let mut send_task = tokio::spawn(async move {
            while let Some(Ok(Message::Text(text))) = receiver.next().await {
                let _ = input.write_all(text.as_bytes()).await;
            }
        });

        let mut recv_task = tokio::spawn(async move {
            while let Some(Ok(msg)) = output.next().await {
                let _ = sender.send(Message::Text(msg.to_string().into())).await;
            }
        });

        let _ = tokio::select! {
            _ = &mut send_task => {},
            _ = &mut recv_task => {},
        };

        println!("Cleaning up session: {}", session_id);
        {
            // FIX: Safe lock helper
            let mut map = state.lock_sessions();
            map.remove(&session_id);
        }
        let _ = state.docker.stop_container(&container_name, None).await;
    }
}
