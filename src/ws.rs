use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}},
    response::IntoResponse,
};
use futures_util::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use crate::pty::Pty;
use crate::io::{prepare_pty_stream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{error, info};
use std::sync::Arc;
use std::os::unix::io::AsRawFd;

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum TerminalMessage {
    Resize { rows: u16, cols: u16 },
    Input(String),
}

/// WebSocket handler for PTY terminal interaction
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    // Add application state here if needed
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket))
}

async fn handle_socket(mut socket: WebSocket) {
    info!("WebSocket connection established for terminal session");

    // 1. Spawn a PTY
    let pty = match Pty::spawn() {
        Ok(p) => Arc::new(p),
        Err(e) => {
            error!("Failed to spawn PTY: {}", e);
            let _ = socket.send(Message::Text(format!("Error: {}", e))).await;
            return;
        }
    };

    // 2. Prepare IO streams
    let (pty_reader, pty_writer) = match prepare_pty_stream(pty.fd.as_raw_fd()) {
        Ok(streams) => streams,
        Err(e) => {
            error!("Failed to prepare PTY stream: {}", e);
            return;
        }
    };

    let (mut ws_sink, mut ws_stream) = socket.split();

    // Task 1: Forward PTY STDOUT to WebSocket Binary
    let mut pty_stdout_task = {
        let mut reader = pty_reader;
        tokio::spawn(async move {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf).await {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        if let Err(e) = ws_sink.send(Message::Binary(buf[..n].to_vec())).await {
                            error!("WS send failed: {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        error!("PTY read failed: {}", e);
                        break;
                    }
                }
            }
            info!("Closing PTY readout task");
        })
    };

    // Task 2: Forward WebSocket Input/Commands to PTY STDIN/Ioctl
    let mut ws_input_task = {
        let pty_ref = pty.clone();
        let mut writer = pty_writer;
        tokio::spawn(async move {
            while let Some(Ok(msg)) = ws_stream.next().await {
                match msg {
                    Message::Binary(bin) => {
                        // Raw binary stdin
                        if let Err(e) = writer.write_all(&bin).await {
                            error!("PTY write failed: {}", e);
                            break;
                        }
                    }
                    Message::Text(text) => {
                        // Command handling (JSON)
                        if let Ok(cmd) = serde_json::from_str::<TerminalMessage>(&text) {
                            match cmd {
                                TerminalMessage::Resize { rows, cols } => {
                                    if let Err(e) = pty_ref.resize(rows, cols) {
                                        error!("Resize failed: {}", e);
                                    }
                                }
                                TerminalMessage::Input(input) => {
                                    if let Err(e) = writer.write_all(input.as_bytes()).await {
                                        error!("PTY input write failed: {}", e);
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            info!("Closing WS input forwarding task");
        })
    };

    // Wait for either task to end
    tokio::select! {
        _ = (&mut pty_stdout_task) => { ws_input_task.abort(); }
        _ = (&mut ws_input_task) => { pty_stdout_task.abort(); }
    }
}
