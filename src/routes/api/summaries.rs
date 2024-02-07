use axum::{
    extract::{Path, Query, State},
    middleware,
    routing::{get, post},
    Json, Router,
};

use crate::{
    middleware::mw_require_auth,
    models::{BenchmarkBmc, BenchmarkFilter, ModelManager, Summary, SummaryBmc, SummaryFilter},
    server::AppState,
    Result,
};

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/generate", post(generate_summaries))
        .layer(middleware::from_fn(mw_require_auth))
        .route("/", get(list_summaies))
        .route("/:year/:participant", get(get_summary))
        .with_state(state)
}

async fn generate_summaries(
    State(mm): State<ModelManager>,
    Json(payload): Json<i32>,
) -> Result<Json<Vec<(i32, String)>>> {
    let filter = BenchmarkFilter {
        year: Some(payload),
        ..Default::default()
    };

    tracing::debug!("finding benchmarks for {payload}");
    let benchmarks = BenchmarkBmc::list(&mm, filter).await?;

    // if we didn't find anything, don't bother doing the next steps
    if benchmarks.is_empty() {
        tracing::debug!("no benchmarks found");
        return Ok(Json(Vec::new()));
    }

    let num_benches = benchmarks.len();

    tracing::debug!("generating summaries from {} benchmarks", num_benches);
    let summaries = Summary::from_benchmarks(benchmarks)?;
    tracing::debug!(
        "generated {} summaries from {} benchmarks",
        summaries.len(),
        num_benches
    );

    Ok(Json(
        SummaryBmc::batch_create_or_update(&mm, summaries).await?,
    ))
}

async fn list_summaies(
    State(mm): State<ModelManager>,
    Query(filter): Query<SummaryFilter>,
) -> Result<Json<Vec<Summary>>> {
    Ok(Json(SummaryBmc::list(&mm, filter).await?))
}

async fn get_summary(
    State(mm): State<ModelManager>,
    Path((year, participant)): Path<(i32, String)>,
) -> Result<Json<Summary>> {
    Ok(Json(SummaryBmc::get(&mm, year, &participant).await?))
}

#[cfg(test)]
mod tests {
    use axum::{
        body::{to_bytes, Body},
        http::{self, Request, StatusCode},
    };
    use tower::ServiceExt;

    use super::*;

    // this fixture being benchmarks is intentional
    #[sqlx::test(fixtures("../../../fixtures/benchmarks.sql"))]
    async fn test_generate_ok(pool: sqlx::PgPool) -> anyhow::Result<()> {
        let state = AppState::new(ModelManager::from(pool));

        // check no summaries
        let routes = super::routes(state.clone());

        let response = routes
            .oneshot(Request::builder().uri("/").body(Body::empty())?)
            .await?;

        assert_eq!(response.status(), StatusCode::OK);

        let raw_body = to_bytes(response.into_body(), usize::MAX).await?;
        let body: Vec<Summary> = serde_json::from_slice(&raw_body)?;
        assert_eq!(body.len(), 0);

        // generate
        let routes = super::routes(state.clone());

        let response = routes
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/generate")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .header(http::header::AUTHORIZATION, "Bearer sandcastle")
                    .body(Body::from(serde_json::to_vec(&2023)?))?,
            )
            .await?;

        assert_eq!(response.status(), StatusCode::OK);

        // check two summaries
        let routes = super::routes(state.clone());

        let response = routes
            .oneshot(Request::builder().uri("/").body(Body::empty())?)
            .await?;

        assert_eq!(response.status(), StatusCode::OK);

        let raw_body = to_bytes(response.into_body(), usize::MAX).await?;
        let body: Vec<Summary> = serde_json::from_slice(&raw_body)?;
        assert_eq!(body.len(), 2);

        Ok(())
    }

    #[sqlx::test]
    async fn test_generate_requires_auth(pool: sqlx::PgPool) -> anyhow::Result<()> {
        let state = AppState::new(ModelManager::from(pool));

        let routes = super::routes(state.clone());

        let response = routes
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/generate")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .body(Body::from(serde_json::to_vec(&2023)?))?,
            )
            .await?;

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        // and with wrong token
        let routes = super::routes(state.clone());

        let response = routes
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/generate")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .header(http::header::AUTHORIZATION, "Bearer wrong")
                    .body(Body::from(serde_json::to_vec(&2023)?))?,
            )
            .await?;

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        Ok(())
    }

    #[sqlx::test(fixtures("../../../fixtures/summaries.sql"))]
    async fn test_list_ok(pool: sqlx::PgPool) -> anyhow::Result<()> {
        let state = AppState::new(ModelManager::from(pool));
        let routes = super::routes(state);

        let response = routes
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await?;

        assert_eq!(response.status(), StatusCode::OK);

        let raw_body = to_bytes(response.into_body(), usize::MAX).await?;
        let body: Vec<Summary> = serde_json::from_slice(&raw_body)?;
        assert_eq!(body.len(), 2);

        Ok(())
    }

    #[sqlx::test]
    async fn test_list_empty_ok(pool: sqlx::PgPool) -> anyhow::Result<()> {
        let state = AppState::new(ModelManager::from(pool));
        let routes = super::routes(state);

        let response = routes
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await?;

        assert_eq!(response.status(), StatusCode::OK);

        let raw_body = to_bytes(response.into_body(), usize::MAX).await?;
        let body: Vec<Summary> = serde_json::from_slice(&raw_body)?;
        assert_eq!(body.len(), 0);

        Ok(())
    }

    #[sqlx::test(fixtures("../../../fixtures/summaries.sql"))]
    async fn test_list_filtered_ok(pool: sqlx::PgPool) -> anyhow::Result<()> {
        let state = AppState::new(ModelManager::from(pool));
        let routes = super::routes(state);

        let response = routes
            .oneshot(
                Request::builder()
                    .uri("/?year=2023")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await?;

        assert_eq!(response.status(), StatusCode::OK);

        let raw_body = to_bytes(response.into_body(), usize::MAX).await?;

        let body: Vec<Summary> = serde_json::from_slice(&raw_body)?;
        assert_eq!(body.len(), 1);

        Ok(())
    }
}
