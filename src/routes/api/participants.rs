use axum::{
    extract::{Path, Query, State},
    middleware,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;

use crate::{
    middleware::mw_require_auth,
    models::{ModelManager, Participant, ParticipantBmc, ParticipantFilter},
    server::AppState,
    Result,
};

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/", post(create_participants))
        .layer(middleware::from_fn(mw_require_auth))
        .route("/", get(list_participants))
        .route("/:year/:participant", get(get_participant))
        .with_state(state)
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum CreateRequest {
    One(Participant),
    Many(Vec<Participant>),
}

async fn create_participants(
    State(mm): State<ModelManager>,
    Json(payload): Json<CreateRequest>,
) -> Result<Json<Vec<(i32, String)>>> {
    let data = match payload {
        CreateRequest::One(v) => vec![v],
        CreateRequest::Many(v) => v,
    };
    Ok(Json(
        ParticipantBmc::batch_create_or_update(&mm, data).await?,
    ))
}

async fn list_participants(
    State(mm): State<ModelManager>,
    Query(filter): Query<ParticipantFilter>,
) -> Result<Json<Vec<Participant>>> {
    Ok(Json(ParticipantBmc::list(&mm, filter).await?))
}

async fn get_participant(
    State(mm): State<ModelManager>,
    Path((year, name)): Path<(i32, String)>,
) -> Result<Json<Participant>> {
    Ok(Json(ParticipantBmc::get(&mm, year, &name).await?))
}

#[cfg(test)]
mod tests {
    use axum::{
        body::{to_bytes, Body},
        http::{self, Request, StatusCode},
    };
    use tower::ServiceExt;

    use super::*;

    #[sqlx::test(fixtures("../../../fixtures/participants.sql"))]
    async fn test_create_ok(pool: sqlx::PgPool) -> anyhow::Result<()> {
        let state = AppState::new(ModelManager::from(pool));

        let routes = super::routes(state.clone());

        let create_args = Participant {
            year: 2023,
            name: "herp".into(),
            language: "rust".into(),
            repo: "https://bar/baz".into(),
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
        let body: Vec<(i32, String)> = serde_json::from_slice(&raw_body)?;
        assert_eq!(body.len(), 1);
        assert_eq!(body, vec![(2023, "herp".to_string())]);

        // we need to get that created object back
        let routes = super::routes(state);
        let response = routes
            .oneshot(
                Request::builder()
                    .uri("/2023/herp")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await?;

        assert_eq!(response.status(), StatusCode::OK);

        let raw_body = to_bytes(response.into_body(), usize::MAX).await?;
        let body: Participant = serde_json::from_slice(&raw_body)?;
        assert_eq!(body, create_args);

        Ok(())
    }

    #[sqlx::test]
    async fn test_create_batch_ok(pool: sqlx::PgPool) -> anyhow::Result<()> {
        let state = AppState::new(ModelManager::from(pool));

        let routes = super::routes(state.clone());

        let create_args = vec![
            Participant {
                year: 2023,
                name: "herp".into(),
                language: "rust".into(),
                repo: "https://bar/baz".into(),
            },
            Participant {
                year: 2023,
                name: "derp".into(),
                language: "java".into(),
                repo: "https://bar/doof".into(),
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
        let body: Vec<(i32, String)> = serde_json::from_slice(&raw_body)?;
        assert_eq!(body.len(), 2);
        assert_eq!(
            body,
            vec![(2023, "herp".to_string()), (2023, "derp".to_string()),]
        );

        Ok(())
    }

    #[sqlx::test]
    async fn test_create_requires_auth(pool: sqlx::PgPool) -> anyhow::Result<()> {
        let state = AppState::new(ModelManager::from(pool));

        let routes = super::routes(state.clone());

        let create_args = Participant {
            year: 2023,
            name: "herp".into(),
            language: "rust".into(),
            repo: "https://bar/baz".into(),
        };

        let response = routes
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .header(http::header::AUTHORIZATION, "Bearer foo")
                    .body(Body::from(serde_json::to_vec(&create_args)?))
                    .unwrap(),
            )
            .await?;

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        Ok(())
    }

    #[sqlx::test(fixtures("../../../fixtures/participants.sql"))]
    async fn test_list_ok(pool: sqlx::PgPool) -> anyhow::Result<()> {
        let state = AppState::new(ModelManager::from(pool));

        let routes = super::routes(state.clone());

        let response = routes
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await?;

        assert_eq!(response.status(), StatusCode::OK);

        let raw_body = to_bytes(response.into_body(), usize::MAX).await?;
        let body: Vec<Participant> = serde_json::from_slice(&raw_body)?;
        // idk if there's a better way to test that we get what we want
        assert_eq!(body.len(), 3);

        Ok(())
    }

    #[sqlx::test]
    async fn test_list_empty_ok(pool: sqlx::PgPool) -> anyhow::Result<()> {
        let state = AppState::new(ModelManager::from(pool));

        let routes = super::routes(state.clone());

        let response = routes
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await?;

        assert_eq!(response.status(), StatusCode::OK);

        let raw_body = to_bytes(response.into_body(), usize::MAX).await?;
        let body: Vec<Participant> = serde_json::from_slice(&raw_body)?;
        // idk if there's a better way to test that we get what we want
        assert_eq!(body.len(), 0);

        Ok(())
    }

    #[sqlx::test(fixtures("../../../fixtures/participants.sql"))]
    async fn test_list_filterd_ok(pool: sqlx::PgPool) -> anyhow::Result<()> {
        let state = AppState::new(ModelManager::from(pool));
        let routes = routes(state);

        let response = routes
            .oneshot(
                Request::builder()
                    .uri("/?year=2022")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await?;

        assert_eq!(response.status(), StatusCode::OK);

        let raw_body = to_bytes(response.into_body(), usize::MAX).await?;
        let body: Vec<Participant> = serde_json::from_slice(&raw_body)?;
        // idk if there's a better way to test that we get what we want
        assert_eq!(body.len(), 1);

        Ok(())
    }

    #[sqlx::test(fixtures("../../../fixtures/participants.sql"))]
    async fn test_get_ok(pool: sqlx::PgPool) -> anyhow::Result<()> {
        let state = AppState::new(ModelManager::from(pool));
        let routes = routes(state);

        let response = routes
            .oneshot(
                Request::builder()
                    .uri("/2022/foo")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await?;

        assert_eq!(response.status(), StatusCode::OK);

        let raw_body = to_bytes(response.into_body(), usize::MAX).await?;
        let body: Participant = serde_json::from_slice(&raw_body)?;
        // idk if there's a better way to test this
        assert_eq!(body.name, "foo".to_string());

        Ok(())
    }

    #[sqlx::test]
    async fn test_get_not_found(pool: sqlx::PgPool) -> anyhow::Result<()> {
        let state = AppState::new(ModelManager::from(pool));
        let routes = routes(state);

        let response = routes
            .oneshot(
                Request::builder()
                    .uri("/2025/bar")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await?;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        Ok(())
    }
}
