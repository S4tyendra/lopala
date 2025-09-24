use nix::pty::{forkpty, Winsize};
use nix::unistd::{ForkResult};
use std::os::unix::io::{OwnedFd, AsRawFd};
use tracing::{info, error};

/// THE DIRTY OS-LEVEL SHIT: forkpty, ioctl, and SIGWINCH handling.
/// This is where the magic happens/everything breaks.
pub struct Pty {
    pub fd: OwnedFd,
    pub pid: nix::unistd::Pid,
}

impl Pty {
    /// Spawn a new PTY with a shell (defaulting to /bin/bash or $SHELL)
    pub fn spawn() -> anyhow::Result<Self> {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".into());
        let winsize = Winsize {
            ws_row: 24,
            ws_col: 80,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };

        match unsafe { forkpty(Some(&winsize), None) } {
            Ok(fork_result) => match fork_result.fork_result {
                ForkResult::Parent { child: pid } => {
                    info!("Spawned PTY child process PID: {}", pid);
                    Ok(Pty { fd: fork_result.master, pid })
                }
                ForkResult::Child => {
                    let c_shell = std::ffi::CString::new(shell).unwrap();
                    let args = [c_shell.clone()];
                    let _ = nix::unistd::execv(&c_shell, &args);
                    error!("Failed to exec shell in PTY child");
                    std::process::exit(1);
                }
            },
            Err(e) => anyhow::bail!("forkpty failed: {}", e),
        }
    }

    /// Resize terminal via ioctl TIOCSWINSZ
    pub fn resize(&self, rows: u16, cols: u16) -> anyhow::Result<()> {
        let ws = Winsize {
            ws_row: rows,
            ws_col: cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        unsafe {
            if libc::ioctl(self.fd.as_raw_fd(), libc::TIOCSWINSZ, &ws) < 0 {
                return Err(anyhow::anyhow!("ioctl TIOCSWINSZ failed"));
            }
        }
        Ok(())
    }
}
