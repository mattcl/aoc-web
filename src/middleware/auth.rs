use axum::{body::Body, http::Request, middleware::Next, response::Response};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};

use crate::{config, Error, Result};

pub async fn mw_require_auth(
    bearer: Option<TypedHeader<Authorization<Bearer>>>,
    req: Request<Body>,
    next: Next,
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
