use tokio::process::{Command, Child};
use tokio::io::{BufReader, AsyncBufReadExt};
use tracing::info;
use std::process::Stdio;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::path::{Path, PathBuf};

/// Manages the cloudflared tunnel lifecycle and scrapes the public URL
pub struct Tunnel {
    #[allow(dead_code)]
    pub process: Child,
    #[allow(dead_code)]
    pub public_url: Arc<Mutex<Option<String>>>,
}

impl Tunnel {
    /// Start a new cloudflared quick-tunnel
    pub async fn start(port: u16) -> anyhow::Result<Self> {
        info!("Starting cloudflared tunnel on port {}...", port);
        
        let cf_bin = Self::ensure_cloudflared().await?;
        let url = format!("http://localhost:{}", port);
        let mut child = Command::new(&cf_bin)
            .args(["tunnel", "--url", &url])
            .stdout(Stdio::null())
            .stderr(Stdio::piped()) 
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
                        if lock.is_none() {
                            *lock = Some(final_url.to_string());
                            info!("Tunnel Online: {}", final_url);
                        }
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

    async fn ensure_cloudflared() -> anyhow::Result<PathBuf> {
        // 1. Check in PATH
        if let Ok(out) = std::process::Command::new("which").arg("cloudflared").output() {
            let p = String::from_utf8_lossy(&out.stdout);
            if !p.trim().is_empty() {
                return Ok(PathBuf::from(p.trim()));
            }
        }

        // 2. Check in ~/.local/bin
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
        let local_bin = Path::new(&home).join(".local").join("bin");
        let cf_path = local_bin.join("cloudflared");

        if cf_path.exists() {
            return Ok(cf_path);
        }

        // 3. Download based on ARCH
        info!("cloudflared not found natively. Preparing to download to {:?}", cf_path);
        std::fs::create_dir_all(&local_bin)?;

        let arch = std::env::consts::ARCH;
        let suffix = match arch {
            "x86_64" => "cloudflared-linux-amd64",
            "aarch64" => "cloudflared-linux-arm64",
            "arm" => "cloudflared-linux-armhf", 
            "x86" => "cloudflared-linux-386",
            _ => anyhow::bail!("Unsupported architecture for cloudflared: {}", arch),
        };

        let url = format!("https://github.com/cloudflare/cloudflared/releases/download/2026.3.0/{}", suffix);
        info!("Downloading cloudflared from {} ...", url);
        
        let status = tokio::process::Command::new("curl")
            .args(["-L", "-o", cf_path.to_str().unwrap(), &url])
            .status().await?;
            
        if !status.success() {
            // fallback to wget
            let st = tokio::process::Command::new("wget")
                .args(["-O", cf_path.to_str().unwrap(), &url])
                .status().await?;
            if !st.success() {
                anyhow::bail!("Failed to download cloudflared binary");
            }
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(meta) = std::fs::metadata(&cf_path) {
                let mut perms = meta.permissions();
                perms.set_mode(0o755);
                let _ = std::fs::set_permissions(&cf_path, perms);
            }
        }

        info!("cloudflared downloaded and installed successfully.");
        Ok(cf_path)
    }
}
