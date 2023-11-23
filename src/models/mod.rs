mod base;
mod benchmark;
mod error;
mod store;
mod summary;

pub use self::benchmark::{Benchmark, BenchmarkBmc, BenchmarkCreate, BenchmarkFilter};
pub use self::error::{Error, Result};
pub use self::summary::{Summary, SummaryBmc, SummaryFilter};

// do not expose above the model layer
use self::store::{new_db_pool, Db};

#[derive(Debug, Clone)]
pub struct ModelManager {
    db: Db,
}

impl ModelManager {
    pub async fn new() -> Result<Self> {
        let db = new_db_pool().await?;

        Ok(ModelManager { db })
    }

    /// This is convenience for the health checking to avoid that having to
    /// have a db reference.
    pub async fn check_connectivity(&self) -> Result<()> {
        sqlx::query("SELECT 1").execute(self.db()).await?;
        Ok(())
    }

    pub(in crate::models) fn db(&self) -> &Db {
        &self.db
    }
}

#[cfg(test)]
impl From<Db> for ModelManager {
    fn from(db: Db) -> Self {
        Self { db }
    }
}
