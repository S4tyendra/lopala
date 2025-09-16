use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=ui/src");
    println!("cargo:rerun-if-changed=ui/astro.config.mjs");

    let status = Command::new("bun")
        .args(["run", "build"])
        .current_dir("ui")
        .status();

    if let Ok(s) = status {
        if !s.success() {
            println!("cargo:warning=UI build failed");
        }
    } else {
         println!("cargo:warning=Failed to run bun. Ensure bun is installed.");
    }
}
