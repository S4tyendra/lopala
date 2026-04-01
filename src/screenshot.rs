use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{error, info};

#[derive(Serialize)]
pub struct DisplayInfo {
    pub name: String,
    pub description: String,
}

pub async fn get_displays() -> impl IntoResponse {
    let mut displays = Vec::new();

    // 1. Try hyprctl
    if let Ok(out) = Command::new("hyprctl").arg("monitors").arg("-j").output() {
        if out.status.success() {
            if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&out.stdout) {
                if let Some(arr) = json.as_array() {
                    for m in arr {
                        let name = m["name"].as_str().unwrap_or("").to_string();
                        let desc = m["description"].as_str().unwrap_or(&name).to_string();
                        displays.push(DisplayInfo {
                            name,
                            description: desc,
                        });
                    }
                }
            }
        }
    }

    // 2. Try swaymsg (if hyprctl failed)
    if displays.is_empty() {
        if let Ok(out) = Command::new("swaymsg")
            .arg("-t")
            .arg("get_outputs")
            .arg("-r")
            .output()
        {
            if out.status.success() {
                if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&out.stdout) {
                    if let Some(arr) = json.as_array() {
                        for m in arr {
                            let name = m["name"].as_str().unwrap_or("").to_string();
                            let desc = m["make"].as_str().unwrap_or(&name).to_string();
                            displays.push(DisplayInfo {
                                name,
                                description: desc,
                            });
                        }
                    }
                }
            }
        }
    }

    // 3. Fallback to /sys/class/drm
    if displays.is_empty() {
        if let Ok(entries) = std::fs::read_dir("/sys/class/drm") {
            for entry in entries.flatten() {
                let path = entry.path();
                let status_path = path.join("status");
                if let Ok(status) = std::fs::read_to_string(status_path) {
                    if status.trim() == "connected" {
                        let dir_name = path
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into_owned();
                        let mut parts = dir_name.splitn(2, '-');
                        parts.next(); // Skip "card0" etc
                        if let Some(name) = parts.next() {
                            displays.push(DisplayInfo {
                                name: name.to_string(),
                                description: format!("Generic Display ({})", name),
                            });
                        }
                    }
                }
            }
        }
    }

    (StatusCode::OK, Json(displays)).into_response()
}

#[derive(Deserialize)]
pub struct TakeScreenshotReq {
    pub display: String,
}

pub async fn take_screenshot(Json(req): Json<TakeScreenshotReq>) -> impl IntoResponse {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let dir = format!("/tmp/latch/screenshots/{}", req.display);
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
