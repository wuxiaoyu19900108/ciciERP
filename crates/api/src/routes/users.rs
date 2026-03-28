//! 用户管理 API 路由

use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Extension, Json, Router,
};
use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    PasswordHasher, Argon2,
};
use tracing::{info, warn};
use validator::Validate;

use crate::middleware::auth::{require_admin, AuthUser};
use crate::state::AppState;
use cicierp_db::queries::users::{RoleQueries, UserQueries};
use cicierp_models::common::PagedResponse;
use cicierp_models::user::{
    CreateUserRequest, ResetPasswordRequest, RoleInfo, UpdateUserRequest, User, UserQuery,
};
use cicierp_utils::{AppError, AppResult, ApiResponse};

/// 创建用户管理路由
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/users", get(list_users))
        .route("/users", post(create_user))
        .route("/users/:id", get(get_user))
        .route("/users/:id", put(update_user))
        .route("/users/:id", delete(delete_user))
        .route("/users/:id/reset-password", post(reset_password))
        .route("/roles", get(list_roles))
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

/// @api GET /api/v1/users
/// @desc 获取用户列表（需要管理员权限）
/// @query page: number (页码，默认1)
/// @query page_size: number (每页数量，默认20)
/// @query keyword: string (搜索关键词，可选)
/// @query status: number (状态筛选，可选)
/// @response 200 PagedResponse<User>
/// @example curl -X GET "http://localhost:3000/api/v1/users?page=1&page_size=20" \
///   -H "Authorization: Bearer <token>"
pub async fn list_users(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Query(query): Query<UserQuery>,
) -> AppResult<Json<ApiResponse<PagedResponse<User>>>> {
    info!("List users: page={}, page_size={}", query.page(), query.page_size());

    // 检查管理员权限
    require_admin(&auth_user)?;

    let queries = UserQueries::new(state.db.pool());
    let (users, total) = queries
        .list(
            query.page(),
            query.page_size(),
            query.keyword.as_deref(),
            query.status,
        )
        .await?;

    let response = PagedResponse::new(users, query.page(), query.page_size(), total);
    Ok(Json(ApiResponse::success(response)))
}

/// @api POST /api/v1/users
/// @desc 创建用户（需要管理员权限）
/// @body CreateUserRequest
/// @response 200 User
/// @response 400 参数错误
/// @response 409 用户名已存在
/// @example curl -X POST "http://localhost:3000/api/v1/users" \
///   -H "Authorization: Bearer <token>" \
///   -H "Content-Type: application/json" \
///   -d '{"username":"newuser","password":"password123"}'
pub async fn create_user(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<CreateUserRequest>,
) -> AppResult<Json<ApiResponse<User>>> {
    info!("Create user: username={}", req.username);

    // 检查管理员权限
    require_admin(&auth_user)?;

    // 验证请求
    req.validate().map_err(|e| {
        warn!("Create user validation failed: {}", e);
        AppError::BadRequest(e.to_string())
    })?;

    let queries = UserQueries::new(state.db.pool());

    // 检查用户名是否已存在
    if queries.get_by_username(&req.username).await?.is_some() {
        return Err(AppError::Conflict("用户名已存在".to_string()));
    }

    // 检查邮箱是否已存在
    if let Some(ref email) = req.email {
        if queries.get_by_email(email).await?.is_some() {
            return Err(AppError::Conflict("邮箱已被使用".to_string()));
        }
    }

    // 检查手机号是否已存在
    if let Some(ref mobile) = req.mobile {
        if queries.get_by_mobile(mobile).await?.is_some() {
            return Err(AppError::Conflict("手机号已被使用".to_string()));
        }
    }

    // 哈希密码
    let password_hash = hash_password(&req.password)?;

    // 创建用户
    let user = queries.create(&req, &password_hash).await?;
    info!("User created: id={}, username={}", user.id, user.username);

    Ok(Json(ApiResponse::success(user)))
}

