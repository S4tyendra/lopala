use reqwest;
use serde::Deserialize;
use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;
use uuid::Uuid;

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
}

pub async fn check_and_update(force_update: bool) -> anyhow::Result<()> {
    let repo = "s4tyendra/latch";
    let client = reqwest::Client::builder().user_agent("latch-updater").build()?;
    let url = format!("https://api.github.com/repos/{}/releases/latest", repo);
    
    let resp = client.get(&url).send().await.map_err(|e| anyhow::anyhow!("Could not reach GitHub API: {}", e))?;
    let release_text = resp.text().await?;
    
    let release: GithubRelease = serde_json::from_str(&release_text)
        .map_err(|_| anyhow::anyhow!("Could not parse latest release tag from GitHub API response"))?;
        
    let latest_tag = release.tag_name;
    let current_ver = format!("v{}", env!("CARGO_PKG_VERSION"));
    
    if force_update {
        println!("→ OS: {}  |  Architecture: {}", env::consts::OS, env::consts::ARCH);
        println!("→ Checking latest release...");
        println!("→ Latest release: {}", latest_tag);
        println!("→ Installed version: {}", current_ver);

        if latest_tag == current_ver {
            println!("✅ Already up to date ({}). Skipping download.", latest_tag);
            return Ok(());
        }
        
        let arch = env::consts::ARCH;
        let os = env::consts::OS;
        
        let binary_name = match (os, arch) {
            ("linux", "x86_64") => "latch-linux-x64",
            ("linux", "aarch64") => "latch-linux-arm64",
            _ => anyhow::bail!("Error: Unsupported architecture: {}", arch),
        };
        
        let download_url = format!("https://github.com/{}/releases/download/{}/{}", repo, latest_tag, binary_name);
        println!("→ Downloading {} from {} ...", binary_name, download_url);
        
        let binary_resp = client.get(&download_url).send().await?;
        if !binary_resp.status().is_success() {
            anyhow::bail!("Error: Download failed (URL: {})", download_url);
        }
        let binary_bytes = binary_resp.bytes().await?;
        
        if binary_bytes.len() < 1000000 {
            anyhow::bail!("Error: Downloaded file is suspiciously small ({} bytes). Aborting.", binary_bytes.len());
        }
        
        let tmp_path = format!("/tmp/latch.{}", Uuid::new_v4());
        fs::write(&tmp_path, &binary_bytes)?;
        fs::set_permissions(&tmp_path, fs::Permissions::from_mode(0o755))?;
        
        let current_exe = env::current_exe()?;
        let dest = current_exe.to_string_lossy();
        
        println!("→ Installing to {} (requires sudo)...", dest);
        
        let status = Command::new("sudo")
            .arg("mv")
            .arg(&tmp_path)
            .arg(dest.as_ref())
            .status()?;
            
        if !status.success() {
            let _ = fs::remove_file(&tmp_path);
            anyhow::bail!("Error: Failed to move binary to {}", dest);
        }
        
        // Also ensure +x on dest via sudo chmod +x
        let _ = Command::new("sudo").arg("chmod").arg("+x").arg(dest.as_ref()).status();
        
        println!("✅ Latch {} installed successfully.", latest_tag);
    } else {
        if latest_tag != current_ver {
            println!("🔔 A new version of Latch is available: {} (current: {})", latest_tag, current_ver);
            println!("   Run 'latch --update' to upgrade.\n");
        }
    }
    
    Ok(())
}
