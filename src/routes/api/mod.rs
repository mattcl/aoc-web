use axum::Router;

use crate::server::AppState;

mod benchmarks;
mod participants;
mod summaries;

pub fn routes(state: AppState) -> Router {
    Router::new().nest("/api/v1", v1(state))
}

fn v1(state: AppState) -> Router {
    Router::new()
        .nest("/benchmarks", benchmarks::routes(state.clone()))
        .nest("/participants", participants::routes(state.clone()))
        .nest("/summaries", summaries::routes(state.clone()))
}
