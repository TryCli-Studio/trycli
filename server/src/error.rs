use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

/// A specialized Result type for our application
pub type Result<T> = std::result::Result<T, AppError>;

#[derive(thiserror::Error, Debug)]
pub enum AppError {
    // Expected Domain Errors 
    #[error("Authentication failed: {0}")]
    Auth(String),

    #[error("Permission denied: {0}")]
    Forbidden(String),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Validation failed: {0}")]
    ValidationError(String),

    #[error("Rate limit exceeded: {0}")]
    RateLimit(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    // Opaque Catch-all (500)
    // We use anyhow::Error for all unexpected, fatal infrastructure errors.
    // Transparent delegates both `Display`'s and `source`'s implementation 
    // to the type wrapped by `UnexpectedError`.
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match &self {
            AppError::Auth(msg) => (StatusCode::UNAUTHORIZED, msg.clone()),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg.clone()),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::RateLimit(msg) => (StatusCode::TOO_MANY_REQUESTS, msg.clone()),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, msg.clone()),
            AppError::ValidationError(msg) => (StatusCode::UNPROCESSABLE_ENTITY, msg.clone()),
            
            // For unexpected errors, the caller does not understand the intricacies 
            // of the domain, so we return an opaque 500 response.
            AppError::UnexpectedError(_) => {
                // Errors should be logged when they are handled. 
                // Since this error is not propagating any further up the stack, we log it here.
                tracing::error!("{:?}", self);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error".to_string())
            }
        };

        let body = Json(json!({ "error": error_message }));
        (status, body).into_response()
    }
}