use axum::{
    body::Body,
    extract::Query,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio_util::io::ReaderStream;


fn safe_path(raw: &str) -> Option<PathBuf> {
    let p = PathBuf::from(raw);
    // Reject paths with traversal attempts
    if p.components().any(|c| c.as_os_str() == "..") {
        return None;
    }
    Some(p)
}

#[derive(Serialize)]
pub struct FileEntry {
    name: String,
    path: String,
    is_dir: bool,
    size: u64,
    modified: u64,
    mime: String,
}

#[derive(Deserialize)]
pub struct PathQuery {
    pub path: Option<String>,
}

pub async fn list_files(Query(q): Query<PathQuery>) -> impl IntoResponse {
    let root = q.path.unwrap_or_else(|| std::env::var("HOME").unwrap_or("/".into()));
    let root_path = match safe_path(&root) {
        Some(p) => p,
        None => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "invalid path"}))).into_response(),
    };

    let mut entries: Vec<FileEntry> = Vec::new();

    let mut dir = match fs::read_dir(&root_path).await {
        Ok(d) => d,
        Err(e) => return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    };

    while let Ok(Some(entry)) = dir.next_entry().await {
        let meta = match entry.metadata().await {
            Ok(m) => m,
            Err(_) => continue,
        };
        let name = entry.file_name().to_string_lossy().to_string();
        let path = entry.path().to_string_lossy().to_string();
        let is_dir = meta.is_dir();
        let size = meta.len();
        let modified = meta.modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let mime = if is_dir {
            "inode/directory".into()
        } else {
            mime_guess::from_path(&path).first_or_octet_stream().to_string()
        };

        entries.push(FileEntry { name, path, is_dir, size, modified, mime });
    }

    entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name)));
    Json(entries).into_response()
}

#[derive(Deserialize)]
pub struct MoveReq { pub from: String, pub to: String }

pub async fn move_file(Json(body): Json<MoveReq>) -> impl IntoResponse {
    let src = match safe_path(&body.from) { Some(p) => p, None => return (StatusCode::BAD_REQUEST, "invalid path").into_response() };
    let dst = match safe_path(&body.to) { Some(p) => p, None => return (StatusCode::BAD_REQUEST, "invalid path").into_response() };
    match fs::rename(&src, &dst).await {
        Ok(_) => (StatusCode::OK, "ok").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn copy_file(Json(body): Json<MoveReq>) -> impl IntoResponse {
    let src = match safe_path(&body.from) { Some(p) => p, None => return (StatusCode::BAD_REQUEST, "invalid path").into_response() };
    let dst = match safe_path(&body.to) { Some(p) => p, None => return (StatusCode::BAD_REQUEST, "invalid path").into_response() };
    match fs::copy(&src, &dst).await {
        Ok(_) => (StatusCode::OK, "ok").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
pub struct RenameReq { pub path: String, pub name: String }

pub async fn rename_file(Json(body): Json<RenameReq>) -> impl IntoResponse {
    let src = match safe_path(&body.path) { Some(p) => p, None => return (StatusCode::BAD_REQUEST, "invalid path").into_response() };
    let dst = src.parent().unwrap_or(Path::new("/")).join(&body.name);
    match fs::rename(&src, &dst).await {
        Ok(_) => (StatusCode::OK, "ok").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
pub struct DeleteReq { pub path: String }

pub async fn delete_file(Json(body): Json<DeleteReq>) -> impl IntoResponse {
    let p = match safe_path(&body.path) { Some(p) => p, None => return (StatusCode::BAD_REQUEST, "invalid path").into_response() };
    let result = if p.is_dir() {
        fs::remove_dir_all(&p).await
    } else {
        fs::remove_file(&p).await
    };
    match result {
        Ok(_) => (StatusCode::OK, "ok").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn download_file(Query(q): Query<PathQuery>) -> impl IntoResponse {
    let raw = match q.path { Some(p) => p, None => return (StatusCode::BAD_REQUEST, Body::empty()).into_response() };
    let p = match safe_path(&raw) { Some(p) => p, None => return (StatusCode::BAD_REQUEST, Body::empty()).into_response() };

    let file = match tokio::fs::File::open(&p).await {
        Ok(f) => f,
        Err(_) => return (StatusCode::NOT_FOUND, Body::empty()).into_response(),
    };

    let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    Response::builder()
        .header(header::CONTENT_DISPOSITION, format!("attachment; filename=\"{}\"", name))
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .body(body)
        .unwrap()
}

pub async fn read_file_text(Query(q): Query<PathQuery>) -> impl IntoResponse {
    let raw = match q.path { Some(p) => p, None => return (StatusCode::BAD_REQUEST, "no path".to_string()).into_response() };
    let p = match safe_path(&raw) { Some(p) => p, None => return (StatusCode::BAD_REQUEST, "invalid path".to_string()).into_response() };

    // Limit to 2MB for preview
    let meta = match std::fs::metadata(&p) {
        Ok(m) => m,
        Err(e) => return (StatusCode::NOT_FOUND, e.to_string()).into_response(),
    };
    if meta.len() > 2_000_000 {
        return (StatusCode::PAYLOAD_TOO_LARGE, "File too large for preview".to_string()).into_response();
    }

    match fs::read_to_string(&p).await {
        Ok(content) => (StatusCode::OK, content).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
