use sea_query::{Cond, Expr, Iden, PostgresQueryBuilder, Query};
use sea_query_binder::SqlxBinder;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use strum::{EnumIter, IntoEnumIterator};

use super::{
    base::{bmc_list, DbBmc},
    Error, ModelManager, Result,
};

#[derive(Debug, Default, Clone, PartialEq, PartialOrd, FromRow, Serialize)]
#[cfg_attr(test, derive(Deserialize))]
pub struct Benchmark {
    pub id: i32,
    pub year: i32,
    pub day: i32,
    pub input: String,
    pub participant: String,
    pub language: String,
    pub mean: f64,
    pub stddev: f64,
    pub median: f64,
    #[sqlx(rename = "tuser")]
    pub user: f64,
    #[sqlx(rename = "tsystem")]
    pub system: f64,
    #[sqlx(rename = "tmin")]
    pub min: f64,
    #[sqlx(rename = "tmax")]
    pub max: f64,
}

// this sucks, but we have to wait for a newer version of sea-query to allow
// more control over the struct proc macro
#[derive(Debug, Clone, Copy, Iden, EnumIter)]
pub enum BenchmarkIden {
    #[iden = "benchmarks"]
    Table,
    Id,
    Year,
    Day,
    Input,
    Participant,
    Language,
    Mean,
    Stddev,
    Median,
    #[iden = "tuser"]
    User,
    #[iden = "tsystem"]
    System,
    #[iden = "tmin"]
    Min,
    #[iden = "tmax"]
    Max,
}

pub struct BenchmarkBmc;

impl DbBmc for BenchmarkBmc {
    const TABLE: &'static str = "benchmarks";
    type Iden = BenchmarkIden;

    fn table_iden() -> Self::Iden {
        Self::Iden::Table
    }
}

impl BenchmarkBmc {
    /// Create a new Benchmark or update an existing one on a collision.
    #[allow(dead_code)]
    pub async fn create_or_update(mm: &ModelManager, data: BenchmarkCreate) -> Result<i32> {
        let ids = Self::batch_create_or_update(mm, [data]).await?;

        Ok(ids[0])
    }

    /// Create new Benchmarks or update an existing ones on a collision.
    pub async fn batch_create_or_update<Iter>(mm: &ModelManager, data: Iter) -> Result<Vec<i32>>
    where
        Iter: IntoIterator<Item = BenchmarkCreate>,
    {
        let db = mm.db();

        let mut query = Query::insert();

        query.into_table(BenchmarkIden::Table).columns(
            BenchmarkIden::iter()
                .filter(|c| !matches!(c, BenchmarkIden::Table | BenchmarkIden::Id)),
        );

        // clunky, but since we don't know the length of the iterator ahead of
        // time and do not want to alloc to consume it
        let mut count = 0;

        for benchmark_data in data {
            query.values_panic([
                benchmark_data.year.into(),
                benchmark_data.day.into(),
                benchmark_data.input.into(),
                benchmark_data.participant.into(),
                benchmark_data.language.into(),
                benchmark_data.mean.into(),
                benchmark_data.stddev.into(),
                benchmark_data.median.into(),
                benchmark_data.user.into(),
                benchmark_data.system.into(),
                benchmark_data.min.into(),
                benchmark_data.max.into(),
            ]);
            count += 1;
        }

        if count == 0 {
            return Err(Error::EmptyBatch("benchmarks"));
        }

        let (sql, values) = query.build_sqlx(PostgresQueryBuilder);
        // probably not the best hack, but sea-query doesn't allow for setting
        // the ON CONSTRAINT expression.
        let sql = sql
            + r#" ON CONFLICT ON CONSTRAINT single_entry DO UPDATE SET
language = excluded.language,
mean = excluded.mean,
stddev = excluded.stddev,
median = excluded.median,
tuser = excluded.tuser,
tsystem = excluded.tsystem,
tmin = excluded.tmin,
tmax = excluded.tmax
RETURNING id"#;

        let ids = sqlx::query_as_with::<_, (i32,), _>(&sql, values)
            .fetch_all(db)
            .await?;

        Ok(ids.into_iter().map(|i| i.0).collect())
    }