/// @api GET /api/v1/users/:id
/// @desc 获取用户详情（需要管理员权限）
/// @param id: number (用户ID)
/// @response 200 User
/// @response 404 用户不存在
/// @example curl -X GET "http://localhost:3000/api/v1/users/1" \
///   -H "Authorization: Bearer <token>"
pub async fn get_user(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<User>>> {
    info!("Get user: id={}", id);

    // 检查管理员权限
    require_admin(&auth_user)?;

    let queries = UserQueries::new(state.db.pool());
    let user = queries.get_by_id(id).await?.ok_or(AppError::NotFound)?;

    Ok(Json(ApiResponse::success(user)))
}

/// @api PUT /api/v1/users/:id
/// @desc 更新用户信息（需要管理员权限）
/// @param id: number (用户ID)
/// @body UpdateUserRequest
/// @response 200 User
/// @response 404 用户不存在
/// @example curl -X PUT "http://localhost:3000/api/v1/users/1" \
///   -H "Authorization: Bearer <token>" \
///   -H "Content-Type: application/json" \
///   -d '{"real_name":"新名称"}'
pub async fn update_user(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateUserRequest>,
) -> AppResult<Json<ApiResponse<User>>> {
    info!("Update user: id={}", id);

    // 检查管理员权限
    require_admin(&auth_user)?;

    // 验证请求
    req.validate().map_err(|e| {
        warn!("Update user validation failed: {}", e);
        AppError::BadRequest(e.to_string())
    })?;

    let queries = UserQueries::new(state.db.pool());

    // 检查邮箱是否已被其他用户使用
    if let Some(ref email) = req.email {
        if let Some(existing) = queries.get_by_email(email).await? {
            if existing.id != id {
                return Err(AppError::Conflict("邮箱已被其他用户使用".to_string()));
            }
        }
    }

    // 检查手机号是否已被其他用户使用
    if let Some(ref mobile) = req.mobile {
        if let Some(existing) = queries.get_by_mobile(mobile).await? {
            if existing.id != id {
                return Err(AppError::Conflict("手机号已被其他用户使用".to_string()));
            }
        }
    }

    let user = queries.update(id, &req).await?.ok_or(AppError::NotFound)?;
    info!("User updated: id={}", id);

    Ok(Json(ApiResponse::success(user)))
}

/// @api DELETE /api/v1/users/:id
/// @desc 删除用户（软删除，需要管理员权限）
/// @param id: number (用户ID)
/// @response 200 { "code": 200, "message": "删除成功" }
/// @response 404 用户不存在
/// @example curl -X DELETE "http://localhost:3000/api/v1/users/1" \
///   -H "Authorization: Bearer <token>"
pub async fn delete_user(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<()>>> {
    info!("Delete user: id={}", id);

    // 检查管理员权限
    require_admin(&auth_user)?;

    // 不能删除自己
    if auth_user.user_id == id {
        return Err(AppError::BadRequest("不能删除自己的账户".to_string()));
    }

    let queries = UserQueries::new(state.db.pool());
    let deleted = queries.delete(id).await?;

    if !deleted {
        return Err(AppError::NotFound);
    }

    info!("User deleted: id={}", id);
    Ok(Json(ApiResponse::success_message("删除成功")))
}

/// @api POST /api/v1/users/:id/reset-password
/// @desc 重置用户密码（需要管理员权限）
/// @param id: number (用户ID)
/// @body ResetPasswordRequest
/// @response 200 { "code": 200, "message": "密码重置成功" }
/// @response 404 用户不存在
/// @example curl -X POST "http://localhost:3000/api/v1/users/1/reset-password" \
///   -H "Authorization: Bearer <token>" \
///   -H "Content-Type: application/json" \
///   -d '{"new_password":"newpassword123"}'
pub async fn reset_password(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<i64>,
    Json(req): Json<ResetPasswordRequest>,
) -> AppResult<Json<ApiResponse<()>>> {
    info!("Reset password for user: id={}", id);

    // 检查管理员权限
    require_admin(&auth_user)?;

    // 验证请求
    req.validate().map_err(|e| {
        warn!("Reset password validation failed: {}", e);
        AppError::BadRequest(e.to_string())
    })?;

    let queries = UserQueries::new(state.db.pool());

    // 检查用户是否存在
    let user = queries.get_by_id(id).await?.ok_or(AppError::NotFound)?;

    // 哈希新密码
    let password_hash = hash_password(&req.new_password)?;

    // 更新密码
    queries.update_password(user.id, &password_hash).await?;

    info!("Password reset for user: id={}", id);
    Ok(Json(ApiResponse::success_message("密码重置成功")))
}

/// @api GET /api/v1/roles
/// @desc 获取角色列表（需要管理员权限）
/// @response 200 Vec<Role>
/// @example curl -X GET "http://localhost:3000/api/v1/roles" \
///   -H "Authorization: Bearer <token>"
pub async fn list_roles(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
) -> AppResult<Json<ApiResponse<Vec<cicierp_models::user::Role>>>> {
    info!("List roles");

    // 检查管理员权限
    require_admin(&auth_user)?;

    let queries = RoleQueries::new(state.db.pool());
    let roles = queries.list().await?;

    Ok(Json(ApiResponse::success(roles)))
}
