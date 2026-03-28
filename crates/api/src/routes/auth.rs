//! 认证模块 API 路由

use axum::{
    extract::State,
    routing::{get, post},
    Extension, Json, Router,
};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use tracing::{info, warn};
use validator::Validate;

use crate::middleware::auth::AuthUser;
use crate::state::AppState;
use cicierp_db::queries::users::UserQueries;
use cicierp_models::auth::{JwtConfig, LoginResponse, RoleBrief, UserInfo as AuthUserInfo};
use cicierp_models::user::{ChangePasswordRequest, LoginRequest};
use cicierp_utils::{AppError, AppResult, ApiResponse};

/// 创建公开认证路由（无需认证）
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
}

/// 创建需要认证的认证路由
pub fn protected_router() -> Router<AppState> {
    Router::new()
        .route("/auth/me", get(get_current_user))
        .route("/auth/password", post(change_password))
}

/// 哈希密码
fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| {
            warn!("Password hashing failed: {}", e);
            AppError::InternalError(anyhow::anyhow!("Password hashing failed"))
        })
}

/// 验证密码
fn verify_password(password: &str, hash: &str) -> Result<bool, AppError> {
    let parsed_hash = PasswordHash::new(hash).map_err(|e| {
        warn!("Invalid password hash: {}", e);
        AppError::InternalError(anyhow::anyhow!("Invalid password hash"))
    })?;

    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

/// @api POST /api/v1/auth/login
/// @desc 用户登录
/// @body LoginRequest { username, password }
/// @response 200 LoginResponse { token, token_type, expires_in, user }
/// @response 401 用户名或密码错误
/// @example curl -X POST "http://localhost:3000/api/v1/auth/login" \
///   -H "Content-Type: application/json" \
///   -d '{"username":"admin","password":"admin123"}'
pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> AppResult<Json<ApiResponse<LoginResponse>>> {
    info!("Login attempt: username={}", req.username);

    // 验证请求
    req.validate().map_err(|e| {
        warn!("Login validation failed: {}", e);
        AppError::BadRequest(e.to_string())
    })?;

    let queries = UserQueries::new(state.db.pool());

    // 查找用户
    let user = queries.get_by_username(&req.username).await.map_err(|e| {
        warn!("Database error during login: {}", e);
        AppError::InternalError(e.into())
    })?;

    let user = match user {
        Some(u) if u.status == 1 => u,
        Some(_) => {
            warn!("Login failed: user {} is disabled", req.username);
            return Err(AppError::BadRequest("账户已被禁用".to_string()));
        }
        None => {
            warn!("Login failed: user {} not found", req.username);
            return Err(AppError::BadRequest("用户名或密码错误".to_string()));
        }
    };

    // 验证密码
    if !verify_password(&req.password, &user.password_hash)? {
        warn!("Login failed: invalid password for user {}", req.username);
        return Err(AppError::BadRequest("用户名或密码错误".to_string()));
    }

    // 获取用户角色和权限
    let roles = queries.get_user_roles(user.id).await.map_err(|e| {
        warn!("Failed to get user roles: {}", e);
        AppError::InternalError(e.into())
    })?;

    let permissions = queries.get_user_permissions(user.id).await.map_err(|e| {
        warn!("Failed to get user permissions: {}", e);
        AppError::InternalError(e.into())
    })?;

    let role_codes: Vec<String> = roles.iter().map(|r| r.code.clone()).collect();
    let permission_list: Vec<String> = permissions;

    // 生成 token
    let config = JwtConfig::from_env();
    let token = crate::middleware::auth::generate_token(
        user.id,
        &user.username,
        role_codes.clone(),
        permission_list.clone(),
        &config,
    )
    .map_err(|e| {
        warn!("Token generation failed: {}", e);
        AppError::InternalError(anyhow::anyhow!("Token generation failed"))
    })?;

    // 更新最后登录时间
    let _ = queries.update_last_login(user.id, None).await;

    info!("User logged in successfully: {}", user.username);

    // 构建响应
    let response = LoginResponse {
        token,
        token_type: "Bearer".to_string(),
        expires_in: config.expires_in,
        user: AuthUserInfo {
            id: user.id,
            username: user.username,
            email: user.email,
            real_name: user.real_name,
            avatar: user.avatar,
            roles: roles
                .into_iter()
                .map(|r| RoleBrief {
                    id: r.id,
                    name: r.name,
                    code: r.code,
                })
                .collect(),
        },
    };

    Ok(Json(ApiResponse::success(response)))
}

/// @api POST /api/v1/auth/logout
/// @desc 用户登出（客户端清除 token 即可，服务端无状态）
/// @response 200 { "code": 200, "message": "登出成功" }
/// @example curl -X POST "http://localhost:3000/api/v1/auth/logout" \
///   -H "Authorization: Bearer <token>"
pub async fn logout() -> AppResult<Json<ApiResponse<()>>> {
    // JWT 是无状态的，登出只需客户端清除 token
    // 如果需要服务端 token 黑名单，可以在这里实现
    info!("User logged out");
    Ok(Json(ApiResponse::success_message("登出成功")))
}

/// @api GET /api/v1/auth/me
/// @desc 获取当前登录用户信息
/// @response 200 UserInfo
/// @response 401 未登录
/// @example curl -X GET "http://localhost:3000/api/v1/auth/me" \
///   -H "Authorization: Bearer <token>"
pub async fn get_current_user(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
) -> AppResult<Json<ApiResponse<AuthUserInfo>>> {
    info!("Get current user: {}", auth_user.username);

    let queries = UserQueries::new(state.db.pool());

    let user = queries.get_by_id(auth_user.user_id).await?.ok_or_else(|| {
        warn!("User not found in database: {}", auth_user.username);
        AppError::Unauthorized
    })?;

    let roles = queries.get_user_roles(user.id).await.map_err(|e| {
        warn!("Failed to get user roles: {}", e);
        AppError::InternalError(e.into())
    })?;

    let user_info = AuthUserInfo {
        id: user.id,
        username: user.username,
        email: user.email,
        real_name: user.real_name,
        avatar: user.avatar,
        roles: roles
            .into_iter()
            .map(|r| RoleBrief {
                id: r.id,
                name: r.name,
                code: r.code,
            })
            .collect(),
    };

    Ok(Json(ApiResponse::success(user_info)))
}

/// @api POST /api/v1/auth/password
/// @desc 修改密码
/// @body ChangePasswordRequest { old_password, new_password }
/// @response 200 { "code": 200, "message": "密码修改成功" }
/// @response 400 旧密码错误
/// @example curl -X POST "http://localhost:3000/api/v1/auth/password" \
///   -H "Authorization: Bearer <token>" \
///   -H "Content-Type: application/json" \
///   -d '{"old_password":"old123","new_password":"new123"}'
pub async fn change_password(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<ChangePasswordRequest>,
) -> AppResult<Json<ApiResponse<()>>> {
    info!("Change password: {}", auth_user.username);

    // 验证请求
    req.validate().map_err(|e| {
        warn!("Password change validation failed: {}", e);
        AppError::BadRequest(e.to_string())
    })?;

    let queries = UserQueries::new(state.db.pool());

    let user = queries.get_by_id(auth_user.user_id).await?.ok_or_else(|| {
        warn!("User not found in database: {}", auth_user.username);
        AppError::Unauthorized
    })?;

    // 验证旧密码
    if !verify_password(&req.old_password, &user.password_hash)? {
        warn!("Invalid old password for user: {}", auth_user.username);
        return Err(AppError::BadRequest("旧密码错误".to_string()));
    }

    // 哈希新密码
    let new_hash = hash_password(&req.new_password)?;

    // 更新密码
    queries
        .update_password(user.id, &new_hash)
        .await
        .map_err(|e| {
            warn!("Failed to update password: {}", e);
            AppError::InternalError(e.into())
        })?;

    info!("Password changed successfully: {}", auth_user.username);
    Ok(Json(ApiResponse::success_message("密码修改成功")))
}
