use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{error, info};
use crate::state::{GlobalState, WsEvent};

/// Spawn a grim capture loop for the given display.
/// Called when watcher count goes from 0 → 1.
/// Returns a JoinHandle whose AbortHandle is stored in GlobalState::stream_tasks.
pub fn spawn_stream(disp: String, state: GlobalState) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        info!("Screen stream started for display: {}", disp);

        // Dedicated dir so it doesn't pollute the screenshot gallery
        let dir = format!("/tmp/lopala/streams/{}", disp);
        std::fs::create_dir_all(&dir).unwrap_or_default();

        // Rolling buffer: keep last N frames on disk, delete older ones
        const KEEP_FRAMES: usize = 30;
        let mut frame_ring: std::collections::VecDeque<String> = std::collections::VecDeque::new();

        loop {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis();

            let path = format!("{}/{}.jpg", dir, now);

            // grim with JPEG + scale-down for minimum payload at 10fps
            let result = Command::new("grim")
                .arg("-o").arg(&disp)
                .arg("-t").arg("jpeg")
                .arg("-q").arg("70")
                .arg("-s").arg("0.6")
                .arg(&path)
                .output();

            match result {
                Ok(out) if out.status.success() => {
                    let _ = state.tx.send(WsEvent::ScreenFrame {
                        display: disp.clone(),
                        path: path.clone(),
                    });

                    frame_ring.push_back(path);
                    if frame_ring.len() > KEEP_FRAMES {
                        if let Some(old) = frame_ring.pop_front() {
                            let _ = std::fs::remove_file(&old);
                        }
                    }
                }
                Ok(out) => {
                    let err = String::from_utf8_lossy(&out.stderr).to_string();
                    error!("grim stream frame failed for {}: {}", disp, err);
                }
                Err(e) => {
                    error!("grim not found or failed to spawn: {}", e);
                    break;
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        info!("Screen stream stopped for display: {}", disp);
    })
}
