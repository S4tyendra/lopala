use axum::{
    routing::{get, post},
    Router,
    http::{StatusCode, HeaderValue, header},
    response::{IntoResponse},
    extract::FromRef,
};
use crate::embed::Assets;
use crate::ws::ws_handler;
use crate::state::GlobalState;
use crate::files::{list_files, move_file, copy_file, rename_file, delete_file, download_file, read_file_text, write_file};
use crate::screenshot::{get_displays, take_screenshot};
use crate::search::search_files;
use crate::upload::{upload_init, upload_chunk, upload_status, UploadSessions};
use crate::system::{list_processes, kill_process};
use tower_http::cors::CorsLayer;
use tracing::info;

/// Combined app state — both global session state and upload session map.
/// Axum's `FromRef` lets individual handlers extract just the sub-state they need.
#[derive(Clone)]
pub struct AppState {
    pub global: GlobalState,
    pub uploads: UploadSessions,
}

impl FromRef<AppState> for GlobalState {
    fn from_ref(s: &AppState) -> Self { s.global.clone() }
}

impl FromRef<AppState> for UploadSessions {
    fn from_ref(s: &AppState) -> Self { s.uploads.clone() }
}

/// Boots the Axum server with WS and Embedded Assets
pub async fn start_server(port: u16, state: GlobalState, upload_sessions: UploadSessions) -> anyhow::Result<()> {
    let shared = AppState { global: state, uploads: upload_sessions };

    let api = Router::new()
        // ── File Management ───────────────────────────────────────────────────
        .route("/files", get(list_files))
        .route("/files/move", post(move_file))
        .route("/files/copy", post(copy_file))
        .route("/files/rename", post(rename_file))
        .route("/files/delete", post(delete_file))
        .route("/files/download", get(download_file))
        .route("/files/read", get(read_file_text))
        .route("/files/write", post(write_file))
        // ── Chunked Upload API ────────────────────────────────────────────────
        .route("/files/upload/init", post(upload_init))
        .route("/files/upload/chunk", post(upload_chunk))
        .route("/files/upload/status", get(upload_status))
        // ── System / Task Manager ─────────────────────────────────────────────
        .route("/system/ps", get(list_processes))
        .route("/system/kill", post(kill_process))
        // ── Misc ──────────────────────────────────────────────────────────────
        .route("/displays", get(get_displays))
        .route("/screenshots/take", post(take_screenshot))
        .route("/search", get(search_files));

    let app = Router::new()
        .route("/_ws", get(ws_handler))
        .nest("/api", api)
        .fallback(static_asset_handler)
        .layer(CorsLayer::permissive())
        .with_state(shared);

    let listener = tokio::net::TcpListener::bind(&format!("0.0.0.0:{}", port)).await?;
    info!("HTTP Server listening on: {}", port);
    axum::serve(listener, app).await?;
    Ok(())
}

/// Fallback for Axum to serve embedded assets from `ui/dist`
async fn static_asset_handler(uri: axum::http::Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');
    let file_path = if path.is_empty() { "index.html" } else { path };

    match Assets::get(file_path) {
        Some(content) => {
            let mime = mime_guess::from_path(file_path).first_or_octet_stream();
            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, HeaderValue::from_str(mime.as_ref()).unwrap())],
                content.data.to_vec(),
            ).into_response()
        }
        None => {
            if let Some(index) = Assets::get("index.html") {
                (
                    StatusCode::OK,
                    [(header::CONTENT_TYPE, HeaderValue::from_static("text/html"))],
                    index.data.to_vec(),
                ).into_response()
            } else {
                (StatusCode::NOT_FOUND, "Not Found").into_response()
            }
        }
    }
}
