use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}},
    response::IntoResponse,
};
use futures_util::{sink::SinkExt, stream::StreamExt};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;
use tracing::{error, info};
use std::os::unix::io::AsRawFd;

use crate::pty::Pty;
use crate::io::{prepare_pty_stream};
use crate::state::{GlobalState, WsEvent, PtyHandle};

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    axum::extract::State(state): axum::extract::State<GlobalState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: GlobalState) {
    info!("New client connected to multi-user environment");

    // Send initial sync state
    {
        let data = state.data.lock().await;
        if let Ok(json) = serde_json::to_string(&WsEvent::SyncState { state: data.clone() }) {
            let _ = socket.send(Message::Text(json)).await;
        }
    }

    let (mut ws_sink, mut ws_stream) = socket.split();
    let mut rx = state.tx.subscribe();

    // Broadcast dispatcher
    let mut broadcast_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&msg) {
                if ws_sink.send(Message::Text(json)).await.is_err() {
                    break;
                }
            }
        }
    });

    // Ingress receiver
    let state_clone = state.clone();
    let mut ingress_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_stream.next().await {
            match msg {
                Message::Text(text) => {
                    if let Ok(event) = serde_json::from_str::<WsEvent>(&text) {
                        handle_client_event(event, state_clone.clone()).await;
                    }
                }
                _ => {}
            }
        }
    });

    tokio::select! {
        _ = (&mut broadcast_task) => ingress_task.abort(),
        _ = (&mut ingress_task) => broadcast_task.abort(),
    }
}

