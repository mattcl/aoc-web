use sea_query::{Cond, Expr, Iden, PostgresQueryBuilder, Query};
use sea_query_binder::SqlxBinder;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use strum::{EnumIter, IntoEnumIterator};

use super::{
    base::{bmc_list, DbBmc},
    Error, ModelManager, Result,
};

#[derive(
    Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, FromRow, Serialize, Deserialize,
)]
pub struct Participant {
    pub year: i32,
    pub name: String,
    pub language: String,
    pub repo: String,
}

// this sucks, but we have to wait for a newer version of sea-query to allow
// more control over the struct proc macro
#[derive(Debug, Clone, Copy, Iden, EnumIter)]
pub enum ParticipantIden {
    #[iden = "participants"]
    Table,
    Year,
    Name,
    Language,
    Repo,
}

pub struct ParticipantBmc;

impl DbBmc for ParticipantBmc {
    const TABLE: &'static str = "participants";
    type Iden = ParticipantIden;

    fn table_iden() -> Self::Iden {
        Self::Iden::Table
    }
}

impl ParticipantBmc {
    pub async fn batch_create_or_update<Iter>(
        mm: &ModelManager,
        data: Iter,
    ) -> Result<Vec<(i32, String)>>
    where
        Iter: IntoIterator<Item = Participant>,
    {
        let db = mm.db();

        let mut query = Query::insert();

        query
            .into_table(ParticipantIden::Table)
            .columns(ParticipantIden::iter().filter(|v| !matches!(v, ParticipantIden::Table)));

        for participant_data in data {
            query.values_panic([
                participant_data.year.into(),
                participant_data.name.into(),
                participant_data.language.into(),
                participant_data.repo.into(),
            ]);
        }

        let (sql, values) = query.build_sqlx(PostgresQueryBuilder);

        // probably not the best hack, but sea-query doesn't allow for setting
        // the ON CONSTRAINT expression.
        let sql = sql
            + r#" ON CONFLICT (year, name) DO UPDATE SET
language = excluded.language,
repo = excluded.repo
RETURNING year, name"#;

        let ids = sqlx::query_as_with::<_, (i32, String), _>(&sql, values)
            .fetch_all(db)
            .await?;

        Ok(ids)
    }

    pub async fn list(mm: &ModelManager, filter: ParticipantFilter) -> Result<Vec<Participant>> {
        bmc_list::<Self, _, _>(mm, filter).await
    }

    pub async fn get(mm: &ModelManager, year: i32, name: &str) -> Result<Participant> {
        let db = mm.db();

        let (sql, values) = Query::select()
            .columns(ParticipantIden::iter())
            .from(ParticipantIden::Table)
            .and_where(Expr::col(ParticipantIden::Year).eq(year))
            .and_where(Expr::col(ParticipantIden::Name).eq(name))
            .build_sqlx(PostgresQueryBuilder);

        let entity = sqlx::query_as_with(&sql, values)
            .fetch_optional(db)
            .await?
            .ok_or(Error::EntityNotFound {
                table: ParticipantBmc::TABLE,
                id: year,
            })?;

        Ok(entity)
    }
}

#[derive(Debug, Default, Clone, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
pub struct ParticipantFilter {
    pub year: Option<i32>,
    pub name: Option<String>,
    pub language: Option<String>,
}

impl From<ParticipantFilter> for Cond {
    fn from(value: ParticipantFilter) -> Self {
        let mut cond = Cond::all();

        // there has to be a better way to do this, and there probably is
        if let Some(year) = value.year {
            cond = cond.add(Expr::col(ParticipantIden::Year).eq(year));
        }

        if let Some(name) = value.name {
            cond = cond.add(Expr::col(ParticipantIden::Name).eq(name));
        }

        if let Some(language) = value.language {
            cond = cond.add(Expr::col(ParticipantIden::Language).eq(language));
        }

        cond
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test]
    async fn test_create_batch_ok(pool: sqlx::PgPool) -> anyhow::Result<()> {
        let mm = ModelManager::from(pool);

        let data = vec![
            Participant {
                year: 2023,
                name: "foo".into(),
                language: "ruby".into(),
                repo: "https://foobar/foo2".into(),
            },
            Participant {
                year: 2023,
                name: "bar".into(),
                language: "python".into(),
                repo: "https://foobar/bar".into(),
            },
            // this should be valid because the year is different
            Participant {
                year: 2022,
                name: "foo".into(),
                language: "rust".into(),
                repo: "https://foobar/foo".into(),
            },
        ];

        let ids = ParticipantBmc::batch_create_or_update(&mm, data.clone()).await?;

        assert_eq!(
            ids,
            vec![
                (2023, "foo".to_string()),
                (2023, "bar".to_string()),
                (2022, "foo".to_string()),
            ]
        );

        let participants = ParticipantBmc::list(&mm, ParticipantFilter::default()).await?;

        assert_eq!(participants[0], data[0]);
        assert_eq!(participants[1], data[1]);
        assert_eq!(participants[2], data[2]);

        // now we're going to change one of them while making a new one
        let data = vec![
            Participant {
                year: 2023,
                name: "foo".into(),
                language: "ruby".into(),
                repo: "https://foobar/foo3".into(),
            },
            // this should be valid because the year is different
            Participant {
                year: 2022,
                name: "bar".into(),
                language: "php".into(),
                repo: "https://foobar/bar2".into(),
            },
        ];

        let ids = ParticipantBmc::batch_create_or_update(&mm, data.clone()).await?;

        assert_eq!(
            ids,
            vec![(2023, "foo".to_string()), (2022, "bar".to_string()),]
        );

        let mut participants = ParticipantBmc::list(&mm, ParticipantFilter::default()).await?;
        participants.sort_by(|a, b| a.year.cmp(&b.year).then_with(|| a.name.cmp(&b.name)));

        assert_eq!(participants[0], data[1]);
        assert_eq!(participants[3], data[0]);

        Ok(())
    }

    #[sqlx::test(fixtures("../../fixtures/participants.sql"))]
    async fn test_list_ok(pool: sqlx::PgPool) -> anyhow::Result<()> {
        let mm = ModelManager::from(pool);

        let entities = ParticipantBmc::list(&mm, ParticipantFilter::default()).await?;

        assert_eq!(entities.len(), 3);

        Ok(())
    }

    #[sqlx::test]
    async fn test_list_empty_ok(pool: sqlx::PgPool) -> anyhow::Result<()> {
        let mm = ModelManager::from(pool);

        let entities = ParticipantBmc::list(&mm, ParticipantFilter::default()).await?;

        assert_eq!(entities.len(), 0);

        Ok(())
    }

    #[sqlx::test(fixtures("../../fixtures/participants.sql"))]
    async fn test_list_filtered_ok(pool: sqlx::PgPool) -> anyhow::Result<()> {
        let mm = ModelManager::from(pool);

        let entities = ParticipantBmc::list(
            &mm,
            ParticipantFilter {
                name: Some("bar".into()),
                ..Default::default()
            },
        )
        .await?;

        assert_eq!(entities.len(), 1);

        Ok(())
    }
}
