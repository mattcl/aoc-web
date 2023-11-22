use axum::Router;
use tower_http::trace::{self, TraceLayer};
use tracing::Level;

use crate::server::AppState;

mod api;
mod health;

pub fn router(state: AppState) -> Router {
    Router::new()
        .merge(api::routes(state.clone()))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(trace::DefaultMakeSpan::new().level(Level::INFO))
                .on_response(trace::DefaultOnResponse::new().level(Level::INFO))
                .on_failure(trace::DefaultOnFailure::new().level(Level::ERROR)),
        )
        // define after tracing so no traces on health routes
        .merge(health::routes(state))
}
