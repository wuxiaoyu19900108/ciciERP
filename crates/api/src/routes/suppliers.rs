//! 供应商模块 API 路由

use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use tracing::{info, instrument};
use validator::Validate;

use crate::state::AppState;
use cicierp_db::queries::suppliers::SupplierQueries;
use cicierp_models::common::PagedResponse;
use cicierp_models::supplier::{CreateSupplierRequest, ProductSupplierInfo, Supplier, SupplierDetail, SupplierQuery, UpdateSupplierRequest};
use cicierp_utils::{AppError, AppResult, ApiResponse};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/suppliers", get(list_suppliers))
        .route("/suppliers", post(create_supplier))
        .route("/suppliers/:id", get(get_supplier))
        .route("/suppliers/:id", put(update_supplier))
        .route("/suppliers/:id", delete(delete_supplier))
        .route("/suppliers/:id/products", get(get_supplier_products))
}

/// @api GET /api/v1/suppliers
/// @desc 获取供应商列表
/// @query page: number (页码，默认1)
/// @query page_size: number (每页数量，默认20)
/// @query status: number (状态：1合作 2暂停 3终止，可选)
/// @query rating_level: string (评级 A/B/C/D，可选)
/// @query keyword: string (搜索关键词，可选)
/// @response 200 PagedResponse<Supplier>
/// @example curl -X GET "http://localhost:3000/api/v1/suppliers?page=1&page_size=20"
#[instrument(skip(state))]
pub async fn list_suppliers(
    State(state): State<AppState>,
    Query(query): Query<SupplierQuery>,
) -> AppResult<Json<ApiResponse<PagedResponse<Supplier>>>> {
    info!("Listing suppliers");

    let queries = SupplierQueries::new(state.db.pool());
    let result = queries
        .list(
            query.page(),
            query.page_size(),
            query.status,
            query.rating_level.as_deref(),
            query.keyword.as_deref(),
        )
        .await?;

    Ok(Json(ApiResponse::success(result)))
}

/// @api GET /api/v1/suppliers/:id
/// @desc 获取供应商详情
/// @param id: number (供应商ID)
/// @response 200 SupplierDetail
/// @response 404 供应商不存在
/// @example curl -X GET "http://localhost:3000/api/v1/suppliers/1"
#[instrument(skip(state))]
pub async fn get_supplier(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<SupplierDetail>>> {
    info!("Getting supplier: id={}", id);

    let queries = SupplierQueries::new(state.db.pool());
    let supplier = queries.get_by_id(id).await?.ok_or(AppError::NotFound)?;
    let products = queries.get_products(id).await?;

    let detail = SupplierDetail {
        supplier,
        products,
    };

    Ok(Json(ApiResponse::success(detail)))
}

/// @api POST /api/v1/suppliers
/// @desc 创建供应商
/// @body CreateSupplierRequest
/// @response 200 Supplier
/// @example curl -X POST "http://localhost:3000/api/v1/suppliers" \
///   -H "Content-Type: application/json" \
///   -d '{"supplier_code":"S001","name":"测试供应商"}'
#[instrument(skip(state))]
pub async fn create_supplier(
    State(state): State<AppState>,
    Json(req): Json<CreateSupplierRequest>,
) -> AppResult<Json<ApiResponse<Supplier>>> {
    info!("Creating supplier: code={}", req.supplier_code.clone().unwrap_or_else(|| "auto".to_string()));

    req.validate().map_err(AppError::from)?;

    let queries = SupplierQueries::new(state.db.pool());
    let supplier = queries.create(&req).await?;

    info!("Supplier created: id={}", supplier.id);
    Ok(Json(ApiResponse::success(supplier)))
}

/// @api PUT /api/v1/suppliers/:id
/// @desc 更新供应商
/// @param id: number (供应商ID)
/// @body UpdateSupplierRequest
/// @response 200 Supplier
/// @response 404 供应商不存在
/// @example curl -X PUT "http://localhost:3000/api/v1/suppliers/1" \
///   -H "Content-Type: application/json" \
///   -d '{"name":"新名称"}'
#[instrument(skip(state))]
pub async fn update_supplier(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateSupplierRequest>,
) -> AppResult<Json<ApiResponse<Supplier>>> {
    info!("Updating supplier: id={}", id);

    // 添加输入验证
    req.validate().map_err(AppError::from)?;

    let queries = SupplierQueries::new(state.db.pool());
    let supplier = queries.update(id, &req).await?.ok_or(AppError::NotFound)?;

    info!("Supplier updated: id={}", id);
    Ok(Json(ApiResponse::success(supplier)))
}

/// @api DELETE /api/v1/suppliers/:id
/// @desc 删除供应商（软删除）
/// @param id: number (供应商ID)
/// @response 200 {"code": 200, "message": "删除成功"}
/// @response 404 供应商不存在
/// @example curl -X DELETE "http://localhost:3000/api/v1/suppliers/1"
#[instrument(skip(state))]
pub async fn delete_supplier(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<()>>> {
    info!("Deleting supplier: id={}", id);

    let queries = SupplierQueries::new(state.db.pool());
    let deleted = queries.delete(id).await?;

    if !deleted {
        return Err(AppError::NotFound);
    }

    info!("Supplier deleted: id={}", id);
    Ok(Json(ApiResponse::success_message("删除成功")))
}

/// @api GET /api/v1/suppliers/:id/products
/// @desc 获取供应商的产品列表
/// @param id: number (供应商ID)
/// @response 200 [ProductSupplierInfo]
/// @example curl -X GET "http://localhost:3000/api/v1/suppliers/1/products"
#[instrument(skip(state))]
pub async fn get_supplier_products(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Vec<ProductSupplierInfo>>>> {
    info!("Getting supplier products: supplier_id={}", id);

    let queries = SupplierQueries::new(state.db.pool());
    let products = queries.get_products(id).await?;

    Ok(Json(ApiResponse::success(products)))
}
