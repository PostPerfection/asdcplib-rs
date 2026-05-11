//! Error types for asdcplib.

/// Error type for asdcplib operations.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("asdcplib error (code {0})")]
    AsdcpError(i32),
    #[error("invalid argument: {0}")]
    InvalidArgument(&'static str),
    #[error("buffer too small (needed {needed}, had {capacity})")]
    BufferTooSmall { needed: usize, capacity: usize },
}

pub type Result<T> = std::result::Result<T, Error>;

/// Check an asdcplib result code, returning Ok(()) or an error.
pub fn check(result: i32) -> Result<()> {
    let ok = unsafe { asdcplib_sys::asdcp_result_ok(result) };
    if ok != 0 {
        Ok(())
    } else {
        Err(Error::AsdcpError(result))
    }
}
