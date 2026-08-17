use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    IOError(#[from] std::io::Error),

    #[error("UTF-8 error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Cache error: {0}")]
    CacheError(#[from] CacheError),

    #[error("Exec error: {0}")]
    ExecError(#[from] ExecError),

    #[error("Invalid target")]
    InvalidTarget,

    #[error("Route operation error: {0}")]
    RouteOpError(#[from] RouteOpError),
}

#[derive(Error, Debug)]
pub enum CacheError {
    #[error("IO error: {0}")]
    IOError(#[from] std::io::Error),
}

#[derive(Error, Debug)]
pub enum ExecError {
    #[error("Exec error: {0}")]
    ExecError(#[from] std::io::Error),
}

/// Error type for route table operations.
#[derive(Error, Debug)]
pub enum RouteOpError {
    #[error("IO error: {0}")]
    OpError(#[from] std::io::Error),

    #[error("cannot find system default gateway")]
    NoGatewayError,

    #[error("cannot create handle")]
    HandleInitError,

    #[error("futures join error: {0}")]
    FutureError(#[from] tokio::task::JoinError),

    #[error("get default interface error: {0}")]
    GetInterfaceError(String),

    #[error("route already exists")]
    RouteAlreadyExistsError,

    #[error("route not found")]
    RouteNotFoundError,
}

pub type Result<T> = std::result::Result<T, Error>;
