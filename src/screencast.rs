use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{error, info};
use crate::state::{GlobalState, WsEvent};

fn now_ms() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis()
}

/// Spawn a grim capture loop for the given display.
/// Frames are written to /tmp/lopala/live/{display}/ and frames older than
/// 10 seconds are pruned on every iteration.
pub fn spawn_stream(disp: String, state: GlobalState) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let dir = format!("/tmp/lopala/live/{}", disp);
        std::fs::create_dir_all(&dir).unwrap_or_default();
        info!("Screen stream started: {}", disp);

        // Track written paths for time-based pruning
        let mut frame_ring: std::collections::VecDeque<(u128, String)> = std::collections::VecDeque::new();

        loop {
            let ts = now_ms();
            let path = format!("{}/{}.jpg", dir, ts);

            let result = Command::new("grim")
                .arg("-o").arg(&disp)
                .arg("-t").arg("jpeg")
                .arg("-q").arg("72")
                .arg("-s").arg("0.6")
                .arg(&path)
                .output();

            match result {
                Ok(out) if out.status.success() => {
                    let _ = state.tx.send(WsEvent::ScreenFrame {
                        display: disp.clone(),
                        path: path.clone(),
                    });
                    frame_ring.push_back((ts, path));

                    // Prune frames older than 10 seconds
                    let cutoff = now_ms().saturating_sub(10_000);
                    while let Some((frame_ts, _)) = frame_ring.front() {
                        if *frame_ts < cutoff {
                            let (_, old_path) = frame_ring.pop_front().unwrap();
                            let _ = std::fs::remove_file(&old_path);
                        } else {
                            break;
                        }
                    }
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

        // Cleanup this display's dir when stream ends
        let _ = std::fs::remove_dir_all(&dir);
        info!("Screen stream stopped: {}", disp);
    })
}
