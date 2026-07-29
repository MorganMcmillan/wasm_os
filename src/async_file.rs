use tokio::io::{AsyncRead, AsyncWrite};

trait AsyncReadWrite: AsyncRead + AsyncWrite {}

impl<T> AsyncReadWrite for T where T: AsyncRead + AsyncWrite {}

pub enum AsyncFile {
    ReadOnly(Box<dyn AsyncRead>),
    WriteOnly(Box<dyn AsyncWrite>),
    ReadWrite(Box<dyn AsyncReadWrite>),
    /// Represents no file. Reading and writing does nothing.
    Null,
}

unsafe impl Send for AsyncFile {}
unsafe impl Sync for AsyncFile {}

impl AsyncFile {
    pub fn read_only<T: AsyncRead + 'static>(value: T) -> Self {
        Self::ReadOnly(Box::new(value))
    }

    pub fn write_only<T: AsyncWrite + 'static>(value: T) -> Self {
        Self::WriteOnly(Box::new(value))
    }

    pub fn read_write<T: AsyncRead + AsyncWrite + 'static>(value: T) -> Self {
        Self::ReadWrite(Box::new(value))
    }
}
