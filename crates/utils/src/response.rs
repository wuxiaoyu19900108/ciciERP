//! 统一响应格式

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

use crate::error::AppError;

/// 统一 API 响应
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub code: u16,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    pub timestamp: i64,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            code: 200,
            message: "success".to_string(),
            data: Some(data),
            timestamp: chrono::Utc::now().timestamp(),
        }
    }

    pub fn success_message(message: &str) -> Self {
        Self {
            code: 200,
            message: message.to_string(),
            data: None,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
}

impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Response {
        let status = if self.code >= 200 && self.code < 300 {
            StatusCode::OK
        } else if self.code >= 400 && self.code < 500 {
            StatusCode::BAD_REQUEST
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        };

        (status, Json(self)).into_response()
    }
}

/// 空响应
pub type EmptyResponse = ApiResponse<()>;

impl AppError {
    pub fn into_api_response<T>(self) -> ApiResponse<T> {
        ApiResponse {
            code: self.code(),
            message: self.message(),
            data: None,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
}
