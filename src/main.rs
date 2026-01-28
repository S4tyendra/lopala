use clap::Parser;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod server;
mod ws;
mod pty;
mod io;
mod tunnel;
mod embed;
mod state;
mod files;
mod screenshot;
mod screencast;
mod search;
mod upload;
mod system;


#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Port to listen on. If omitted, defaults to 8080 (or a random 40000-60000 port if --tunnel is set).
    #[arg(short, long)]
    port: Option<u16>,

    /// Whether to start a public cloudflared tunnel
    #[arg(short, long, default_value_t = false)]
    tunnel: bool,

    /// 4-digit PIN for authentication. If not provided, one is generated.
    #[arg(long)]
    pin: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Initialize Logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    // 2. Parse CLI Arguments
    let args = Args::parse();
    
    let port = args.port.unwrap_or_else(|| {
        if args.tunnel {
            let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().subsec_nanos();
            let p = 40000 + (nanos % 20000);
            p as u16
        } else {
            8080
        }
    });

    let pin = args.pin.unwrap_or_else(|| {
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().subsec_nanos();
        let p = 1000 + (nanos % 9000);
        format!("{:04}", p)
    });

    info!("Starting Lopala Terminal Server...");
    info!("===================================");
    info!(" AUTH PIN: {}", pin);
    info!("===================================");

    // Clean the ephemeral live-stream dir on every startup
    let live_dir = "/tmp/lopala/live";
    let _ = std::fs::remove_dir_all(live_dir);
    std::fs::create_dir_all(live_dir).unwrap_or_default();
    info!("Cleared live stream dir: {}", live_dir);

    // Ephemeral bin directory for injected CLI tools (inherits to all PTYs)
    let bin_dir = "/tmp/lopala/bin";
    std::fs::create_dir_all(bin_dir).unwrap_or_default();
    let wdl_path = format!("{}/wdl", bin_dir);
    let wdl_script = r#"#!/usr/bin/env bash
if [ -z "$1" ]; then
    echo "Usage: wdl <file-path>"
    exit 1
fi
REAL_PATH=$(realpath "$1" 2>/dev/null)
if [ ! -f "$REAL_PATH" ]; then
   echo "Error: File not found: $1"
   exit 1
fi
echo "Triggering download in browser: $REAL_PATH"
printf "\033]999;DOWNLOAD;%s\007" "$REAL_PATH"
"#;
    if std::fs::write(&wdl_path, wdl_script).is_ok() {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&wdl_path, std::fs::Permissions::from_mode(0o755));
    }

    if let Ok(current_path) = std::env::var("PATH") {
        if !current_path.contains(bin_dir) {
            unsafe {
                std::env::set_var("PATH", format!("{}:{}", bin_dir, current_path));
            }
        }
    }

    // 3. (Optional) Run Cloudflare Tunnel
    let _tunnel = if args.tunnel {
        match tunnel::Tunnel::start(port).await {
            Ok(t) => {
                info!("Public tunnel initiated.");
                Some(t)
            },
            Err(e) => {
                info!("Could not start tunnel: {}. Ensure cloudflared binary is in current dir.", e);
                None
            }
        }
    } else {
        None
    };

    // 4. Start HTTP Server (with graceful CTRL-C cleanup)
    info!("Lopala UI ready at http://localhost:{}", port);
    let global_state = state::GlobalState::new();

    // Upload sessions store + 3-min idle reaper
    let upload_sessions: upload::UploadSessions = std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));
    upload::start_cleanup_task(upload_sessions.clone()).await;

    // System vitals broadcast loop (CPU/RAM/Disk every 2s)
    system::start_vitals_loop(global_state.clone());

    tokio::select! {
        res = server::start_server(port, pin, global_state, upload_sessions) => { res? }
        _ = tokio::signal::ctrl_c() => {
            info!("Shutting down — clearing live stream dir");
            let _ = std::fs::remove_dir_all(live_dir);
        }
    }

    Ok(())
}
