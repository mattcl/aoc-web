use super::store;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{table} not found: {id}")]
    EntityNotFound { table: &'static str, id: i32 },

    #[error("Day out of range: {0}")]
    DayOutOfRange(i32),

    #[error("Empty batch for {0}")]
    EmptyBatch(&'static str),

    #[error(transparent)]
    Store(#[from] store::Error),

    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),

    #[error(transparent)]
    Unhandled(#[from] anyhow::Error),
}
