//! 对接 API 认证中间件
//!
//! 用于验证外部系统（如 cicishop）的 API 调用
//! 认证方式：API Key + HMAC 签名

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use tracing::{debug, warn};

use cicierp_db::queries::api_clients::ApiClientQueries;
use cicierp_models::api_client::ApiClient;
use cicierp_utils::AppError;

use crate::state::AppState;

type HmacSha256 = Hmac<Sha256>;

/// 认证客户端信息，用于在请求扩展中传递
#[derive(Debug, Clone)]
pub struct IntegrationClient {
    pub id: i64,
    pub client_id: String,
    pub client_name: String,
    pub permissions: Vec<String>,
}

impl IntegrationClient {
    pub fn from_api_client(client: &ApiClient) -> Self {
        let permissions = if let serde_json::Value::Array(perms) = &client.permissions {
            perms
                .iter()
                .filter_map(|p| p.as_str().map(|s| s.to_string()))
                .collect()
        } else {
            vec![]
        };

        IntegrationClient {
            id: client.id,
            client_id: client.client_id.clone(),
            client_name: client.client_name.clone(),
            permissions,
        }
    }

    /// 检查是否有指定权限
    pub fn has_permission(&self, permission: &str) -> bool {
        // 通配符权限
        if self.permissions.contains(&"*".to_string()) {
            return true;
        }
        // 精确匹配
        if self.permissions.contains(&permission.to_string()) {
            return true;
        }
        // 模块通配符匹配
        if let Some(colon_pos) = permission.find(':') {
            let module_perm = format!("{}:*", &permission[..colon_pos]);
            if self.permissions.contains(&module_perm) {
                return true;
            }
        }
        false
    }
}

/// 对接 API 认证中间件
pub async fn integration_auth_middleware(
    State(state): State<AppState>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    // 1. 提取 API Key（克隆以避免借用问题）
    let api_key = req.headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|s| s.to_string())
        .ok_or_else(|| {
            warn!("Missing or invalid Authorization header");
            AppError::Unauthorized
        })?;

    // 2. 提取签名和时间戳
    let signature = req.headers()
        .get("X-Signature")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            warn!("Missing X-Signature header");
            AppError::Unauthorized
        })?;

    let timestamp = req.headers()
        .get("X-Timestamp")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            warn!("Missing X-Timestamp header");
            AppError::Unauthorized
        })?;

    // 3. 验证时间戳（防重放攻击）
    let ts: i64 = timestamp.parse().map_err(|_| {
        warn!("Invalid timestamp format");
        AppError::Unauthorized
    })?;

    let now = chrono::Utc::now().timestamp();
    if (now - ts).abs() > 300 {  // 5分钟有效期
        warn!("Timestamp expired or too far in the future");
        return Err(AppError::Unauthorized);
    }

    // 4. 验证 API Key 并获取客户端信息
    let queries = ApiClientQueries::new(state.db.pool());
    let client = queries.get_by_api_key(&api_key).await
        .map_err(|e| {
            warn!("Database error while validating API key: {}", e);
            AppError::InternalError(anyhow::anyhow!("Database error"))
        })?
        .ok_or_else(|| {
            warn!("Invalid API key: {}", api_key);
            AppError::Unauthorized
        })?;

    // 5. 验证签名
    // 对于有请求体的请求，需要读取 body 并验证签名
    let (parts, body) = req.into_parts();
    let body_bytes = axum::body::to_bytes(body, usize::MAX)
        .await
        .map_err(|e| {
            warn!("Failed to read request body: {}", e);
            AppError::BadRequest("Failed to read request body".to_string())
        })?;

    let expected_signature = compute_hmac_signature(
        &client.api_secret,
        &api_key,
        &timestamp,
        &body_bytes,
    );

    if signature != expected_signature {
        warn!("Signature verification failed");
        return Err(AppError::Unauthorized);
    }

    // 6. 更新最后使用时间
    if let Err(e) = queries.update_last_used(client.id).await {
        debug!("Failed to update last_used_at: {}", e);
    }

    // 7. 将客户端信息存入请求扩展
    let integration_client = IntegrationClient::from_api_client(&client);

    // 8. 重建请求
    let mut req = Request::from_parts(parts, Body::from(body_bytes));
    req.extensions_mut().insert(integration_client);

    debug!("Integration client authenticated: {}", client.client_id);
    Ok(next.run(req).await)
}

/// 计算 HMAC 签名
fn compute_hmac_signature(secret: &str, api_key: &str, timestamp: &str, body: &[u8]) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .expect("HMAC can take key of any size");

    // 签名内容：api_key + timestamp + body
    mac.update(api_key.as_bytes());
    mac.update(timestamp.as_bytes());
    mac.update(body);

    let result = mac.finalize();
    format!("sha256={}", hex::encode(result.into_bytes()))
}

/// 简化版认证中间件（仅验证 API Key，不验证签名）
/// 用于只读接口或内部调用
pub async fn simple_integration_auth_middleware(
    State(state): State<AppState>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    // 提取 API Key
    let api_key = req.headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| {
            warn!("Missing or invalid Authorization header");
            AppError::Unauthorized
        })?;

    // 验证 API Key
    let queries = ApiClientQueries::new(state.db.pool());
    let client = queries.get_by_api_key(api_key).await
        .map_err(|e| {
            warn!("Database error while validating API key: {}", e);
            AppError::InternalError(anyhow::anyhow!("Database error"))
        })?
        .ok_or_else(|| {
            warn!("Invalid API key");
            AppError::Unauthorized
        })?;

    // 更新最后使用时间
    if let Err(e) = queries.update_last_used(client.id).await {
        debug!("Failed to update last_used_at: {}", e);
    }

    // 将客户端信息存入请求扩展
    let integration_client = IntegrationClient::from_api_client(&client);
    req.extensions_mut().insert(integration_client);

    debug!("Integration client authenticated (simple): {}", client.client_id);
    Ok(next.run(req).await)
}

/// 检查是否有特定权限
pub fn require_integration_permission(client: &IntegrationClient, permission: &str) -> Result<(), AppError> {
    if client.has_permission(permission) {
        Ok(())
    } else {
        warn!("Permission denied: {} for client {}", permission, client.client_id);
        Err(AppError::Forbidden)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_hmac_signature() {
        let secret = "test-secret";
        let api_key = "test-api-key";
        let timestamp = "1234567890";
        let body = b"test-body";

        let signature = compute_hmac_signature(secret, api_key, timestamp, body);
        assert!(signature.starts_with("sha256="));
        assert_eq!(signature.len(), 71); // "sha256=" (7) + 64 hex chars
    }

    #[test]
    fn test_integration_client_permissions() {
        let client = IntegrationClient {
            id: 1,
            client_id: "test".to_string(),
            client_name: "Test Client".to_string(),
            permissions: vec!["products:read".to_string(), "orders:*".to_string()],
        };

        assert!(client.has_permission("products:read"));
        assert!(client.has_permission("orders:create"));
        assert!(!client.has_permission("products:write"));
        assert!(!client.has_permission("customers:read"));

        // 通配符权限
        let admin_client = IntegrationClient {
            id: 2,
            client_id: "admin".to_string(),
            client_name: "Admin Client".to_string(),
            permissions: vec!["*".to_string()],
        };
        assert!(admin_client.has_permission("anything"));
    }
}
