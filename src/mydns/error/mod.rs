use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

/// Unified error type for all API handlers.
#[derive(Debug, Error)]
pub enum ApiError {
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Too many requests: {0}")]
    TooManyRequests(String),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            Self::Unauthorized(m) => (StatusCode::UNAUTHORIZED, m.clone()),
            Self::NotFound(m) => (StatusCode::NOT_FOUND, m.clone()),
            Self::BadRequest(m) => (StatusCode::BAD_REQUEST, m.clone()),
            Self::TooManyRequests(m) => (StatusCode::TOO_MANY_REQUESTS, m.clone()),
            Self::Internal(e) => {
                tracing::error!(error = %e, "Unhandled internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
        };
        (status, Json(json!({"error": message}))).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use serde_json::Value;

    async fn assert_response(error: ApiError, status: StatusCode, message: &str) {
        let response = error.into_response();

        assert_eq!(response.status(), status);
        assert!(response
            .headers()
            .get(axum::http::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap()
            .starts_with("application/json"));

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json, json!({"error": message}));
    }

    #[tokio::test]
    async fn unauthorized_response() {
        let error = ApiError::Unauthorized("authentication required".to_string());
        assert_eq!(error.to_string(), "Unauthorized: authentication required");
        assert_response(error, StatusCode::UNAUTHORIZED, "authentication required").await;
    }

    #[tokio::test]
    async fn not_found_response() {
        let error = ApiError::NotFound("zone.example".to_string());
        assert_eq!(error.to_string(), "Not found: zone.example");
        assert_response(error, StatusCode::NOT_FOUND, "zone.example").await;
    }

    #[tokio::test]
    async fn bad_request_response() {
        let error = ApiError::BadRequest("invalid record".to_string());
        assert_eq!(error.to_string(), "Bad request: invalid record");
        assert_response(error, StatusCode::BAD_REQUEST, "invalid record").await;
    }

    #[tokio::test]
    async fn too_many_requests_response() {
        let error = ApiError::TooManyRequests("rate limit exceeded".to_string());
        assert_eq!(error.to_string(), "Too many requests: rate limit exceeded");
        assert_response(error, StatusCode::TOO_MANY_REQUESTS, "rate limit exceeded").await;
    }

    #[tokio::test]
    async fn internal_response_hides_internal_details() {
        let error = ApiError::Internal(anyhow::anyhow!("database connection failed"));
        assert_eq!(error.to_string(), "database connection failed");
        assert_response(
            error,
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal server error",
        )
        .await;
    }
}
