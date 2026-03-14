use nix::pty::{Winsize, forkpty};
use nix::unistd::ForkResult;
use std::os::unix::io::{AsRawFd, OwnedFd};
use tracing::{error, info};

/// OS-LEVEL SHIT: forkpty, ioctl, and SIGWINCH handling.
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
                    Ok(Pty {
                        fd: fork_result.master,
                        pid,
                    })
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

impl Drop for Pty {
    fn drop(&mut self) {
        // Kill the child shell process when the master Pty file is dropped.
        let _ = nix::sys::signal::kill(self.pid, nix::sys::signal::SIGKILL);
        let _ = nix::sys::wait::waitpid(self.pid, None);
    }
}
