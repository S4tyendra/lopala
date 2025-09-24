use tokio::process::{Command, Child};
use tokio::io::{BufReader, AsyncBufReadExt};
use tracing::{info, warn, error};
use std::process::Stdio;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Manages the cloudflared tunnel lifecycle and scrapes the public URL
pub struct Tunnel {
    pub process: Child,
    pub public_url: Arc<Mutex<Option<String>>>,
}

impl Tunnel {
    /// Start a new cloudflared quick-tunnel
    pub async fn start(port: u16) -> anyhow::Result<Self> {
        info!("Starting cloudflared tunnel on port {}...", port);
        
        let url = format!("http://127.0.0.1:{}", port);
        let mut child = Command::new("./cloudflared")
            .args(["tunnel", "--url", &url])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped()) // cloudflared logs usually go to stderr
            .spawn()?;

        let stderr = child.stderr.take().ok_or_else(|| anyhow::anyhow!("Failed to capture stderr"))?;
        let public_url = Arc::new(Mutex::new(None));
        
        // Background scraper for the tunnel URL
        let url_clone = public_url.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                // cloudflared look-for pattern: https://*.trycloudflare.com
                if let Some(pos) = line.find("https://") {
                    let part = &line[pos..];
                    if let Some(end) = part.find(".trycloudflare.com") {
                        let final_url = &part[..end + 18];
                        let mut lock = url_clone.lock().await;
                        *lock = Some(final_url.to_string());
                        info!("Tunnel Online: {}", final_url);
                        break;
                    }
                }
                // Optional: stop scraping if we found it or if it errors
            }
        });

        Ok(Tunnel {
            process: child,
            public_url,
        })
    }
}
