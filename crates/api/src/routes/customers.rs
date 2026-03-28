//! 客户模块 API 路由

use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use tracing::{info, instrument};
use validator::Validate;

use crate::state::AppState;
use cicierp_db::queries::customers::CustomerQueries;
use cicierp_models::common::PagedResponse;
use cicierp_models::customer::{CreateCustomerRequest, Customer, CustomerAddress, CustomerQuery, UpdateCustomerRequest};
use cicierp_utils::{AppError, AppResult, ApiResponse};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/customers", get(list_customers))
        .route("/customers", post(create_customer))
        .route("/customers/:id", get(get_customer))
        .route("/customers/:id", put(update_customer))
        .route("/customers/:id", delete(delete_customer))
        .route("/customers/:id/addresses", get(get_customer_addresses))
}

/// @api GET /api/v1/customers
/// @desc 获取客户列表
/// @query page: number (页码，默认1)
/// @query page_size: number (每页数量，默认20)
/// @query level_id: number (客户等级ID，可选)
/// @query status: number (状态：1正常 2冻结 3黑名单，可选)
/// @query source: string (来源平台，可选)
/// @query keyword: string (搜索关键词，可选)
/// @response 200 PagedResponse<Customer>
/// @example curl -X GET "http://localhost:3000/api/v1/customers?page=1&page_size=20"
#[instrument(skip(state))]
pub async fn list_customers(
    State(state): State<AppState>,
    Query(query): Query<CustomerQuery>,
) -> AppResult<Json<ApiResponse<PagedResponse<Customer>>>> {
    info!("Listing customers");

    let queries = CustomerQueries::new(state.db.pool());
    let result = queries
        .list(
            query.page(),
            query.page_size(),
            query.level_id,
            query.status,
            query.source.as_deref(),
            query.keyword.as_deref(),
        )
        .await?;

    Ok(Json(ApiResponse::success(result)))
}

/// @api GET /api/v1/customers/:id
/// @desc 获取客户详情
/// @param id: number (客户ID)
/// @response 200 Customer
/// @response 404 客户不存在
/// @example curl -X GET "http://localhost:3000/api/v1/customers/1"
#[instrument(skip(state))]
pub async fn get_customer(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Customer>>> {
    info!("Getting customer: id={}", id);

    let queries = CustomerQueries::new(state.db.pool());
    let customer = queries.get_by_id(id).await?.ok_or(AppError::NotFound)?;

    Ok(Json(ApiResponse::success(customer)))
}

/// @api POST /api/v1/customers
/// @desc 创建客户
/// @body CreateCustomerRequest
/// @response 200 Customer
/// @example curl -X POST "http://localhost:3000/api/v1/customers" \
///   -H "Content-Type: application/json" \
///   -d '{"name":"张三","source":"manual"}'
#[instrument(skip(state))]
pub async fn create_customer(
    State(state): State<AppState>,
    Json(req): Json<CreateCustomerRequest>,
) -> AppResult<Json<ApiResponse<Customer>>> {
    info!("Creating customer: name={}", req.name);

    req.validate().map_err(AppError::from)?;

    let queries = CustomerQueries::new(state.db.pool());
    let customer = queries.create(&req).await?;

    info!("Customer created: id={}", customer.id);
    Ok(Json(ApiResponse::success(customer)))
}

/// @api PUT /api/v1/customers/:id
/// @desc 更新客户
/// @param id: number (客户ID)
/// @body UpdateCustomerRequest
/// @response 200 Customer
/// @response 404 客户不存在
/// @example curl -X PUT "http://localhost:3000/api/v1/customers/1" \
///   -H "Content-Type: application/json" \
///   -d '{"name":"新名称"}'
#[instrument(skip(state))]
pub async fn update_customer(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateCustomerRequest>,
) -> AppResult<Json<ApiResponse<Customer>>> {
    info!("Updating customer: id={}", id);

    // 添加输入验证
    req.validate().map_err(AppError::from)?;

    let queries = CustomerQueries::new(state.db.pool());
    let customer = queries.update(id, &req).await?.ok_or(AppError::NotFound)?;

    info!("Customer updated: id={}", id);
    Ok(Json(ApiResponse::success(customer)))
}

/// @api DELETE /api/v1/customers/:id
/// @desc 删除客户（软删除）
/// @param id: number (客户ID)
/// @response 200 {"code": 200, "message": "删除成功"}
/// @response 404 客户不存在
/// @example curl -X DELETE "http://localhost:3000/api/v1/customers/1"
#[instrument(skip(state))]
pub async fn delete_customer(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<()>>> {
    info!("Deleting customer: id={}", id);

    let queries = CustomerQueries::new(state.db.pool());
    let deleted = queries.delete(id).await?;

    if !deleted {
        return Err(AppError::NotFound);
    }

    info!("Customer deleted: id={}", id);
    Ok(Json(ApiResponse::success_message("删除成功")))
}

/// @api GET /api/v1/customers/:id/addresses
/// @desc 获取客户地址列表
/// @param id: number (客户ID)
/// @response 200 Vec<CustomerAddress>
/// @response 404 客户不存在
/// @example curl -X GET "http://localhost:3000/api/v1/customers/1/addresses"
#[instrument(skip(state))]
pub async fn get_customer_addresses(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Vec<CustomerAddress>>>> {
    info!("Getting customer addresses: customer_id={}", id);

    let queries = CustomerQueries::new(state.db.pool());

    // 先检查客户是否存在
    let _ = queries.get_by_id(id).await?.ok_or(AppError::NotFound)?;

    let addresses = queries.get_addresses(id).await?;

    Ok(Json(ApiResponse::success(addresses)))
}
