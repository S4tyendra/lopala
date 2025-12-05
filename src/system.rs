use axum::{
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use sysinfo::System;
use crate::state::{GlobalState, WsEvent, SystemVitals, ProcessInfo};
use tracing::info;

// ─── Vitals broadcast loop ───────────────────────────────────────────────────

/// Spawns a background task that collects CPU/RAM/Disk stats every 2s
/// and broadcasts them to all connected clients via WsEvent::SystemVitals.
pub fn start_vitals_loop(state: GlobalState) {
    tokio::spawn(async move {
        let mut sys = System::new_all();
        // Initial refresh to populate baselines
        sys.refresh_all();
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        loop {
            sys.refresh_all();

            let cpu_percent = sys.global_cpu_info().cpu_usage() as f64;
            let cpu_per_core: Vec<f64> = sys.cpus().iter().map(|c| c.cpu_usage() as f64).collect();
            let ram_used_mb = sys.used_memory() / (1024 * 1024);
            let ram_total_mb = sys.total_memory() / (1024 * 1024);
            let swap_used_mb = sys.used_swap() / (1024 * 1024);
            let swap_total_mb = sys.total_swap() / (1024 * 1024);

            // Disk I/O — read from /proc/diskstats
            let mut disk_read: u64 = 0;
            let mut disk_write: u64 = 0;
            if let Ok(content) = std::fs::read_to_string("/proc/diskstats") {
                for line in content.lines() {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 14 {
                        let dev = parts[2];
                        let is_whole_disk = (dev.starts_with("sd") && dev.len() == 3)
                            || (dev.starts_with("nvme") && dev.contains("n") && !dev.contains("p"));
                        if is_whole_disk {
                            disk_read += parts[5].parse::<u64>().unwrap_or(0) * 512;
                            disk_write += parts[9].parse::<u64>().unwrap_or(0) * 512;
                        }
                    }
                }
            }

            let vitals = SystemVitals {
                cpu_percent,
                ram_used_mb,
                ram_total_mb,
                swap_used_mb,
                swap_total_mb,
                disk_read_bytes: disk_read,
                disk_write_bytes: disk_write,
                cpu_per_core,
            };

            let _ = state.tx.send(WsEvent::SystemVitals { vitals });
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    });
}

// ─── HTTP handlers ───────────────────────────────────────────────────────────

/// GET /api/system/ps — returns process list
pub async fn list_processes() -> impl IntoResponse {
    let mut sys = System::new();
    sys.refresh_processes();

    let mut procs: Vec<ProcessInfo> = sys.processes().iter().map(|(pid, proc_)| {
        ProcessInfo {
            pid: pid.as_u32(),
            name: proc_.name().to_string(),
            cpu: proc_.cpu_usage() as f64,
            mem_mb: proc_.memory() / (1024 * 1024),
            user: proc_.user_id()
                .map(|u| u.to_string())
                .unwrap_or_else(|| "-".into()),
            command: proc_.cmd().iter().map(|s| s.to_string()).collect::<Vec<_>>().join(" "),
        }
    }).collect();

    // Sort by CPU desc by default
    procs.sort_by(|a, b| b.cpu.partial_cmp(&a.cpu).unwrap_or(std::cmp::Ordering::Equal));

    Json(procs).into_response()
}

#[derive(Deserialize)]
pub struct KillReq {
    pub pid: u32,
}

/// POST /api/system/kill — send SIGKILL to a process
pub async fn kill_process(Json(req): Json<KillReq>) -> impl IntoResponse {
    match nix::sys::signal::kill(
        nix::unistd::Pid::from_raw(req.pid as i32),
        nix::sys::signal::Signal::SIGKILL,
    ) {
        Ok(_) => {
            info!("Killed process {}", req.pid);
            (StatusCode::OK, Json(serde_json::json!({"ok": true, "pid": req.pid}))).into_response()
        }
        Err(e) => {
            (StatusCode::FORBIDDEN, Json(serde_json::json!({"error": e.to_string()}))).into_response()
        }
    }
}
