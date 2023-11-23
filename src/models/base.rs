use sea_query::{Cond, PostgresQueryBuilder, Query};
use sea_query_binder::SqlxBinder;
use sqlx::{postgres::PgRow, FromRow};
use strum::IntoEnumIterator;

use super::{ModelManager, Result};

pub trait DbBmc {
    const TABLE: &'static str;

    type Iden: 'static + IntoEnumIterator + sea_query::Iden;

    // clunky for now
    fn table_iden() -> Self::Iden;
}

pub async fn bmc_list<MC, T, F>(mm: &ModelManager, filter: F) -> Result<Vec<T>>
where
    MC: DbBmc,
    Cond: From<F>,
    T: for<'r> FromRow<'r, PgRow> + Unpin + Send,
{
    let db = mm.db();

    let mut query = Query::select();

    query
        .columns(MC::Iden::iter())
        .from(MC::table_iden())
        .cond_where(Cond::from(filter));

    let (sql, values) = query.build_sqlx(PostgresQueryBuilder);

    let entities = sqlx::query_as_with(&sql, values).fetch_all(db).await?;

    Ok(entities)
}
