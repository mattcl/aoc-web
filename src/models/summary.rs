use std::collections::HashMap;

use sea_query::{Cond, Expr, Iden, PostgresQueryBuilder, Query};
use sea_query_binder::SqlxBinder;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use strum::{EnumIter, IntoEnumIterator};

use super::{
    base::{bmc_list, DbBmc},
    Benchmark, Error, ModelManager, Result,
};

// So yeah, this layout is maybe not ideal, but since there's a fixed number of
// days this at least allows simpler queries asking about specific days that if
// we stored as json or something
#[derive(Debug, Default, Clone, PartialEq, PartialOrd, FromRow, Serialize, Deserialize)]
pub struct Summary {
    pub year: i32,
    pub participant: String,
    pub language: String,
    pub day_1: Option<f64>,
    pub day_2: Option<f64>,
    pub day_3: Option<f64>,
    pub day_4: Option<f64>,
    pub day_5: Option<f64>,
    pub day_6: Option<f64>,
    pub day_7: Option<f64>,
    pub day_8: Option<f64>,
    pub day_9: Option<f64>,
    pub day_10: Option<f64>,
    pub day_11: Option<f64>,
    pub day_12: Option<f64>,
    pub day_13: Option<f64>,
    pub day_14: Option<f64>,
    pub day_15: Option<f64>,
    pub day_16: Option<f64>,
    pub day_17: Option<f64>,
    pub day_18: Option<f64>,
    pub day_19: Option<f64>,
    pub day_20: Option<f64>,
    pub day_21: Option<f64>,
    pub day_22: Option<f64>,
    pub day_23: Option<f64>,
    pub day_24: Option<f64>,
    pub day_25: Option<f64>,
    pub total: Option<f64>,
}

impl Summary {
    /// Transforms a set of benchmarks into one or more summaries.
    ///
    /// Care should be taken that the set of benchmarks here is the _complete_
    /// set of benchmarks, as this will have the effect of overwriting existing
    /// summary rows, so partial benchmark sets will cause some summaries to
    /// "lose" data.
    pub fn from_benchmarks(benchmarks: Vec<Benchmark>) -> Result<Vec<Summary>> {
        // we need an intermediate object to use to accumulate
        let mut summaries: HashMap<i32, HashMap<String, SummaryAccumulator>> = HashMap::default();
        for bench in benchmarks {
            let year_group = summaries.entry(bench.year).or_default();

            let acc = year_group
                .entry(bench.participant.clone())
                .or_insert_with(|| SummaryAccumulator {
                    year: bench.year,
                    participant: bench.participant.clone(),
                    language: bench.language.clone(),
                    ..Default::default()
                });

            acc.add(bench)?;
        }

        let mut out = Vec::new();

        for (_, year_group) in summaries {
            for (_, acc) in year_group {
                out.push(acc.into())
            }
        }

        Ok(out)
    }
}

#[derive(Debug, Default, Clone)]
struct SummaryAccumulator {
    year: i32,
    participant: String,
    language: String,
    days: [Vec<f64>; 25],
}

impl SummaryAccumulator {
    fn add(&mut self, benchmark: Benchmark) -> Result<()> {
        if benchmark.day > 25 || benchmark.day < 1 {
            return Err(Error::DayOutOfRange(benchmark.day));
        }

        self.days[benchmark.day as usize - 1].push(benchmark.mean);

        Ok(())
    }

    fn average(&self, day: usize) -> Option<f64> {
        let len = self.days[day].len();
        if len == 0 {
            return None;
        }

        Some(self.days[day].iter().sum::<f64>() / len as f64)
    }
}

impl From<SummaryAccumulator> for Summary {
    fn from(value: SummaryAccumulator) -> Self {
        let averages: Vec<_> = (0..25).map(|day| value.average(day)).collect();
        let raw: Vec<_> = averages.iter().copied().flatten().collect();
        let total = if raw.is_empty() {
            None
        } else {
            Some(raw.into_iter().sum::<f64>())
        };

        Self {
            year: value.year,
            participant: value.participant,
            language: value.language,
            day_1: averages[0],
            day_2: averages[1],
            day_3: averages[2],
            day_4: averages[3],
            day_5: averages[4],
            day_6: averages[5],
            day_7: averages[6],
            day_8: averages[7],
            day_9: averages[8],
            day_10: averages[9],
            day_11: averages[10],
            day_12: averages[11],
            day_13: averages[12],
            day_14: averages[13],
            day_15: averages[14],
            day_16: averages[15],
            day_17: averages[16],
            day_18: averages[17],
            day_19: averages[18],
            day_20: averages[19],
            day_21: averages[20],
            day_22: averages[21],
            day_23: averages[22],
            day_24: averages[23],
            day_25: averages[24],
            total,
        }
    }
}

