use crate::{config, models::ModelManager, routes};
use axum::{extract::FromRef, http::Method, Router};
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Debug, Clone, FromRef)]
pub struct AppState {
    mm: ModelManager,
}

#[cfg(test)]
impl AppState {
    pub fn new(mm: ModelManager) -> Self {
        Self { mm }
    }
}

pub async fn serve() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "aoc_web=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let state = AppState {
        mm: ModelManager::new().await?,
    };

    tracing::info!("initialized app state");

    // attempt to apply migrations
    tracing::info!("applying migrations");
    state.mm.migrate().await?;

    let socket_addr = config().socket_addr();
    tracing::info!("listening on {}", &socket_addr);

    axum::Server::bind(&socket_addr)
        .serve(service(state).into_make_service())
        .await?;

    Ok(())
}

// separated to allow testing without the server, and not allow for not reaching
// into the router module directly by an extenal caller
pub fn service(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_origin(Any);

    routes::router(state).layer(cors)
}
