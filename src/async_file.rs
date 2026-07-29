use std::io::SeekFrom;

use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt, Stderr, Stdin, Stdout, stderr, stdin, stdout},
};

pub enum FileError {
    CannotRead,
    CannotWrite,
    CannotSeek,
    FileClosed,
    InvalidSeekOption,
    Other,
}

pub enum AsyncFile {
    /// Represents no file. Reading and writing does nothing.
    Null,
    Stdin(Stdin),
    Stdout(Stdout),
    Stderr(Stderr),
    File(File),
}

unsafe impl Send for AsyncFile {}
unsafe impl Sync for AsyncFile {}

impl AsyncFile {
    pub fn stdin() -> Self {
        Self::Stdin(stdin())
    }

    pub fn stdout() -> Self {
        Self::Stdout(stdout())
    }

    pub fn stderr() -> Self {
        Self::Stderr(stderr())
    }

    pub async fn read(&mut self, buf: &mut [u8]) -> Result<usize, FileError> {
        match self {
            Self::Null => Ok(0),
            Self::Stdin(stdin) => stdin.read(buf).await.map_err(|_| FileError::Other),
            Self::File(file) => file.read(buf).await.map_err(|_| FileError::Other),
            _ => Err(FileError::CannotRead),
        }
    }

    pub async fn write(&mut self, src: &[u8]) -> Result<usize, FileError> {
        match self {
            Self::Null => Ok(src.len()),
            Self::Stdout(stdout) => stdout.write(src).await.map_err(|_| FileError::Other),
            Self::Stderr(stderr) => stderr.write(src).await.map_err(|_| FileError::Other),
            _ => Err(FileError::CannotWrite),
        }
    }

    pub async fn seek(&mut self, offset: i64, from: u8) -> Result<u64, FileError> {
        let seek_from = match from {
            0 => SeekFrom::Start(offset as u64),
            1 => SeekFrom::End(offset),
            2 => SeekFrom::Current(offset),
            _ => return Err(FileError::InvalidSeekOption),
        };

        match self {
            Self::Null => Ok(0),
            Self::File(file) => file.seek(seek_from).await.map_err(|_| FileError::Other),
            _ => Err(FileError::CannotSeek),
        }
    }
}