// this sucks, but we have to wait for a newer version of sea-query to allow
// more control over the struct proc macro
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, Iden, EnumIter)]
pub enum SummaryIden {
    #[iden = "summaries"]
    Table,
    Year,
    Participant,
    Language,
    Day_1,
    Day_2,
    Day_3,
    Day_4,
    Day_5,
    Day_6,
    Day_7,
    Day_8,
    Day_9,
    Day_10,
    Day_11,
    Day_12,
    Day_13,
    Day_14,
    Day_15,
    Day_16,
    Day_17,
    Day_18,
    Day_19,
    Day_20,
    Day_21,
    Day_22,
    Day_23,
    Day_24,
    Day_25,
    Total,
}

pub struct SummaryBmc;

impl DbBmc for SummaryBmc {
    const TABLE: &'static str = "summaries";
    type Iden = SummaryIden;

    fn table_iden() -> Self::Iden {
        Self::Iden::Table
    }
}

impl SummaryBmc {
    // we can just take the whole summary since there's no auto key
    pub async fn batch_create_or_update<Iter>(
        mm: &ModelManager,
        data: Iter,
    ) -> Result<Vec<(i32, String)>>
    where
        Iter: IntoIterator<Item = Summary>,
    {
        let db = mm.db();

        let mut query = Query::insert();

        query
            .into_table(SummaryIden::Table)
            .columns(SummaryIden::iter().filter(|v| !matches!(v, SummaryIden::Table)));

        for summary_data in data {
            query.values_panic([
                summary_data.year.into(),
                summary_data.participant.into(),
                summary_data.language.into(),
                summary_data.day_1.into(),
                summary_data.day_2.into(),
                summary_data.day_3.into(),
                summary_data.day_4.into(),
                summary_data.day_5.into(),
                summary_data.day_6.into(),
                summary_data.day_7.into(),
                summary_data.day_8.into(),
                summary_data.day_9.into(),
                summary_data.day_10.into(),
                summary_data.day_11.into(),
                summary_data.day_12.into(),
                summary_data.day_13.into(),
                summary_data.day_14.into(),
                summary_data.day_15.into(),
                summary_data.day_16.into(),
                summary_data.day_17.into(),
                summary_data.day_18.into(),
                summary_data.day_19.into(),
                summary_data.day_20.into(),
                summary_data.day_21.into(),
                summary_data.day_22.into(),
                summary_data.day_23.into(),
                summary_data.day_24.into(),
                summary_data.day_25.into(),
                summary_data.total.into(),
            ]);
        }

        let (sql, values) = query.build_sqlx(PostgresQueryBuilder);

        // probably not the best hack, but sea-query doesn't allow for setting
        // the ON CONSTRAINT expression.
        let sql = sql
            + r#" ON CONFLICT (year, participant) DO UPDATE SET
language = excluded.language,
day_1 = excluded.day_1,
day_2 = excluded.day_2,
day_3 = excluded.day_3,
day_4 = excluded.day_4,
day_5 = excluded.day_5,
day_6 = excluded.day_6,
day_7 = excluded.day_7,
day_8 = excluded.day_8,
day_9 = excluded.day_9,
day_10 = excluded.day_10,
day_11 = excluded.day_11,
day_12 = excluded.day_12,
day_13 = excluded.day_13,
day_14 = excluded.day_14,
day_15 = excluded.day_15,
day_16 = excluded.day_16,
day_17 = excluded.day_17,
day_18 = excluded.day_18,
day_19 = excluded.day_19,
day_20 = excluded.day_20,
day_21 = excluded.day_21,
day_22 = excluded.day_22,
day_23 = excluded.day_23,
day_24 = excluded.day_24,
day_25 = excluded.day_25,
total = excluded.total
RETURNING year, participant"#;

        let ids = sqlx::query_as_with::<_, (i32, String), _>(&sql, values)
            .fetch_all(db)
            .await?;

        Ok(ids)
    }

    pub async fn list(mm: &ModelManager, filter: SummaryFilter) -> Result<Vec<Summary>> {
        bmc_list::<Self, _, _>(mm, filter).await
    }

    pub async fn get(mm: &ModelManager, year: i32, participant: &str) -> Result<Summary> {
        let db = mm.db();

        let (sql, values) = Query::select()
            .columns(SummaryIden::iter())
            .from(SummaryIden::Table)
            .and_where(Expr::col(SummaryIden::Year).eq(year))
            .and_where(Expr::col(SummaryIden::Participant).eq(participant))
            .build_sqlx(PostgresQueryBuilder);

        let entity = sqlx::query_as_with(&sql, values)
            .fetch_optional(db)
            .await?
            .ok_or(Error::EntityNotFound {
                table: SummaryBmc::TABLE,
                id: year,
            })?;

        Ok(entity)
    }
}

