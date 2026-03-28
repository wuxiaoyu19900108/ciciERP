//! 错误处理

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;
use validator::ValidationErrors;

/// 应用错误类型
#[derive(Debug, Error)]
pub enum AppError {
    #[error("资源未找到")]
    NotFound,

    #[error("请求参数错误: {0}")]
    BadRequest(String),

    #[error("验证失败: {0}")]
    ValidationError(String),

    #[error("数据库错误: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("内部错误: {0}")]
    InternalError(#[from] anyhow::Error),

    #[error("未授权")]
    Unauthorized,

    #[error("禁止访问")]
    Forbidden,

    #[error("资源已存在: {0}")]
    Conflict(String),
}

impl AppError {
    pub fn code(&self) -> u16 {
        match self {
            AppError::NotFound => 404,
            AppError::BadRequest(_) => 400,
            AppError::ValidationError(_) => 422,
            AppError::DatabaseError(_) => 500,
            AppError::InternalError(_) => 500,
            AppError::Unauthorized => 401,
            AppError::Forbidden => 403,
            AppError::Conflict(_) => 409,
        }
    }

    pub fn message(&self) -> String {
        self.to_string()
    }
}

impl From<ValidationErrors> for AppError {
    fn from(err: ValidationErrors) -> Self {
        AppError::ValidationError(err.to_string())
    }
}

/// 错误响应
#[derive(Serialize)]
struct ErrorResponse {
    code: u16,
    message: String,
    timestamp: i64,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match self {
            AppError::NotFound => StatusCode::NOT_FOUND,
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::ValidationError(_) => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::DatabaseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Unauthorized => StatusCode::UNAUTHORIZED,
            AppError::Forbidden => StatusCode::FORBIDDEN,
            AppError::Conflict(_) => StatusCode::CONFLICT,
        };

        let body = ErrorResponse {
            code: self.code(),
            message: self.message(),
            timestamp: chrono::Utc::now().timestamp(),
        };

        (status, Json(body)).into_response()
    }
}

/// Result 类型别名
pub type AppResult<T> = Result<T, AppError>;
