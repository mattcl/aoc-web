use axum::Router;

use crate::server::AppState;

mod benchmarks;

pub fn routes(state: AppState) -> Router {
    Router::new().nest("/api/v1/benchmarks", benchmarks::routes(state.clone()))
}
