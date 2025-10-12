use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex, mpsc};
use serde::{Serialize, Deserialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct User {
    pub id: String,
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub workspace: u32,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Window {
    pub id: String,
    pub app: String,
    pub workspace: u32,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub z: u32,
    pub minimized: bool,
    pub maximized: bool,
    pub title: String,
    pub channel: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ChatMessage {
    pub id: String,
    pub channel: String,
    pub user_name: String,
    pub content: String,
    pub timestamp: u64,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct AppStateData {
    pub users: HashMap<String, User>,
    pub windows: HashMap<String, Window>,
    pub chats: Vec<ChatMessage>,
    pub workspace_count: u32,
}

pub struct PtyHandle {
    pub tx: mpsc::Sender<String>,
    pub resize_tx: mpsc::Sender<(u16, u16)>,
}

#[derive(Clone)]
pub struct GlobalState {
    pub data: Arc<Mutex<AppStateData>>,
    pub tx: broadcast::Sender<WsEvent>,
    pub pty_history: Arc<Mutex<HashMap<String, String>>>,
    pub ptys: Arc<Mutex<HashMap<String, PtyHandle>>>,
}

impl GlobalState {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(1024);
        GlobalState {
            data: Arc::new(Mutex::new(AppStateData {
                users: HashMap::new(),
                windows: HashMap::new(),
                chats: Vec::new(),
                workspace_count: 4,
            })),
            tx,
            pty_history: Arc::new(Mutex::new(HashMap::new())),
            ptys: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum WsEvent {
    SyncState { state: AppStateData },
    CursorMove { id: String, x: f64, y: f64, workspace: u32 },
    UserJoined { user: User },
    UserLeft { id: String },
    SpawnWindow { window: Window },
    UpdateWindow { window: Window },
    CloseWindow { id: String },
    PtyOut { id: String, data: String },
    PtyIn { id: String, data: String },
    PtyResize { id: String, rows: u16, cols: u16 },
    SendChat { channel: String, content: String, user_name: String },
    ChatMsg { msg: ChatMessage },
    SetWorkspaceCount { count: u32 },
    RequestHistory { id: String },
    HistoryData { id: String, data: String },
    UpdateTitle { id: String, title: String },
}
