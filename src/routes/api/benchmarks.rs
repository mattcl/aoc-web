use axum::{
    extract::{Path, Query, State},
    middleware,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;

use crate::{
    middleware::mw_require_auth,
    models::{Benchmark, BenchmarkBmc, BenchmarkCreate, BenchmarkFilter, ModelManager},
    server::AppState,
    Result,
};

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/", post(create_benchmarks))
        .layer(middleware::from_fn(mw_require_auth))
        .route("/", get(list_benchmarks))
        .route("/:benchmark", get(get_benchmark))
        .with_state(state)
}

// TODO: maybe this is the wrong place for this, but we want the _endpoint_ to
// accept single _and_ multiple records. This is probably something the
// controller doesn't need to be aware of - MCL - 2023-11-21
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum CreateRequest {
    One(BenchmarkCreate),
    Many(Vec<BenchmarkCreate>),
}

async fn create_benchmarks(
    State(mm): State<ModelManager>,
    Json(payload): Json<CreateRequest>,
) -> Result<Json<Vec<i32>>> {
    let data = match payload {
        CreateRequest::One(v) => vec![v],
        CreateRequest::Many(v) => v,
    };

    Ok(Json(BenchmarkBmc::batch_create_or_update(&mm, data).await?))
}

async fn list_benchmarks(
    State(mm): State<ModelManager>,
    Query(filter): Query<BenchmarkFilter>,
) -> Result<Json<Vec<Benchmark>>> {
    Ok(Json(BenchmarkBmc::list(&mm, filter).await?))
}

