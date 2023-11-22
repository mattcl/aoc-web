pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed to create DB pool: {0}")]
    FailedToCreatePool(String),

    #[error(transparent)]
    Unhandled(#[from] anyhow::Error),
}