#[derive(Debug, Default, Clone, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
pub struct SummaryFilter {
    pub year: Option<i32>,
    pub participant: Option<String>,
    pub language: Option<String>,
}

impl From<SummaryFilter> for Cond {
    fn from(value: SummaryFilter) -> Self {
        let mut cond = Cond::all();

        // there has to be a better way to do this, and there probably is
        if let Some(year) = value.year {
            cond = cond.add(Expr::col(SummaryIden::Year).eq(year));
        }

        if let Some(participant) = value.participant {
            cond = cond.add(Expr::col(SummaryIden::Participant).eq(participant));
        }

        if let Some(language) = value.language {
            cond = cond.add(Expr::col(SummaryIden::Language).eq(language));
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
            Summary {
                year: 2023,
                participant: "foo".into(),
                language: "ruby".into(),
                day_1: Some(1.0),
                day_2: Some(2.0),
                day_3: Some(3.0),
                day_4: Some(4.0),
                day_5: Some(5.0),
                day_6: Some(6.0),
                day_7: Some(7.0),
                day_8: Some(8.0),
                day_9: Some(9.0),
                day_10: Some(10.0),
                day_11: Some(11.0),
                day_12: Some(12.0),
                day_13: Some(13.0),
                day_14: Some(14.0),
                day_15: Some(15.0),
                day_16: Some(16.0),
                day_17: Some(17.0),
                day_18: Some(18.0),
                day_19: Some(19.0),
                day_20: Some(20.0),
                day_21: Some(21.0),
                day_22: Some(22.0),
                day_23: Some(23.0),
                day_24: Some(24.0),
                day_25: Some(25.0),
                total: Some(325.0),
            },
            Summary {
                year: 2023,
                participant: "bar".into(),
                language: "php".into(),
                day_1: Some(101.0),
                day_2: Some(102.0),
                day_3: Some(103.0),
                day_4: Some(104.0),
                day_5: Some(105.0),
                day_6: Some(106.0),
                day_7: Some(107.0),
                day_8: Some(108.0),
                day_9: Some(109.0),
                day_10: Some(110.0),
                day_11: Some(111.0),
                day_12: Some(112.0),
                day_13: Some(113.0),
                day_14: Some(114.0),
                day_15: Some(115.0),
                day_16: Some(116.0),
                day_17: Some(117.0),
                day_18: Some(118.0),
                day_19: Some(119.0),
                day_20: Some(120.0),
                day_21: Some(121.0),
                day_22: Some(122.0),
                day_23: Some(123.0),
                day_24: Some(124.0),
                day_25: Some(125.0),
                total: Some(2825.0),
            },
        ];

        let ids = SummaryBmc::batch_create_or_update(&mm, data.clone()).await?;

        assert_eq!(
            ids,
            vec![(2023, "foo".to_string()), (2023, "bar".to_string())]
        );

        let summaries = SummaryBmc::list(&mm, SummaryFilter::default()).await?;

        assert_eq!(summaries[0], data[0]);
        assert_eq!(summaries[1], data[1]);

        // now we're going to change one of them while making a new one
        let data2 = vec![
            Summary {
                year: 2023,
                participant: "baz".into(),
                language: "visual basic".into(),
                day_1: Some(301.0),
                day_2: Some(302.0),
                day_3: Some(303.0),
                day_4: Some(304.0),
                day_5: Some(305.0),
                day_6: Some(306.0),
                day_7: Some(307.0),
                day_8: None,
                day_9: Some(309.0),
                day_10: Some(310.0),
                day_11: Some(311.0),
                day_12: Some(312.0),
                day_13: Some(313.0),
                day_14: Some(314.0),
                day_15: Some(315.0),
                day_16: Some(316.0),
                day_17: Some(317.0),
                day_18: Some(318.0),
                day_19: Some(319.0),
                day_20: Some(320.0),
                day_21: Some(321.0),
                day_22: Some(322.0),
                day_23: Some(323.0),
                day_24: Some(324.0),
                day_25: Some(325.0),
                total: Some(7517.0),
            },
            Summary {
                year: 2023,
                participant: "bar".into(),
                language: "php".into(),
                day_1: Some(51.0),
                day_2: Some(52.0),
                day_3: Some(53.0),
                day_4: Some(54.0),
                day_5: Some(55.0),
                day_6: Some(56.0),
                day_7: Some(57.0),
                day_8: Some(58.0),
                day_9: Some(59.0),
                day_10: Some(60.0),
                day_11: Some(61.0),
                day_12: Some(62.0),
                day_13: Some(63.0),
                day_14: Some(64.0),
                day_15: Some(65.0),
                day_16: Some(66.0),
                day_17: Some(67.0),
                day_18: Some(68.0),
                day_19: Some(69.0),
                day_20: Some(70.0),
                day_21: Some(71.0),
                day_22: Some(72.0),
                day_23: Some(73.0),
                day_24: Some(74.0),
                day_25: Some(75.0),
                total: Some(1575.0),
            },
        ];

        let ids = SummaryBmc::batch_create_or_update(&mm, data2.clone()).await?;

        assert_eq!(
            ids,
            vec![(2023, "baz".to_string()), (2023, "bar".to_string())]
        );

        let mut summaries = SummaryBmc::list(&mm, SummaryFilter::default()).await?;
        // yeah, only partial ord
        summaries.sort_by(|a, b| a.total.partial_cmp(&b.total).unwrap());

        assert_eq!(summaries[0], data[0]);
        assert_eq!(summaries[1], data2[1]);
        assert_eq!(summaries[2], data2[0]);

        Ok(())
    }

