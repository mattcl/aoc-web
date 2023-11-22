use super::store;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{table} not found: {id}")]
    EntityNotFound { table: &'static str, id: i32 },

    #[error(transparent)]
    StoreError(#[from] store::Error),

    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),

    #[error(transparent)]
    Unhandled(#[from] anyhow::Error),
}
