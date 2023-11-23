use axum::{http::StatusCode, response::IntoResponse};

use crate::models;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Unauthorized")]
    MissingAuthHeader,

    #[error("Unauthorized")]
    Unauthorized,

    #[error(transparent)]
    ModelError(#[from] models::Error),

    #[error(transparent)]
    Unhandled(#[from] anyhow::Error),
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::MissingAuthHeader | Self::Unauthorized => {
                tracing::debug!("{self:?}");
                (StatusCode::UNAUTHORIZED, "Unauthorized").into_response()
            }
            Self::ModelError(models::Error::EntityNotFound { .. }) => {
                tracing::debug!("{self:?}");
                (StatusCode::NOT_FOUND, "Not found").into_response()
            }
            Self::ModelError(models::Error::EmptyBatch(_)) => {
                tracing::debug!("{self:?}");
                (StatusCode::BAD_REQUEST, "Empty batch").into_response()
            }
            _ => {
                tracing::error!("{self:?}");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
            }
        }
    }
}
