use axum::{
    routing::{get, post},
    Router,
    http::{StatusCode, HeaderValue, header},
    response::{IntoResponse},
};
use crate::embed::Assets;
use crate::ws::ws_handler;
use crate::state::GlobalState;
use crate::files::{list_files, move_file, copy_file, rename_file, delete_file, download_file, read_file_text};
use tower_http::cors::CorsLayer;
use tracing::info;

/// Boots the Axum server with WS and Embedded Assets
pub async fn start_server(port: u16, state: GlobalState) -> anyhow::Result<()> {
    let api = Router::new()
        .route("/files", get(list_files))
        .route("/files/move", post(move_file))
        .route("/files/copy", post(copy_file))
        .route("/files/rename", post(rename_file))
        .route("/files/delete", post(delete_file))
        .route("/files/download", get(download_file))
        .route("/files/read", get(read_file_text));

    let app = Router::new()
        .route("/_ws", get(ws_handler))
        .nest("/api", api)
        .fallback(static_asset_handler)
        .layer(CorsLayer::permissive())
        .with_state(state);


    let listener = tokio::net::TcpListener::bind(&format!("0.0.0.0:{}", port)).await?;
    info!("HTTP Server listening on: {}", port);
    axum::serve(listener, app).await?;
    Ok(())
}

/// Fallback for Axum to serve embedded assets from `ui/dist`
async fn static_asset_handler(uri: axum::http::Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');
    
    // Default to index.html for SPA-like Astro output
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
            // Check if we should fallback to index.html for routing
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
