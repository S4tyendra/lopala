use axum::{response::IntoResponse, Json, http::StatusCode};
use serde::{Deserialize, Serialize};
use std::process::Command;
use tracing::{error, info};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize)]
pub struct DisplayInfo {
    pub name: String,
    pub description: String,
}

pub async fn get_displays() -> impl IntoResponse {
    // Run hyprctl monitors -j
    let output = Command::new("hyprctl")
        .arg("monitors")
        .arg("-j")
        .output();
        
    match output {
        Ok(out) if out.status.success() => {
            if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&out.stdout) {
                if let Some(arr) = json.as_array() {
                    let mut displays = Vec::new();
                    for m in arr {
                        let name = m["name"].as_str().unwrap_or("").to_string();
                        let desc = m["description"].as_str().unwrap_or(&name).to_string();
                        displays.push(DisplayInfo { name, description: desc });
                    }
                    return (StatusCode::OK, Json(displays)).into_response();
                }
            }
        }
        _ => {}
    }

    // Fallback or error
    let dummy = vec![DisplayInfo { name: "eDP-1".into(), description: "Built-in Display".into() }];
    (StatusCode::OK, Json(dummy)).into_response()
}

#[derive(Deserialize)]
pub struct TakeScreenshotReq {
    pub display: String,
}

pub async fn take_screenshot(Json(req): Json<TakeScreenshotReq>) -> impl IntoResponse {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let dir = format!("/tmp/lopala/screenshots/{}", req.display);
    std::fs::create_dir_all(&dir).unwrap_or_default();
    
    let path = format!("{}/{}.png", dir, now);
    let output = Command::new("grim")
        .arg("-o")
        .arg(&req.display)
        .arg("-t")
        .arg("png")
        .arg(&path)
        .output();
        
    match output {
        Ok(out) if out.status.success() => {
            info!("Screenshot saved to {}", path);
            (StatusCode::OK, path).into_response()
        }
        Ok(out) => {
            let err = String::from_utf8_lossy(&out.stderr);
            error!("grim failed: {}", err);
            (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response()
        }
        Err(e) => {
            error!("Failed to execute grim: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
        }
    }
}
