use std::fs::File as StdFile;
use std::os::unix::io::{FromRawFd, RawFd};
use tokio::io::{AsyncRead, AsyncWrite, split, ReadHalf, WriteHalf};
use tokio::fs::File as TokioFile;
use nix::fcntl::{fcntl, FcntlArg, OFlag};

/// Safely wraps a PTY raw file descriptor for tokio async IO
pub fn prepare_pty_stream(fd: RawFd) -> anyhow::Result<(ReadHalf<TokioFile>, WriteHalf<TokioFile>)> {
    // Set non-blocking for tokio
    fcntl(fd, FcntlArg::F_SETFL(OFlag::O_NONBLOCK))?;

    // Wrap raw fd in a tokio-aware File
    let std_file = unsafe { StdFile::from_raw_fd(fd) };
    let tokio_file = TokioFile::from_std(std_file);

    // Split for independent reading and writing
    Ok(split(tokio_file))
}

pub type PtyReader = ReadHalf<TokioFile>;
pub type PtyWriter = WriteHalf<TokioFile>;