async fn get_benchmark(
    State(mm): State<ModelManager>,
    Path(id): Path<i32>,
) -> Result<Json<Benchmark>> {
    Ok(Json(BenchmarkBmc::get(&mm, id).await?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{to_bytes, Body},
        http::{self, Request, StatusCode},
    };
    use std::f64::EPSILON;
    use tower::ServiceExt;

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
    async fn test_create_ok(pool: sqlx::PgPool) -> anyhow::Result<()> {
        let state = AppState::new(ModelManager::from(pool));
        let routes = super::routes(state.clone());

        let create_args = BenchmarkCreate {
            year: 2023,
            day: 12,
            input: "input-foo".into(),
            participant: "foo".into(),
            language: "rust".into(),
            mean: 0.57,
            stddev: 0.07,
            median: 0.52,
            user: 0.44,
            system: 0.13,
            min: 0.20,
            max: 0.71,
        };

        let response = routes
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .header(http::header::AUTHORIZATION, "Bearer sandcastle")
                    .body(Body::from(serde_json::to_vec(&create_args)?))
                    .unwrap(),
            )
            .await?;

        assert_eq!(response.status(), StatusCode::OK);

        let raw_body = to_bytes(response.into_body(), usize::MAX).await?;
        let body: Vec<i32> = serde_json::from_slice(&raw_body)?;
        assert_eq!(body.len(), 1);
        assert_eq!(body, vec![1000]);

        // we need to get that created object back
        let routes = super::routes(state);
        let response = routes
            .oneshot(Request::builder().uri("/1000").body(Body::empty()).unwrap())
            .await?;

        assert_eq!(response.status(), StatusCode::OK);

        let raw_body = to_bytes(response.into_body(), usize::MAX).await?;
        let body: Benchmark = serde_json::from_slice(&raw_body)?;
        assert_benchmarks_equal!(body, create_args);

        Ok(())
    }

    #[sqlx::test]
    async fn test_create_batch_ok(pool: sqlx::PgPool) -> anyhow::Result<()> {
        let state = AppState::new(ModelManager::from(pool));
        let routes = super::routes(state.clone());

        let create_args = vec![
            BenchmarkCreate {
                year: 2023,
                day: 12,
                input: "input-foo".into(),
                participant: "foo".into(),
                language: "rust".into(),
                mean: 0.57,
                stddev: 0.07,
                median: 0.52,
                user: 0.44,
                system: 0.13,
                min: 0.20,
                max: 0.71,
            },
            BenchmarkCreate {
                year: 2023,
                day: 12,
                input: "input-foo".into(),
                participant: "baz".into(),
                language: "python".into(),
                mean: 1.57,
                stddev: 1.07,
                median: 1.52,
                user: 1.44,
                system: 1.13,
                min: 1.20,
                max: 1.71,
            },
        ];

        let response = routes
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .header(http::header::AUTHORIZATION, "Bearer sandcastle")
                    .body(Body::from(serde_json::to_vec(&create_args)?))
                    .unwrap(),
            )
            .await?;

        assert_eq!(response.status(), StatusCode::OK);

        let raw_body = to_bytes(response.into_body(), usize::MAX).await?;
        let body: Vec<i32> = serde_json::from_slice(&raw_body)?;
        assert_eq!(body.len(), 2);
        assert_eq!(body, vec![1000, 1001]);

        // we want to get those objects back, so just list
        let routes = super::routes(state);
        let response = routes
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await?;

        assert_eq!(response.status(), StatusCode::OK);

        let raw_body = to_bytes(response.into_body(), usize::MAX).await?;
        let body: Vec<Benchmark> = serde_json::from_slice(&raw_body)?;
        assert_benchmarks_equal!(body[0], create_args[0]);
        assert_benchmarks_equal!(body[1], create_args[1]);

        Ok(())
    }

    #[sqlx::test]
    async fn test_create_requires_auth(pool: sqlx::PgPool) -> anyhow::Result<()> {
        let state = AppState::new(ModelManager::from(pool));

        let create_args = vec![
            BenchmarkCreate {
                year: 2023,
                day: 12,
                input: "input-foo".into(),
                participant: "foo".into(),
                language: "rust".into(),
                mean: 0.57,
                stddev: 0.07,
                median: 0.52,
                user: 0.44,
                system: 0.13,
                min: 0.20,
                max: 0.71,
            },
            BenchmarkCreate {
                year: 2023,
                day: 12,
                input: "input-foo".into(),
                participant: "baz".into(),
                language: "python".into(),
                mean: 1.57,
                stddev: 1.07,
                median: 1.52,
                user: 1.44,
                system: 1.13,
                min: 1.20,
                max: 1.71,
            },
        ];

        let routes = super::routes(state.clone());

        let response = routes
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .body(Body::from(serde_json::to_vec(&create_args)?))?,
            )
            .await?;

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        // and with wrong token
        let routes = super::routes(state.clone());

        let response = routes
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .header(http::header::AUTHORIZATION, "Bearer wrong")
                    .body(Body::from(serde_json::to_vec(&create_args)?))?,
            )
            .await?;

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        Ok(())
    }

    #[sqlx::test(fixtures("../../../fixtures/benchmarks.sql"))]
    async fn test_list_ok(pool: sqlx::PgPool) -> anyhow::Result<()> {
        let state = AppState::new(ModelManager::from(pool));
        let routes = routes(state);

        let response = routes
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await?;

        assert_eq!(response.status(), StatusCode::OK);

        let raw_body = to_bytes(response.into_body(), usize::MAX).await?;
        let body: Vec<Benchmark> = serde_json::from_slice(&raw_body)?;
        // idk if there's a better way to test that we get what we want
        assert_eq!(body.len(), 4);

        Ok(())
    }

    #[sqlx::test]
    async fn test_list_empty_ok(pool: sqlx::PgPool) -> anyhow::Result<()> {
        let state = AppState::new(ModelManager::from(pool));
        let routes = routes(state);

        let response = routes
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await?;

        assert_eq!(response.status(), StatusCode::OK);

        let raw_body = to_bytes(response.into_body(), usize::MAX).await?;
        let body: Vec<Benchmark> = serde_json::from_slice(&raw_body)?;
        // idk if there's a better way to test that we get what we want
        assert!(body.is_empty());

        Ok(())
    }

    #[sqlx::test(fixtures("../../../fixtures/benchmarks.sql"))]
    async fn test_list_filterd_ok(pool: sqlx::PgPool) -> anyhow::Result<()> {
        let state = AppState::new(ModelManager::from(pool));
        let routes = routes(state);

        let response = routes
            .oneshot(
                Request::builder()
                    .uri("/?input=input-bar&participant=bar")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await?;

        assert_eq!(response.status(), StatusCode::OK);

        let raw_body = to_bytes(response.into_body(), usize::MAX).await?;
        let body: Vec<Benchmark> = serde_json::from_slice(&raw_body)?;
        // idk if there's a better way to test that we get what we want
        assert_eq!(body.len(), 1);

        Ok(())
    }

    #[sqlx::test(fixtures("../../../fixtures/benchmarks.sql"))]
    async fn test_get_ok(pool: sqlx::PgPool) -> anyhow::Result<()> {
        let state = AppState::new(ModelManager::from(pool));
        let routes = routes(state);

        let response = routes
            .oneshot(Request::builder().uri("/1001").body(Body::empty()).unwrap())
            .await?;

        assert_eq!(response.status(), StatusCode::OK);

        let raw_body = to_bytes(response.into_body(), usize::MAX).await?;
        let body: Benchmark = serde_json::from_slice(&raw_body)?;
        // idk if there's a better way to test this
        assert_eq!(body.id, 1001);

        Ok(())
    }

    #[sqlx::test]
    async fn test_get_not_found(pool: sqlx::PgPool) -> anyhow::Result<()> {
        let state = AppState::new(ModelManager::from(pool));
        let routes = routes(state);

        let response = routes
            .oneshot(Request::builder().uri("/50").body(Body::empty()).unwrap())
            .await?;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        Ok(())
    }
}
