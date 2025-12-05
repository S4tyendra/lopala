use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex, mpsc};
use tokio::task::AbortHandle;
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
    pub version: u64,
    pub sender: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ScreenshotStateSync {
    pub display: Option<String>,
    pub opened_image: Option<String>,
    pub scroll_top: f64,
    pub version: u64,
    pub sender: String,
}

// ─── System Vitals ────────────────────────────────────────────────────────────

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct SystemVitals {
    pub cpu_percent: f64,
    pub ram_used_mb: u64,
    pub ram_total_mb: u64,
    pub swap_used_mb: u64,
    pub swap_total_mb: u64,
    pub disk_read_bytes: u64,
    pub disk_write_bytes: u64,
    /// Per-core usage percentages
    pub cpu_per_core: Vec<f64>,
}

#[allow(dead_code)]
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu: f64,
    pub mem_mb: u64,
    pub user: String,
    pub command: String,
}

// ─── Editor Sync (CRDT/OT-style) ─────────────────────────────────────────────

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct EditorOp {
    /// The file being edited
    pub file_path: String,
    /// Who is making the edit
    pub user_id: String,
    /// Operation type: "insert", "delete", "replace", "cursor", "selection"
    pub op: String,
    /// Position in the document (character offset)
    pub pos: usize,
    /// For delete/replace: number of characters to remove
    pub del_len: usize,
    /// For insert/replace: text to insert
    pub text: String,
    /// Monotonically increasing version per file — used for conflict resolution
    pub version: u64,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct EditorCursor {
    pub file_path: String,
    pub user_id: String,
    pub user_name: String,
    pub user_color: String,
    /// Primary cursor position (char offset)
    pub pos: usize,
    /// Selection anchor (char offset), same as pos if no selection
    pub anchor: usize,
}

// ─── Taskmanager Sync ─────────────────────────────────────────────────────────

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TaskmanagerStateSync {
    /// "sort_col", "sort_dir", "filter", etc.
    pub sort_column: String,
    pub sort_ascending: bool,
    pub filter: String,
    pub version: u64,
    pub sender: String,
}

// ─── App State ────────────────────────────────────────────────────────────────

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
    // display -> (watcher_count, abort_handle)
    pub stream_tasks: Arc<Mutex<HashMap<String, (u32, AbortHandle)>>>,
    // Per-file editor document versions (file_path -> latest version)
    pub editor_versions: Arc<Mutex<HashMap<String, u64>>>,
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
            stream_tasks: Arc::new(Mutex::new(HashMap::new())),
            editor_versions: Arc::new(Mutex::new(HashMap::new())),
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
    ScreenshotSync { state: ScreenshotStateSync },
    // Screen view streaming — binary frame as base64
    StartStream { display: String },
    StopStream { display: String },
    ScreenFrame { display: String, data: String },
    // System vitals (broadcast every 2s)
    SystemVitals { vitals: SystemVitals },
    // Editor collaboration
    EditorOp { op: EditorOp },
    EditorCursor { cursor: EditorCursor },
    // Taskmanager sync
    TaskmanagerSync { state: TaskmanagerStateSync },
}
