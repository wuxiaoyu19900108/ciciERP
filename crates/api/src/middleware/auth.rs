//! JWT 认证中间件

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use tracing::{debug, warn};

use cicierp_models::auth::{Claims, JwtConfig};
use cicierp_utils::AppError;

use crate::state::AppState;

/// 认证用户信息，用于在请求扩展中传递
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: i64,
    pub username: String,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
}

/// 生成 JWT Token
pub fn generate_token(
    user_id: i64,
    username: &str,
    roles: Vec<String>,
    permissions: Vec<String>,
    config: &JwtConfig,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = chrono::Utc::now().timestamp() as usize;
    let exp = now + config.expires_in as usize;

    let claims = Claims {
        sub: user_id,
        username: username.to_string(),
        roles,
        permissions,
        exp,
        iat: now,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(config.secret.as_bytes()),
    )
}

/// 验证 JWT Token
pub fn verify_token(token: &str, config: &JwtConfig) -> Result<Claims, jsonwebtoken::errors::Error> {
    let token = token.trim();

    decode::<Claims>(
        token,
        &DecodingKey::from_secret(config.secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    )
    .map(|data| data.claims)
}

/// 从请求头或 Cookie 提取 Token
pub fn extract_token<B>(req: &Request<B>) -> Option<String> {
    // 1. 先尝试从 Authorization header 获取
    if let Some(auth_header) = req.headers().get("Authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if auth_str.starts_with("Bearer ") {
                return Some(auth_str[7..].to_string());
            }
        }
    }

    // 2. 再尝试从 Cookie 获取（用于 Web 页面认证）
    if let Some(cookie_header) = req.headers().get("Cookie") {
        if let Ok(cookie_str) = cookie_header.to_str() {
            for cookie in cookie_str.split(';') {
                let cookie = cookie.trim();
                if cookie.starts_with("auth_token=") {
                    return Some(cookie[11..].to_string());
                }
            }
        }
    }

    None
}

/// 判断是否为 Web 页面请求（非 API 请求）
fn is_web_request<B>(req: &Request<B>) -> bool {
    let path = req.uri().path();
    !path.starts_with("/api/")
}

/// 认证中间件
pub async fn auth_middleware(
    State(state): State<AppState>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    // 提取 token（支持 Authorization header 和 Cookie）
    let token = match extract_token(&req) {
        Some(t) => t,
        None => {
            warn!("Missing or invalid token in Authorization header or Cookie");
            return handle_unauthorized(&req);
        }
    };

    // 获取 JWT 配置
    let config = JwtConfig::from_env();

    // 验证 token
    let claims = match verify_token(&token, &config) {
        Ok(c) => c,
        Err(e) => {
            warn!("Token verification failed: {}", e);
            return handle_unauthorized(&req);
        }
    };

    // 检查 token 是否过期
    let now = chrono::Utc::now().timestamp() as usize;
    if claims.exp < now {
        warn!("Token expired");
        return handle_unauthorized(&req);
    }

    // 验证用户是否仍然有效
    let queries = cicierp_db::queries::users::UserQueries::new(state.db.pool());
    let user = match queries.get_by_id(claims.sub).await {
        Ok(u) => u,
        Err(e) => {
            warn!("Database error while validating user: {}", e);
            return handle_unauthorized(&req);
        }
    };

    match user {
        Some(u) if u.status == 1 => {
            // 用户有效，将认证信息存入请求扩展
            let auth_user = AuthUser {
                user_id: claims.sub,
                username: claims.username.clone(),
                roles: claims.roles.clone(),
                permissions: claims.permissions.clone(),
            };

            req.extensions_mut().insert(auth_user);
            debug!("User authenticated: {}", claims.username);

            Ok(next.run(req).await)
        }
        Some(_) => {
            warn!("User account is disabled: {}", claims.username);
            Err(AppError::Forbidden)
        }
        None => {
            warn!("User not found: {}", claims.username);
            handle_unauthorized(&req)
        }
    }
}

/// 处理未授权的情况
/// - Web 页面请求：返回 302 重定向到 /login
/// - API 请求：返回 AppError::Unauthorized（401 JSON）
fn handle_unauthorized<B>(req: &Request<B>) -> Result<Response, AppError> {
    if is_web_request(req) {
        // Web 请求：重定向到登录页
        let response = Response::builder()
            .status(StatusCode::FOUND)
            .header("Location", "/login")
            .body(Body::empty())
            .map_err(|e| AppError::InternalError(anyhow::anyhow!("Failed to build redirect response: {}", e)))?;
        Ok(response)
    } else {
        // API 请求：返回 401 错误
        Err(AppError::Unauthorized)
    }
}

/// 可选认证中间件（不强制要求登录）
pub async fn optional_auth_middleware(
    State(state): State<AppState>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    if let Some(token) = extract_token(&req) {
        let config = JwtConfig::from_env();

        if let Ok(claims) = verify_token(&token, &config) {
            let now = chrono::Utc::now().timestamp() as usize;
            if claims.exp >= now {
                // 验证用户是否有效
                let queries = cicierp_db::queries::users::UserQueries::new(state.db.pool());
                if let Ok(Some(user)) = queries.get_by_id(claims.sub).await {
                    if user.status == 1 {
                        let auth_user = AuthUser {
                            user_id: claims.sub,
                            username: claims.username.clone(),
                            roles: claims.roles.clone(),
                            permissions: claims.permissions.clone(),
                        };
                        req.extensions_mut().insert(auth_user);
                    }
                }
            }
        }
    }

    Ok(next.run(req).await)
}

/// 检查是否为管理员
pub fn require_admin(auth_user: &AuthUser) -> Result<(), AppError> {
    if auth_user.roles.iter().any(|r| r == "admin" || r == "super_admin") {
        Ok(())
    } else if auth_user.permissions.contains(&"*".to_string()) {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}

/// 检查是否有特定权限
pub fn require_permission(auth_user: &AuthUser, permission: &str) -> Result<(), AppError> {
    // 超级管理员拥有所有权限
    if auth_user.permissions.contains(&"*".to_string()) {
        return Ok(());
    }

    // 检查精确匹配
    if auth_user.permissions.contains(&permission.to_string()) {
        return Ok(());
    }

    // 检查模块通配符
    if let Some(colon_pos) = permission.find(':') {
        let module_perm = format!("{}:*", &permission[..colon_pos]);
        if auth_user.permissions.contains(&module_perm) {
            return Ok(());
        }
    }

    Err(AppError::Forbidden)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_and_verify_token() {
        let config = JwtConfig {
            secret: "test-secret".to_string(),
            expires_in: 3600,
        };

        let token = generate_token(
            1,
            "testuser",
            vec!["admin".to_string()],
            vec!["*".to_string()],
            &config,
        )
        .unwrap();

        let claims = verify_token(&token, &config).unwrap();

        assert_eq!(claims.sub, 1);
        assert_eq!(claims.username, "testuser");
    }
}
