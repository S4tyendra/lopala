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
    /// Port to listen on
    #[arg(short, long, default_value_t = 8080)]
    port: u16,

    /// Whether to start a public cloudflared tunnel
    #[arg(short, long, default_value_t = false)]
    tunnel: bool,
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
    info!("Starting Lopala Terminal Server...");

    // Clean the ephemeral live-stream dir on every startup
    let live_dir = "/tmp/lopala/live";
    let _ = std::fs::remove_dir_all(live_dir);
    std::fs::create_dir_all(live_dir).unwrap_or_default();
    info!("Cleared live stream dir: {}", live_dir);

    // 3. (Optional) Run Cloudflare Tunnel
    let _tunnel = if args.tunnel {
        match tunnel::Tunnel::start(args.port).await {
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
    info!("Lopala UI ready at http://localhost:{}", args.port);
    let global_state = state::GlobalState::new();

    // Upload sessions store + 3-min idle reaper
    let upload_sessions: upload::UploadSessions = std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));
    upload::start_cleanup_task(upload_sessions.clone()).await;

    // System vitals broadcast loop (CPU/RAM/Disk every 2s)
    system::start_vitals_loop(global_state.clone());

    tokio::select! {
        res = server::start_server(args.port, global_state, upload_sessions) => { res? }
        _ = tokio::signal::ctrl_c() => {
            info!("Shutting down — clearing live stream dir");
            let _ = std::fs::remove_dir_all(live_dir);
        }
    }

    Ok(())
}
