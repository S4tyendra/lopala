use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    os::unix::fs::FileExt,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

// ─────────────────────────────────────────
//  Session type
// ─────────────────────────────────────────

#[derive(Debug)]
pub struct UploadSession {
    /// Final, pre-allocated destination file path.
    pub dest_path: PathBuf,
    pub file_name: String,
    pub total_size: u64,
    pub total_parts: u32,
    /// true = this part index has been written & verified.
    pub received_parts: Vec<bool>,
    pub last_activity: Instant,
    pub complete: bool,
}

// ─────────────────────────────────────────
//  Shared state (injected into Axum state)
// ─────────────────────────────────────────

pub type UploadSessions = Arc<RwLock<HashMap<String, UploadSession>>>;

/// Spawn a background task that reaps sessions idle for > 3 minutes.
pub async fn start_cleanup_task(sessions: UploadSessions) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            let now = Instant::now();
            let mut map = sessions.write().await;
            let before = map.len();
            map.retain(|id, s| {
                let idle = now.duration_since(s.last_activity);
                if idle > Duration::from_secs(180) && !s.complete {
                    // Remove the ghost-allocated file from disk.
                    let _ = std::fs::remove_file(&s.dest_path);
                    warn!("Upload session {} expired after {:.0?} idle — file removed", id, idle);
                    false
                } else {
                    true
                }
            });
            let removed = before - map.len();
            if removed > 0 {
                info!("Upload GC: removed {} stale sessions", removed);
            }
        }
    });
}

// ─────────────────────────────────────────
//  Request / Response types
// ─────────────────────────────────────────

const CHUNK_SIZE: u64 = 10 * 1024 * 1024; // 10 MiB

#[derive(Deserialize)]
pub struct InitReq {
    /// Absolute destination path requested by the client (validated server-side).
    pub dest_path: String,
    pub file_name: String,
    pub total_size: u64,
}

#[derive(Serialize)]
pub struct InitResp {
    pub session_id: String,
    pub chunk_size: u64,
    pub total_parts: u32,
}

#[derive(Deserialize)]
pub struct UploadQuery {
    pub session_id: String,
    /// 0-indexed part number.
    pub part: u32,
    /// hex-encoded SHA-256 of the raw chunk bytes.
    pub hash: String,
    /// present and = "true" to trigger completion validation.
    pub process: Option<String>,
}

// ─────────────────────────────────────────
//  Handlers
// ─────────────────────────────────────────

/// POST /files/upload/init
///
/// Creates the destination file, pre-allocates it with `set_len`, and registers
/// the session.  Returns a session_id for subsequent part uploads.
pub async fn upload_init(
    State(sessions): State<UploadSessions>,
    Json(req): Json<InitReq>,
) -> impl IntoResponse {
    // Basic path safety.
    if req.dest_path.contains("..") {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "path traversal rejected"}))).into_response();
    }
    if req.total_size == 0 {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "total_size must be > 0"}))).into_response();
    }

    let dest = PathBuf::from(&req.dest_path);

    // Ensure parent directory exists.
    if let Some(parent) = dest.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response();
        }
    }

    // Create & pre-allocate the file using set_len — O(1) on ext4/btrfs/xfs.
    let file = match std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&dest)
    {
        Ok(f) => f,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    };

    if let Err(e) = file.set_len(req.total_size) {
        let _ = std::fs::remove_file(&dest);
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("set_len failed: {}", e)}))).into_response();
    }
    drop(file);

    let total_parts = ((req.total_size + CHUNK_SIZE - 1) / CHUNK_SIZE) as u32;
    let session_id = Uuid::new_v4().to_string();

    let session = UploadSession {
        dest_path: dest,
        file_name: req.file_name.clone(),
        total_size: req.total_size,
        total_parts,
        received_parts: vec![false; total_parts as usize],
        last_activity: Instant::now(),
        complete: false,
    };

    sessions.write().await.insert(session_id.clone(), session);
    info!("Upload init: session={} file={} size={} parts={}", session_id, req.file_name, req.total_size, total_parts);

    (
        StatusCode::OK,
        Json(InitResp {
            session_id,
            chunk_size: CHUNK_SIZE,
            total_parts,
        }),
    ).into_response()
}

