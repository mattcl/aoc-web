use axum::{
    headers::{authorization::Bearer, Authorization},
    http::Request,
    middleware::Next,
    response::Response,
    TypedHeader,
};

use crate::{config, Error, Result};

pub async fn mw_require_auth<B>(
    bearer: Option<TypedHeader<Authorization<Bearer>>>,
    req: Request<B>,
    next: Next<B>,
) -> Result<Response> {
    tracing::debug!("authenticating");
    if let Some(bearer) = bearer {
        let token = bearer.token();

        if !config().secret.validate(token) {
            return Err(Error::Unauthorized);
        }
    } else {
        return Err(Error::MissingAuthHeader);
    }

    Ok(next.run(req).await)
}
