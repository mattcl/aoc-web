mod error;

use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use tracing::instrument;

use crate::config;

pub use self::error::{Error, Result};

pub type Db = Pool<Postgres>;

#[instrument(err(Debug))]
pub async fn new_db_pool() -> Result<Db> {
    let max_connections = if cfg!(test) {
        1
    } else {
        config().db.max_connections
    };

    PgPoolOptions::new()
        .max_connections(max_connections)
        .connect(config().db.url().as_str())
        .await
        .map_err(|e| Error::FailedToCreatePool(e.to_string()))
}