    #[sqlx::test(fixtures("../../fixtures/summaries.sql"))]
    async fn test_list_ok(pool: sqlx::PgPool) -> anyhow::Result<()> {
        let mm = ModelManager::from(pool);

        let entities = SummaryBmc::list(&mm, SummaryFilter::default()).await?;

        assert_eq!(entities.len(), 2);

        Ok(())
    }

    #[sqlx::test]
    async fn test_list_empty_ok(pool: sqlx::PgPool) -> anyhow::Result<()> {
        let mm = ModelManager::from(pool);

        let entities = SummaryBmc::list(&mm, SummaryFilter::default()).await?;

        assert!(entities.is_empty());

        Ok(())
    }

    #[sqlx::test(fixtures("../../fixtures/summaries.sql"))]
    async fn test_list_filtered_ok(pool: sqlx::PgPool) -> anyhow::Result<()> {
        let mm = ModelManager::from(pool);

        let entities = SummaryBmc::list(
            &mm,
            SummaryFilter {
                participant: Some("foo".into()),
                ..Default::default()
            },
        )
        .await?;

        assert_eq!(entities.len(), 1);

        Ok(())
    }

    #[test]
    fn from_benchmarks() -> anyhow::Result<()> {
        let benchmarks = vec![
            Benchmark {
                year: 2022,
                day: 10,
                input: "input-foo".into(),
                participant: "foo".into(),
                language: "ruby".into(),
                mean: 0.5789,
                // the other ones don't matter for the summary
                ..Default::default()
            },
            Benchmark {
                year: 2022,
                day: 10,
                input: "input-bar".into(),
                participant: "foo".into(),
                language: "ruby".into(),
                mean: 0.7821,
                // the other ones don't matter for the summary
                ..Default::default()
            },
            Benchmark {
                year: 2022,
                day: 2,
                input: "input-foo".into(),
                participant: "bar".into(),
                language: "php".into(),
                mean: 0.234,
                // the other ones don't matter for the summary
                ..Default::default()
            },
            Benchmark {
                year: 2023,
                day: 4,
                input: "input-foo".into(),
                participant: "foo".into(),
                language: "ruby".into(),
                mean: 0.934,
                // the other ones don't matter for the summary
                ..Default::default()
            },
            Benchmark {
                year: 2022,
                day: 5,
                input: "input-baz".into(),
                participant: "foo".into(),
                language: "ruby".into(),
                mean: 0.6551,
                // the other ones don't matter for the summary
                ..Default::default()
            },
        ];

        let mut summaries = Summary::from_benchmarks(benchmarks)?;
        summaries.sort_by(|a, b| {
            a.year
                .cmp(&b.year)
                .then_with(|| a.participant.cmp(&b.participant))
        });

        // we expect 3 of these, 1 for 2022 foo, 1 for 2022 bar and 1 for 2023 foo
        assert_eq!(summaries.len(), 3);

        let expected = vec![
            Summary {
                year: 2022,
                participant: "bar".into(),
                language: "php".into(),
                day_2: Some(0.234),
                total: Some(0.234),
                ..Default::default()
            },
            Summary {
                year: 2022,
                participant: "foo".into(),
                language: "ruby".into(),
                day_5: Some(0.6551),
                day_10: Some((0.7821 + 0.5789) / 2.0),
                total: Some((0.7821 + 0.5789) / 2.0 + 0.6551),
                ..Default::default()
            },
            Summary {
                year: 2023,
                participant: "foo".into(),
                language: "ruby".into(),
                day_4: Some(0.934),
                total: Some(0.934),
                ..Default::default()
            },
        ];

        assert_eq!(summaries, expected);

        Ok(())
    }
}