/// POST /files/upload/{session_id}?part=N&hash=sha256hex
///
/// Receives a raw chunk body, verifies SHA-256, then writes it directly to the
/// pre-allocated file at the correct byte offset using `write_at`.
/// No temp files, no combining step.
pub async fn upload_chunk(
    State(sessions): State<UploadSessions>,
    Query(q): Query<UploadQuery>,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    // ── Route to complete handler if ?process=true ────────────────────────────
    if q.process.as_deref() == Some("true") {
        return upload_complete(sessions, q.session_id).await;
    }

    // ── Fetch session (read lock) ─────────────────────────────────────────────
    let (dest_path, total_parts, total_size) = {
        let map = sessions.read().await;
        let s = match map.get(&q.session_id) {
            Some(s) => s,
            None => return (StatusCode::NOT_FOUND, "session not found").into_response(),
        };
        if s.complete {
            return (StatusCode::CONFLICT, "session already complete").into_response();
        }
        (s.dest_path.clone(), s.total_parts, s.total_size)
    };

    if q.part >= total_parts {
        return (StatusCode::BAD_REQUEST, "part index out of range").into_response();
    }

    // ── Verify SHA-256 ───────────────────────────────────────────────────────
    let mut hasher = Sha256::new();
    hasher.update(&body);
    let computed = hex::encode(hasher.finalize());
    if computed != q.hash.to_lowercase() {
        warn!("SHA-256 mismatch for session={} part={}: expected={} computed={}", q.session_id, q.part, q.hash, computed);
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
            "error": "checksum mismatch",
            "expected": q.hash,
            "computed": computed,
        }))).into_response();
    }

    // ── Validate chunk size ──────────────────────────────────────────────────
    let offset = q.part as u64 * CHUNK_SIZE;
    let is_last = q.part == total_parts - 1;
    let expected_len = if is_last {
        total_size - offset
    } else {
        CHUNK_SIZE
    };

    if body.len() as u64 != expected_len {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
            "error": "unexpected chunk length",
            "expected": expected_len,
            "received": body.len(),
        }))).into_response();
    }

    // ── Write directly at offset — zero copy I/O ─────────────────────────────
    // Opens the already-allocated file and seeks to the exact byte position.
    // `write_at` uses pwrite(2) under the hood — thread-safe, no seek/write race.
    let write_result = tokio::task::spawn_blocking({
        let dest = dest_path.clone();
        let data = body.to_vec();
        move || {
            let file = std::fs::OpenOptions::new().write(true).open(&dest)?;
            file.write_at(&data, offset)?;
            Ok::<(), std::io::Error>(())
        }
    }).await;

    match write_result {
        Ok(Ok(())) => {}
        Ok(Err(e)) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }

    // ── Mark part received (write lock) ──────────────────────────────────────
    {
        let mut map = sessions.write().await;
        if let Some(s) = map.get_mut(&q.session_id) {
            s.received_parts[q.part as usize] = true;
            s.last_activity = Instant::now();
        }
    }

    info!("Upload chunk ok: session={} part={}/{}", q.session_id, q.part + 1, total_parts);
    (StatusCode::OK, Json(serde_json::json!({
        "ok": true,
        "part": q.part,
    }))).into_response()
}

/// POST /files/upload/{session_id}?process=true
///
/// Validates all expected parts have arrived, marks session complete.
/// The file is already fully written — nothing to combine.
async fn upload_complete(sessions: UploadSessions, session_id: String) -> axum::response::Response {
    let mut map = sessions.write().await;
    let s = match map.get_mut(&session_id) {
        Some(s) => s,
        None => return (StatusCode::NOT_FOUND, "session not found").into_response(),
    };

    if s.complete {
        return (StatusCode::CONFLICT, "already complete").into_response();
    }

    let missing: Vec<u32> = s.received_parts.iter().enumerate()
        .filter_map(|(i, &ok)| if !ok { Some(i as u32) } else { None })
        .collect();

    if !missing.is_empty() {
        return (StatusCode::CONFLICT, Json(serde_json::json!({
            "error": "missing parts",
            "missing": missing,
        }))).into_response();
    }

    s.complete = true;
    s.last_activity = Instant::now();
    let dest = s.dest_path.to_string_lossy().to_string();
    let name = s.file_name.clone();
    info!("Upload complete: session={} file={} dest={}", session_id, name, dest);

    (StatusCode::OK, Json(serde_json::json!({
        "ok": true,
        "file": name,
        "dest": dest,
    }))).into_response()
}

/// GET /files/upload/status?session_id=...
///
/// Returns upload progress: which parts are done, for resumability.
#[derive(Deserialize)]
pub struct StatusQuery { pub session_id: String }

#[derive(Serialize)]
pub struct UploadStatus {
    pub session_id: String,
    pub file_name: String,
    pub total_parts: u32,
    pub received_count: u32,
    pub complete: bool,
    /// Which part indices are still missing (for resume support).
    pub missing_parts: Vec<u32>,
}

pub async fn upload_status(
    State(sessions): State<UploadSessions>,
    Query(q): Query<StatusQuery>,
) -> impl IntoResponse {
    let map = sessions.read().await;
    let s = match map.get(&q.session_id) {
        Some(s) => s,
        None => return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "session not found"}))).into_response(),
    };
    let missing: Vec<u32> = s.received_parts.iter().enumerate()
        .filter_map(|(i, &ok)| if !ok { Some(i as u32) } else { None })
        .collect();
    let received_count = (s.total_parts as usize - missing.len()) as u32;

    Json(UploadStatus {
        session_id: q.session_id.clone(),
        file_name: s.file_name.clone(),
        total_parts: s.total_parts,
        received_count,
        complete: s.complete,
        missing_parts: missing,
    }).into_response()
}
