use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Not a riff file")]
    NotRiff,
    #[error("Not a wave format file")]
    NotWave,
    #[error("Invalid fmt chunk")]
    InvalidFmtChunk,
    #[error("Invalid Extended Info, less than 22 bytes")]
    InvalidExtendedInfo,
    #[error("IO error reading file: {0}")]
    IOError(io::Error),
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::IOError(error)
    }
}