    pub async fn list(mm: &ModelManager, filter: BenchmarkFilter) -> Result<Vec<Benchmark>> {
        bmc_list::<Self, _, _>(mm, filter).await
    }

    pub async fn get(mm: &ModelManager, id: i32) -> Result<Benchmark> {
        let db = mm.db();
        let (sql, values) = Query::select()
            .columns(BenchmarkIden::iter())
            .from(BenchmarkIden::Table)
            .and_where(Expr::col(BenchmarkIden::Id).eq(id))
            .build_sqlx(PostgresQueryBuilder);

        let entity = sqlx::query_as_with(&sql, values)
            .fetch_optional(db)
            .await?
            .ok_or(Error::EntityNotFound {
                table: BenchmarkBmc::TABLE,
                id,
            })?;

        Ok(entity)
    }
}

#[derive(Debug, Default, Clone, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
pub struct BenchmarkFilter {
    pub year: Option<i32>,
    pub day: Option<i32>,
    pub input: Option<String>,
    pub participant: Option<String>,
    pub language: Option<String>,
}

impl From<BenchmarkFilter> for Cond {
    fn from(value: BenchmarkFilter) -> Self {
        let mut cond = Cond::all();

        // there has to be a better way to do this, and there probably is
        if let Some(year) = value.year {
            cond = cond.add(Expr::col(BenchmarkIden::Year).eq(year));
        }

        if let Some(day) = value.day {
            cond = cond.add(Expr::col(BenchmarkIden::Day).eq(day));
        }

        if let Some(input) = value.input {
            cond = cond.add(Expr::col(BenchmarkIden::Input).eq(input));
        }

        if let Some(participant) = value.participant {
            cond = cond.add(Expr::col(BenchmarkIden::Participant).eq(participant));
        }

        if let Some(language) = value.language {
            cond = cond.add(Expr::col(BenchmarkIden::Language).eq(language));
        }

        cond
    }
}

#[derive(Debug, Default, Clone, PartialEq, PartialOrd, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
pub struct BenchmarkCreate {
    pub year: i32,
    pub day: i32,
    pub input: String,
    pub participant: String,
    pub language: String,
    pub mean: f64,
    pub stddev: f64,
    pub median: f64,
    pub user: f64,
    pub system: f64,
    pub min: f64,
    pub max: f64,
}

#[cfg(test)]
mod tests {
    use std::f64::EPSILON;

    use sqlx::PgPool;

    use super::*;

    // we need to do this to check the floating point values
    macro_rules! assert_benchmarks_equal {
        ($a:expr, $b:expr) => {
            assert_eq!($a.year, $b.year);
            assert_eq!($a.day, $b.day);
            assert_eq!($a.input, $b.input);
            assert_eq!($a.participant, $b.participant);
            assert_eq!($a.language, $b.language);
            assert!($a.mean - $b.mean < EPSILON, "mean differs");
            assert!($a.stddev - $b.stddev < EPSILON, "stddev differs");
            assert!($a.median - $b.median < EPSILON, "median differs");
            assert!($a.user - $b.user < EPSILON, "user differs");
            assert!($a.system - $b.system < EPSILON, "system differs");
            assert!($a.min - $b.min < EPSILON, "min differs");
            assert!($a.max - $b.max < EPSILON, "max differs");
        };
    }

