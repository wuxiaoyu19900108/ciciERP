//! API 客户端相关数据模型

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use validator::Validate;

/// API 客户端状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApiClientStatus {
    #[serde(rename = "1")]
    Active = 1,
    #[serde(rename = "0")]
    Inactive = 0,
}

impl Default for ApiClientStatus {
    fn default() -> Self {
        ApiClientStatus::Active
    }
}

/// API 客户端实体
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ApiClient {
    pub id: i64,
    pub client_id: String,
    pub client_name: String,
    pub api_key: String,
    pub api_secret: String,
    pub permissions: JsonValue,
    pub rate_limit: i64,
    pub status: i64,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ApiClient {
    /// 检查是否有指定权限
    pub fn has_permission(&self, permission: &str) -> bool {
        if let JsonValue::Array(perms) = &self.permissions {
            // 通配符权限
            if perms.contains(&JsonValue::String("*".to_string())) {
                return true;
            }
            // 精确匹配
            if perms.contains(&JsonValue::String(permission.to_string())) {
                return true;
            }
            // 模块通配符匹配
            if let Some(colon_pos) = permission.find(':') {
                let module_perm = format!("{}:*", &permission[..colon_pos]);
                if perms.contains(&JsonValue::String(module_perm)) {
                    return true;
                }
            }
        }
        false
    }

    /// 检查客户端是否有效
    pub fn is_active(&self) -> bool {
        self.status == 1
    }
}

/// 创建 API 客户端请求
#[derive(Debug, Deserialize, Validate)]
pub struct CreateApiClientRequest {
    #[validate(length(min = 1, max = 50))]
    pub client_id: String,
    #[validate(length(min = 1, max = 100))]
    pub client_name: String,
    pub permissions: Option<Vec<String>>,
    pub rate_limit: Option<i64>,
}

/// 更新 API 客户端请求
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateApiClientRequest {
    #[validate(length(min = 1, max = 100))]
    pub client_name: Option<String>,
    pub permissions: Option<Vec<String>>,
    pub rate_limit: Option<i64>,
    pub status: Option<i64>,
}

/// API 客户端查询参数
#[derive(Debug, Deserialize)]
pub struct ApiClientQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub status: Option<i64>,
    pub keyword: Option<String>,
}

impl ApiClientQuery {
    pub fn page(&self) -> u32 {
        self.page.unwrap_or(1).max(1)
    }

    pub fn page_size(&self) -> u32 {
        self.page_size.unwrap_or(20).min(100).max(1)
    }
}

/// 重新生成 API Key 的响应
#[derive(Debug, Serialize)]
pub struct RegenerateKeyResponse {
    pub api_key: String,
    pub api_secret: String,
}
