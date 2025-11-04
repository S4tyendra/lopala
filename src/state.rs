use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex, mpsc};
use serde::{Serialize, Deserialize};

// Shared color palette — assigned round-robin per user join order
pub const USER_COLORS: &[&str] = &[
    "#60a5fa", "#34d399", "#fb923c", "#f472b6",
    "#a78bfa", "#facc15", "#22d3ee", "#f87171",

    "#93c5fd", "#6ee7b7", "#fdba74", "#f9a8d4",
    "#c4b5fd", "#fde047", "#67e8f9", "#fca5a5",

    "#bae6fd", "#bbf7d0", "#fed7aa", "#fbcfe8",
    "#ddd6fe", "#fef08a", "#a5f3fc", "#fecaca",
];

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct User {
    pub id: String,
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub workspace: u32,
    pub color: String,
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
pub struct Channel {
    pub id: String,
    pub name: String,
    pub created_by: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct CanvasStroke {
    pub canvas_id: String,
    pub user_id: String,
    pub color: String,
    pub size: f64,
    pub points: Vec<[f64; 2]>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct FileStateSync {
    pub path: String,
    pub selected: Vec<String>,
    pub scroll_top: f64,
    pub renaming: Option<(String, String)>,
    pub clipboard_op: Option<String>,
    pub clipboard_paths: Vec<String>,
    pub preview_path: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct AppStateData {
    pub users: HashMap<String, User>,
    pub windows: HashMap<String, Window>,
    pub chats: Vec<ChatMessage>,
    pub channels: Vec<Channel>,
    pub workspace_count: u32,
    pub canvas_strokes: HashMap<String, Vec<CanvasStroke>>, // canvas_id -> strokes
    pub user_count: u32, // for color assignment
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
                channels: vec![
                    Channel { id: "#".into(), name: "#".into(), created_by: "system".into() },
                ],
                workspace_count: 4,
                canvas_strokes: HashMap::new(),
                user_count: 0,
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
    CanvasDraw { stroke: CanvasStroke },
    CanvasClear { canvas_id: String },
    // Live line segment — broadcast only, NOT stored in canvas_strokes history
    CanvasLiveLine { canvas_id: String, user_id: String, color: String, size: f64, from: [f64; 2], to: [f64; 2] },
    CreateChannel { name: String, created_by: String },
    ChannelCreated { channel: Channel },
    FileSync { state: FileStateSync },
}
