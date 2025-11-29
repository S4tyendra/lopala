use std::env;
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

mod embed;
mod files;
mod io;
mod pty;
mod screencast;
mod screenshot;
mod search;
mod server;
mod state;
mod tunnel;
mod upload;
mod ws;

#[derive(Debug)]
struct Args {
    port: u16,
    tunnel: bool,
}

fn parse_args() -> Args {
    let mut args = env::args().skip(1);
    let mut port = 8080;
    let mut tunnel = false;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-p" | "--port" => {
                let port_str = args.next().unwrap_or_else(|| {
                    eprintln!("Error: Missing value for port");
                    std::process::exit(1);
                });
                port = port_str.parse().unwrap_or_else(|_| {
                    eprintln!("Error: Invalid port number '{}'", port_str);
                    std::process::exit(1);
                });
            }
            "-t" | "--tunnel" => {
                tunnel = true;
            }
            "-h" | "--help" => {
                println!("Usage: lopala [OPTIONS]");
                println!("\nOptions:");
                println!("  -p, --port <PORT>  Port to listen on [default: 8080]");
                println!("  -t, --tunnel       Start a public cloudflared tunnel");
                println!("  -h, --help         Print help");
                std::process::exit(0);
            }
            unknown => {
                eprintln!("Error: Unknown argument '{}'", unknown);
                eprintln!("Run with --help for usage information.");
                std::process::exit(1);
            }
        }
    }

    Args { port, tunnel }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Initialize Logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    // 2. Parse CLI Arguments
    let args = parse_args();
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
            }
            Err(e) => {
                info!(
                    "Could not start tunnel: {}. Ensure cloudflared binary is in current dir.",
                    e
                );
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
    let upload_sessions: upload::UploadSessions =
        std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));
    upload::start_cleanup_task(upload_sessions.clone()).await;

    tokio::select! {
        res = server::start_server(args.port, global_state, upload_sessions) => { res? }
        _ = tokio::signal::ctrl_c() => {
            info!("Shutting down — clearing live stream dir");
            let _ = std::fs::remove_dir_all(live_dir);
        }
    }

    Ok(())
}
