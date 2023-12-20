use std::{fmt, path::{PathBuf, StripPrefixError}};

#[derive(Debug)]
pub enum ChksumError {
    PartialHashFailed((PathBuf, std::io::Error)),
    HashFailed((PathBuf, std::io::Error)),
    IOError(std::io::Error),
    PathStripPrefixError(StripPrefixError),
    OpenFileFailed((PathBuf, std::io::Error)),
    ReadFileFailed((PathBuf, std::io::Error)),
    FileDoesNotExist(PathBuf)
}

impl std::error::Error for ChksumError {}
impl fmt::Display for ChksumError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChksumError::PartialHashFailed((path, err)) => write!(f, "Failed to partially hash '{}': {err}", path.display()),
            ChksumError::HashFailed((path, err)) => write!(f, "Failed to hash '{}': {err}", path.display()),
            ChksumError::IOError(err) => write!(f, "IO Error: {err}"),
            ChksumError::PathStripPrefixError(err) => write!(f, "Failed to strip path prefix: {err}"),
            ChksumError::OpenFileFailed((path, err)) => write!(
                f,
                "Failed to open file '{}': {err}",
                path.display()),
            ChksumError::ReadFileFailed((path, err)) => write!(
                f,
                "Failed to read file '{}': {err}",
                path.display()),
            ChksumError::FileDoesNotExist(path) => write!(f, "File does not exist '{}'", path.display()),
        }
    }
}

impl From<std::io::Error> for ChksumError {
    fn from(value: std::io::Error) -> Self {
        ChksumError::IOError(value)
    }
}


impl From<ChksumError> for std::io::Error {
    fn from(value: ChksumError) -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::Other, value)
    }
}

impl From<StripPrefixError> for ChksumError {
    fn from(value: StripPrefixError) -> Self {
        ChksumError::PathStripPrefixError(value)
    }
}
