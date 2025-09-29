use std::fs::File as StdFile;
use std::os::unix::io::{FromRawFd, RawFd};
use tokio::io::{split, ReadHalf, WriteHalf};
use nix::fcntl::{fcntl, FcntlArg, FdFlag, OFlag};
use tokio::io::unix::AsyncFd;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use std::io::{Read, Write};

/// Wrap the standard pseudo-terminal FD to directly interface with the Tokio reactor
/// bypassing `tokio::fs::File` which attempts `pwrite`/`pread` operations incompatible with TTYs.
pub struct AsyncPty {
    inner: AsyncFd<StdFile>,
}

impl AsyncPty {
    pub fn new(fd: RawFd) -> anyhow::Result<Self> {
        // We MUST dup() the FD. `AsyncFd` takes ownership, but `Pty` retains the original.
        let new_fd = nix::unistd::dup(fd)?;
        
        // Ensure FD is closed on exec
        fcntl(new_fd, FcntlArg::F_SETFD(FdFlag::FD_CLOEXEC))?;
        
        // Ensure non-blocking is set so epoll reactor handles EAGAIN properly
        let mut flags = OFlag::from_bits_truncate(fcntl(new_fd, FcntlArg::F_GETFL)?);
        flags.insert(OFlag::O_NONBLOCK);
        fcntl(new_fd, FcntlArg::F_SETFL(flags))?;
        
        let std_file = unsafe { StdFile::from_raw_fd(new_fd) };
        let inner = AsyncFd::new(std_file)?;
        
        Ok(AsyncPty { inner })
    }
}

impl AsyncRead for AsyncPty {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        loop {
            let mut guard = futures_util::ready!(self.inner.poll_read_ready(cx))?;
            
            let unfilled = buf.initialize_unfilled();
            match guard.try_io(|inner| (&*inner.get_ref()).read(unfilled)) {
                Ok(Ok(len)) => {
                    buf.advance(len);
                    return Poll::Ready(Ok(()));
                }
                Ok(Err(err)) => return Poll::Ready(Err(err)),
                Err(_would_block) => continue, // Reactor says EAGAIN, we retry via poll_read_ready loop
            }
        }
    }
}

impl AsyncWrite for AsyncPty {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        loop {
            let mut guard = futures_util::ready!(self.inner.poll_write_ready(cx))?;
            
            match guard.try_io(|inner| (&*inner.get_ref()).write(buf)) {
                Ok(Ok(len)) => return Poll::Ready(Ok(len)),
                Ok(Err(err)) => return Poll::Ready(Err(err)),
                Err(_would_block) => continue,
            }
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

/// Safely wraps a PTY raw file descriptor for tokio async IO without pwrite bugs
pub fn prepare_pty_stream(fd: RawFd) -> anyhow::Result<(ReadHalf<AsyncPty>, WriteHalf<AsyncPty>)> {
    let pty = AsyncPty::new(fd)?;
    Ok(split(pty))
}

pub type PtyReader = ReadHalf<AsyncPty>;
pub type PtyWriter = WriteHalf<AsyncPty>;