    #[sqlx::test]
    async fn test_batch_create_or_update_ok(pool: PgPool) -> anyhow::Result<()> {
        let mm = ModelManager::from(pool);

        // insert a row that we'll conflict with
        let entry = BenchmarkCreate {
            year: 2023,
            day: 10,
            input: "input-baz".into(),
            participant: "foo".into(),
            language: "ruby".into(),
            mean: 0.24277,
            stddev: 0.00189,
            median: 0.1999,
            user: 0.2002,
            system: 0.04257,
            min: 0.1865,
            max: 0.4558,
        };

        let id = BenchmarkBmc::create_or_update(&mm, entry).await?;
        assert_eq!(id, 1000);

        // insert a conflicting row and a new row
        let data = vec![
            BenchmarkCreate {
                year: 2023,
                day: 10,
                input: "input-baz".into(),
                participant: "foo".into(),
                language: "ruby".into(),
                mean: 0.54277,
                stddev: 0.00189,
                median: 0.4999,
                user: 0.5002,
                system: 0.04257,
                min: 0.4865,
                max: 0.7558,
            },
            BenchmarkCreate {
                year: 2023,
                day: 11,
                input: "input-baz".into(),
                participant: "foo".into(),
                language: "ruby".into(),
                mean: 0.94277,
                stddev: 0.00189,
                median: 0.8999,
                user: 0.9002,
                system: 0.04257,
                min: 0.7865,
                max: 0.9558,
            },
        ];

        let ids = BenchmarkBmc::batch_create_or_update(&mm, data.clone()).await?;
        // we leave a gap because of the way the insert works in bulk with a
        // conflict
        assert_eq!(ids, vec![1000, 1002]);

        let all = BenchmarkBmc::list(&mm, BenchmarkFilter::default()).await?;

        assert_eq!(all.len(), 2);

        // verify the conflicting row updated
        let b = all.iter().find(|b| b.id == 1000).unwrap();

        assert_benchmarks_equal!(b, data[0]);

        Ok(())
    }

    #[sqlx::test]
    async fn test_batch_create_empty_is_err(pool: PgPool) -> anyhow::Result<()> {
        let mm = ModelManager::from(pool);

        let data = Vec::new();

        let res = BenchmarkBmc::batch_create_or_update(&mm, data).await;

        assert!(matches!(res, Err(Error::EmptyBatch("benchmarks"))));

        Ok(())
    }

    #[sqlx::test(fixtures("../../fixtures/benchmarks.sql"))]
    async fn test_list_all_ok(pool: PgPool) -> anyhow::Result<()> {
        let mm = ModelManager::from(pool);
        let b = BenchmarkBmc::list(&mm, BenchmarkFilter::default()).await?;
        // how to make this more general
        assert_eq!(b.len(), 4);
        Ok(())
    }

    #[sqlx::test]
    async fn test_list_all_empty_ok(pool: PgPool) -> anyhow::Result<()> {
        let mm = ModelManager::from(pool);
        let b = BenchmarkBmc::list(&mm, BenchmarkFilter::default()).await?;
        // how to make this more general
        assert_eq!(b, vec![]);
        Ok(())
    }

    #[sqlx::test(fixtures("../../fixtures/benchmarks.sql"))]
    async fn test_list_filter_ok(pool: PgPool) -> anyhow::Result<()> {
        let mm = ModelManager::from(pool);

        // filter out some things
        let b = BenchmarkBmc::list(
            &mm,
            BenchmarkFilter {
                input: Some("input-bar".to_string()),
                ..Default::default()
            },
        )
        .await?;
        assert_eq!(b.len(), 2);
        assert_eq!(b[0].id, 1002);
        assert_eq!(b[1].id, 1003);
        Ok(())
    }

    #[sqlx::test(fixtures("../../fixtures/benchmarks.sql"))]
    async fn test_get_ok(pool: PgPool) -> anyhow::Result<()> {
        let mm = ModelManager::from(pool);
        BenchmarkBmc::get(&mm, 1000).await?;
        Ok(())
    }

    #[sqlx::test]
    async fn test_get_error(pool: PgPool) -> anyhow::Result<()> {
        let mm = ModelManager::from(pool);
        let res = BenchmarkBmc::get(&mm, 1000).await;

        assert!(res.is_err());

        assert!(
            matches!(
                res,
                Err(Error::EntityNotFound {
                    table: BenchmarkBmc::TABLE,
                    id: 1000,
                })
            ),
            "EntityNotFound does not match"
        );

        Ok(())
    }
}
