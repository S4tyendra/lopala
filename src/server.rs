use axum::{
    routing::{get},
    Router,
    http::{StatusCode, HeaderValue, header},
    response::{IntoResponse},
};
use crate::embed::Assets;
use crate::ws::ws_handler;
use tower_http::cors::CorsLayer;
use tracing::info;

/// Boots the Axum server with WS and Embedded Assets
pub async fn start_server(port: u16) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/_ws", get(ws_handler))
        .fallback(static_asset_handler)
        .layer(CorsLayer::permissive());

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