async fn handle_client_event(event: WsEvent, state: GlobalState) {
    match event {
        WsEvent::UserJoined { mut user } => {
            let mut data = state.data.lock().await;
            // Assign color based on join order
            let color_idx = (data.user_count as usize) % crate::state::USER_COLORS.len();
            user.color = crate::state::USER_COLORS[color_idx].to_string();
            data.user_count += 1;
            data.users.insert(user.id.clone(), user.clone());
            let _ = state.tx.send(WsEvent::UserJoined { user });
        }
        WsEvent::UserLeft { id } => {
            state.data.lock().await.users.remove(&id);
            let _ = state.tx.send(WsEvent::UserLeft { id });
        }
        WsEvent::CursorMove { id, x, y, workspace } => {
            let mut data = state.data.lock().await;
            if let Some(user) = data.users.get_mut(&id) {
                user.x = x;
                user.y = y;
                user.workspace = workspace;
            }
            let _ = state.tx.send(WsEvent::CursorMove { id, x, y, workspace });
        }
        WsEvent::SpawnWindow { window } => {
            state.data.lock().await.windows.insert(window.id.clone(), window.clone());
            let _ = state.tx.send(WsEvent::SpawnWindow { window: window.clone() });

            if window.app == "terminal" {
                spawn_pty(window.id.clone(), state.clone()).await;
            }
        }
        WsEvent::UpdateWindow { window } => {
            state.data.lock().await.windows.insert(window.id.clone(), window.clone());
            let _ = state.tx.send(WsEvent::UpdateWindow { window });
        }
        WsEvent::CloseWindow { id } => {
            state.data.lock().await.windows.remove(&id);
            let _ = state.tx.send(WsEvent::CloseWindow { id: id.clone() });
            state.ptys.lock().await.remove(&id); // Drops the sender, dropping the PTY
        }
        WsEvent::PtyIn { id, data } => {
            if let Some(handle) = state.ptys.lock().await.get(&id) {
                let _ = handle.tx.send(data).await;
            }
        }
        WsEvent::PtyResize { id, rows, cols } => {
            if let Some(handle) = state.ptys.lock().await.get(&id) {
                let _ = handle.resize_tx.send((rows, cols)).await;
            }
        }
        WsEvent::SendChat { channel, content, user_name } => {
            let msg = crate::state::ChatMessage {
                id: format!("{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_micros()),
                channel,
                user_name,
                content,
                timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
            };
            state.data.lock().await.chats.push(msg.clone());
            let _ = state.tx.send(WsEvent::ChatMsg { msg });
        }
        WsEvent::CreateChannel { name, created_by } => {
            let id = name.to_lowercase().replace(' ', "-");
            let ch = crate::state::Channel {
                id: id.clone(),
                name: format!("# {}", name),
                created_by: created_by.clone(),
            };
            let mut data = state.data.lock().await;
            if !data.channels.iter().any(|c| c.id == id) {
                data.channels.push(ch.clone());
            }
            let _ = state.tx.send(WsEvent::ChannelCreated { channel: ch });
        }
        WsEvent::CanvasDraw { stroke } => {
            let mut data = state.data.lock().await;
            data.canvas_strokes
                .entry(stroke.canvas_id.clone())
                .or_insert_with(Vec::new)
                .push(stroke.clone());
            let _ = state.tx.send(WsEvent::CanvasDraw { stroke });
        }
        WsEvent::CanvasClear { canvas_id } => {
            state.data.lock().await.canvas_strokes.remove(&canvas_id);
            let _ = state.tx.send(WsEvent::CanvasClear { canvas_id });
        }
        WsEvent::SetWorkspaceCount { count } => {
            state.data.lock().await.workspace_count = count;
            let _ = state.tx.send(WsEvent::SetWorkspaceCount { count });
        }


        WsEvent::RequestHistory { id } => {
            // We just dispatch history back through the broadcast. Clients filtering via 'id'.
            let hist = state.pty_history.lock().await;
            if let Some(data) = hist.get(&id) {
                let _ = state.tx.send(WsEvent::HistoryData { id, data: data.clone() });
            }
        }
        _ => {}
    }
}

async fn spawn_pty(window_id: String, state: GlobalState) {
    let pty = match Pty::spawn() {
        Ok(p) => Arc::new(p),
        Err(e) => { error!("PTY spawn failed: {}", e); return; }
    };
    let (pty_reader, pty_writer) = match prepare_pty_stream(pty.fd.as_raw_fd()) {
        Ok(s) => s,
        Err(_) => return,
    };

    let (tx, mut rx) = mpsc::channel::<String>(100);
    let (resize_tx, mut resize_rx) = mpsc::channel::<(u16, u16)>(10);
    
    state.ptys.lock().await.insert(window_id.clone(), PtyHandle { tx, resize_tx });

    let state_clone = state.clone();
    let win_id = window_id.clone();
    let pty_ref = pty.clone();

    tokio::spawn(async move {
        while let Some((rows, cols)) = resize_rx.recv().await {
            let _ = pty_ref.resize(rows, cols);
        }
    });

    let mut writer = pty_writer;
    tokio::spawn(async move {
        while let Some(data) = rx.recv().await {
            let _ = writer.write_all(data.as_bytes()).await;
        }
    });

    // PWD Poller
    let pid = pty.pid.as_raw();
    let title_state = state.clone();
    let title_id = window_id.clone();
    tokio::spawn(async move {
        let host = std::env::var("HOSTNAME").unwrap_or_else(|_| "lopala".into());
        let user = std::env::var("USER").unwrap_or_else(|_| "user".into());
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            if let Ok(path) = std::fs::read_link(format!("/proc/{}/cwd", pid)) {
                let p = path.to_string_lossy().to_string();
                let short_p = if p.len() > 20 {
                    format!("...{}", &p[p.len()-17..])
                } else { p };
                let title = format!("{}@{} {}", user, host, short_p);
                
                // mutate state silently and push update
                if let Some(window) = title_state.data.lock().await.windows.get_mut(&title_id) {
                    if window.title != title {
                        window.title = title.clone();
                        let _ = title_state.tx.send(WsEvent::UpdateTitle { id: title_id.clone(), title });
                    }
                }
            } else {
                break; // Process dead
            }
        }
    });

    let mut reader = pty_reader;
    tokio::spawn(async move {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    let text = String::from_utf8_lossy(&buf[..n]).to_string();
                    {
                        let mut hist = state_clone.pty_history.lock().await;
                        let s = hist.entry(win_id.clone()).or_insert_with(String::new);
                        s.push_str(&text);
                        if s.len() > 50000 {
                            *s = s[s.len()-50000..].to_string();
                        }
                    }
                    let _ = state_clone.tx.send(WsEvent::PtyOut { id: win_id.clone(), data: text });
                }
                Err(_) => break,
            }
        }
        state_clone.ptys.lock().await.remove(&win_id);
    });
}
