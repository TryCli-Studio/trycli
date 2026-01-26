use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, Path},
    response::IntoResponse,
    routing::get,
    Router,
};
use bollard::Docker;
use bollard::exec::{CreateExecOptions, StartExecResults};
use futures::{stream::StreamExt, SinkExt}; // Import SinkExt for split()
use std::sync::Arc;
use tokio::io::AsyncWriteExt;

#[tokio::main]
async fn main() {
    let docker = Arc::new(Docker::connect_with_local_defaults().unwrap());

    let app = Router::new()
        .route("/ws/:container_id", get(ws_handler))
        .with_state(docker);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Listening on 0.0.0.0:3000");
    axum::serve(listener, app).await.unwrap();
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    Path(container_id): Path<String>,
    axum::extract::State(docker): axum::extract::State<Arc<Docker>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_terminal_session(socket, docker, container_id))
}

async fn handle_terminal_session(socket: WebSocket, docker: Arc<Docker>, container_id: String) {
    // 1. Create the Docker Exec instance
    let config = CreateExecOptions {
        attach_stdout: Some(true),
        attach_stderr: Some(true),
        attach_stdin: Some(true),
        tty: Some(true),
        cmd: Some(vec!["/bin/bash"]),
        ..Default::default()
    };

    let exec = match docker.create_exec(&container_id, config).await {
        Ok(e) => e,
        Err(e) => {
            println!("Error creating exec: {}", e);
            return;
        }
    };

    // 2. Start the Exec
    if let StartExecResults::Attached { mut output, mut input } = docker.start_exec(&exec.id, None).await.unwrap() {
        
        // 3. SPLIT the WebSocket so we can Read and Write at the same time
        let (mut sender, mut receiver) = socket.split();

        // 4. Task: Browser -> Docker (Input)
        let _send_task = tokio::spawn(async move {
            while let Some(Ok(msg)) = receiver.next().await {
                if let Message::Text(text) = msg {
                    if let Err(e) = input.write_all(text.as_bytes()).await {
                        println!("Failed to write to docker stdin: {}", e);
                        break;
                    }
                }
            }
        });

        // 5. Loop: Docker -> Browser (Output)
        while let Some(Ok(msg)) = output.next().await {
             // Send raw output back to browser
             let text = msg.to_string();
             if let Err(e) = sender.send(Message::Text(text)).await {
                 println!("Failed to send to websocket: {}", e);
                 break;
             }
        }
    };
}