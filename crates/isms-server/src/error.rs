//! RFC 9457 problem details (TDD 10.1). Engine rejections carry their
//! `RejectCode` in `code`.

use crate::actor::ActorError;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use isms_api_types::Problem;
use isms_core::command::Reject;
use isms_store::StoreError;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("{0}")]
    BadRequest(String),
    #[error("sign in required")]
    Unauthorized,
    #[error("{0}")]
    Forbidden(String),
    #[error("{0}")]
    NotFound(String),
    #[error("too many requests")]
    RateLimited,
    #[error("{}", .0.message)]
    Reject(Reject),
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    Actor(#[from] ActorError),
    #[error("{0}")]
    Internal(String),
}

impl ApiError {
    fn status(&self) -> StatusCode {
        match self {
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::Unauthorized => StatusCode::UNAUTHORIZED,
            ApiError::Forbidden(_) => StatusCode::FORBIDDEN,
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            ApiError::Reject(_) => StatusCode::UNPROCESSABLE_ENTITY,
            ApiError::Actor(_) => StatusCode::SERVICE_UNAVAILABLE,
            ApiError::Store(_) | ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// The problem document; internal errors are not described to clients.
    pub fn problem(&self) -> Problem {
        let status = self.status();
        let (detail, code) = match self {
            ApiError::Reject(r) => (Some(r.message.clone()), Some(r.code)),
            ApiError::Store(_) | ApiError::Internal(_) | ApiError::Actor(_) => (None, None),
            other => (Some(other.to_string()), None),
        };
        Problem {
            type_: "about:blank".into(),
            title: status.canonical_reason().unwrap_or("error").to_owned(),
            status: status.as_u16(),
            detail,
            code,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match &self {
            ApiError::Store(e) => tracing::error!("store: {e}"),
            ApiError::Actor(e) => tracing::error!("actor: {e}"),
            ApiError::Internal(e) => tracing::error!("internal: {e}"),
            _ => {}
        }
        let status = self.status();
        let body = serde_json::to_vec(&self.problem()).unwrap_or_default();
        let mut response = (status, body).into_response();
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/problem+json"),
        );
        response
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
