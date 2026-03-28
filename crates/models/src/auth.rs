//! 认证相关模型

use serde::{Deserialize, Serialize};

/// JWT Claims (Token 载荷)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// 用户ID
    pub sub: i64,
    /// 用户名
    pub username: String,
    /// 角色列表
    pub roles: Vec<String>,
    /// 权限列表
    pub permissions: Vec<String>,
    /// 过期时间 (Unix timestamp)
    pub exp: usize,
    /// 签发时间 (Unix timestamp)
    pub iat: usize,
}

/// 登录响应
#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    /// JWT Token
    pub token: String,
    /// Token 类型
    pub token_type: String,
    /// 过期时间（秒）
    pub expires_in: i64,
    /// 用户信息
    pub user: UserInfo,
}

/// 用户信息（用于登录响应）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: i64,
    pub username: String,
    pub email: Option<String>,
    pub real_name: Option<String>,
    pub avatar: Option<String>,
    pub roles: Vec<RoleBrief>,
}

/// 角色简要信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleBrief {
    pub id: i64,
    pub name: String,
    pub code: String,
}

/// JWT 配置
#[derive(Debug, Clone)]
pub struct JwtConfig {
    pub secret: String,
    pub expires_in: i64, // 秒
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            secret: "cicierp-default-secret-change-in-production".to_string(),
            expires_in: 24 * 60 * 60, // 24小时
        }
    }
}

impl JwtConfig {
    pub fn from_env() -> Self {
        Self {
            secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "cicierp-default-secret-change-in-production".to_string()),
            expires_in: std::env::var("JWT_EXPIRES_IN")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(24 * 60 * 60),
        }
    }
}
