//! 用户模型

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;

/// 用户状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(rename_all = "lowercase")]
pub enum UserStatus {
    #[serde(rename = "1")]
    Active = 1,
    #[serde(rename = "2")]
    Disabled = 2,
}

impl Default for UserStatus {
    fn default() -> Self {
        UserStatus::Active
    }
}

/// 用户实体
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: i64,
    pub username: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub email: Option<String>,
    pub mobile: Option<String>,
    pub real_name: Option<String>,
    pub avatar: Option<String>,
    pub status: i64,
    pub last_login_at: Option<String>,
    pub last_login_ip: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<String>,
}

/// 用户简要信息（不含敏感数据）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: i64,
    pub username: String,
    pub email: Option<String>,
    pub mobile: Option<String>,
    pub real_name: Option<String>,
    pub avatar: Option<String>,
    pub status: i64,
    pub roles: Vec<RoleInfo>,
}

impl From<User> for UserInfo {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            username: user.username,
            email: user.email,
            mobile: user.mobile,
            real_name: user.real_name,
            avatar: user.avatar,
            status: user.status,
            roles: vec![],
        }
    }
}

/// 角色信息
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RoleInfo {
    pub id: i64,
    pub name: String,
    pub code: String,
}

/// 角色实体
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Role {
    pub id: i64,
    pub name: String,
    pub code: String,
    pub description: Option<String>,
    pub permissions: String,
    pub status: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// 登录请求
#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(length(min = 1, message = "用户名不能为空"))]
    pub username: String,
    #[validate(length(min = 6, message = "密码长度至少6位"))]
    pub password: String,
}

/// 创建用户请求
#[derive(Debug, Deserialize, Validate)]
pub struct CreateUserRequest {
    #[validate(length(min = 2, max = 50, message = "用户名长度2-50"))]
    pub username: String,
    #[validate(length(min = 6, message = "密码长度至少6位"))]
    pub password: String,
    #[validate(email(message = "邮箱格式不正确"))]
    pub email: Option<String>,
    #[validate(length(min = 6, max = 20, message = "手机号长度6-20"))]
    pub mobile: Option<String>,
    pub real_name: Option<String>,
    pub avatar: Option<String>,
    #[validate(range(min = 1, max = 2))]
    pub status: Option<i64>,
    pub role_ids: Option<Vec<i64>>,
}

/// 更新用户请求
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateUserRequest {
    #[validate(email(message = "邮箱格式不正确"))]
    pub email: Option<String>,
    #[validate(length(min = 6, max = 20, message = "手机号长度6-20"))]
    pub mobile: Option<String>,
    pub real_name: Option<String>,
    pub avatar: Option<String>,
    #[validate(range(min = 1, max = 2))]
    pub status: Option<i64>,
    pub role_ids: Option<Vec<i64>>,
}

/// 修改密码请求
#[derive(Debug, Deserialize, Validate)]
pub struct ChangePasswordRequest {
    #[validate(length(min = 6, message = "旧密码长度至少6位"))]
    pub old_password: String,
    #[validate(length(min = 6, message = "新密码长度至少6位"))]
    pub new_password: String,
}

/// 用户查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct UserQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub keyword: Option<String>,
    pub status: Option<i64>,
}

impl UserQuery {
    pub fn page(&self) -> u32 {
        self.page.unwrap_or(1).max(1)
    }

    pub fn page_size(&self) -> u32 {
        self.page_size.unwrap_or(20).min(100).max(1)
    }
}

/// 重置密码请求
#[derive(Debug, Deserialize, Validate)]
pub struct ResetPasswordRequest {
    #[validate(length(min = 6, message = "新密码长度至少6位"))]
    pub new_password: String,
}
