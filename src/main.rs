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

    // 4. Start HTTP Server (Blocking)
    info!("Lopala UI ready at http://localhost:{}", args.port);
    let global_state = state::GlobalState::new();
    server::start_server(args.port, global_state).await?;

    Ok(())
}
