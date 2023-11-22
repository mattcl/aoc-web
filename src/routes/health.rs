use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use clap::crate_version;
use serde::Serialize;

use crate::{models::ModelManager, server::AppState};

const VERSION: &str = crate_version!();

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .with_state(state)
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum HealthStatus {
    Ok,
    Degraded,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Health {
    version: &'static str,
    status: HealthStatus,
}

async fn health(State(mm): State<ModelManager>) -> impl IntoResponse {
    let mut h = Health {
        version: VERSION,
        status: HealthStatus::Ok,
    };

    if let Err(e) = mm.check_connectivity().await {
        h.status = HealthStatus::Error;
        tracing::error!("Cannot reach database: {:?}", e);
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(h));
    }

    (StatusCode::OK, Json(h))
}
