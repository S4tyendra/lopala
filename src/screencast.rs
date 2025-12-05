use std::process::Stdio;
use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use tracing::{error, info};
use crate::state::{GlobalState, WsEvent};

/// Spawn a grim capture loop for the given display.
/// Captures directly to stdout (no disk I/O), base64-encodes the JPEG buffer,
/// and broadcasts via WebSocket. Zero temp files.
pub fn spawn_stream(disp: String, state: GlobalState) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        info!("Screen stream started (in-memory): {}", disp);

        loop {
            let result = tokio::process::Command::new("grim")
                .arg("-o").arg(&disp)
                .arg("-t").arg("jpeg")
                .arg("-q").arg("65")
                .arg("-s").arg("0.5")
                .arg("-")  // pipe to stdout — no disk write
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await;

            match result {
                Ok(out) if out.status.success() => {
                    // Encode JPEG bytes to base64 and broadcast immediately
                    let b64 = B64.encode(&out.stdout);
                    let _ = state.tx.send(WsEvent::ScreenFrame {
                        display: disp.clone(),
                        data: b64,
                    });
                }
                Ok(out) => {
                    let err = String::from_utf8_lossy(&out.stderr).to_string();
                    error!("grim frame failed for {}: {}", disp, err);
                }
                Err(e) => {
                    error!("grim spawn failed: {}", e);
                    break;
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        info!("Screen stream stopped: {}", disp);
    })
}
