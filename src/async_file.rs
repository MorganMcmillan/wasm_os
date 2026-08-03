use std::io::{ErrorKind, SeekFrom};

use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt, Stderr, Stdin, Stdout, stderr, stdin, stdout},
};

pub enum FileError {
    CannotRead,
    CannotWrite,
    CannotSeek,
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

fn get_error(error: std::io::Error) -> FileError {
    match error.kind() {
        ErrorKind::ReadOnlyFilesystem => FileError::CannotWrite,
        ErrorKind::NotSeekable => FileError::CannotSeek,
        _ => FileError::Other,
    }
}

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
            Self::Stdin(stdin) => stdin.read(buf).await.map_err(get_error),
            Self::File(file) => file.read(buf).await.map_err(get_error),
            _ => Err(FileError::CannotRead),
        }
    }

    pub async fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize, FileError> {
        match self {
            Self::Null => Ok(0),
            Self::Stdin(stdin) => stdin.read_to_end(buf).await.map_err(get_error),
            Self::File(file) => file.read_to_end(buf).await.map_err(get_error),
            _ => Err(FileError::CannotRead),
        }
    }

    pub async fn write(&mut self, src: &[u8]) -> Result<usize, FileError> {
        match self {
            Self::Null => Ok(src.len()),
            Self::Stdout(stdout) => stdout.write(src).await.map_err(get_error),
            Self::Stderr(stderr) => stderr.write(src).await.map_err(get_error),
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
            Self::File(file) => file.seek(seek_from).await.map_err(get_error),
            _ => Err(FileError::CannotSeek),
        }
    }
}
