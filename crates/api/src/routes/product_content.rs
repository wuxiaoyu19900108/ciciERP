//! 产品内容 API 路由

use axum::{
    extract::{Path, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use tracing::{info, instrument};
use validator::Validate;

use crate::state::AppState;
use cicierp_db::queries::product_content::ProductContentQueries;
use cicierp_models::product::{CreateProductContentRequest, ProductContent, UpdateProductContentRequest};
use cicierp_utils::{AppError, AppResult, ApiResponse};

/// 创建产品内容路由
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/products/:product_id/content", get(get_product_content))
        .route("/products/:product_id/content", post(create_product_content))
        .route("/products/:product_id/content", put(upsert_product_content))
        .route("/products/:product_id/content", delete(delete_product_content))
}

/// @api GET /api/v1/products/:product_id/content
/// @desc 获取产品内容
/// @param product_id: number (产品ID)
/// @response 200 ProductContent
/// @response 404 内容不存在
#[instrument(skip(state))]
pub async fn get_product_content(
    State(state): State<AppState>,
    Path(product_id): Path<i64>,
) -> AppResult<Json<ApiResponse<ProductContent>>> {
    info!("Getting product content: product_id={}", product_id);

    let queries = ProductContentQueries::new(state.db.pool());
    let content = queries.get_by_product_id(product_id).await?.ok_or(AppError::NotFound)?;

    Ok(Json(ApiResponse::success(content)))
}

/// @api POST /api/v1/products/:product_id/content
/// @desc 创建产品内容
/// @param product_id: number (产品ID)
/// @body CreateProductContentRequest
/// @response 200 ProductContent
/// @response 409 内容已存在
#[instrument(skip(state))]
pub async fn create_product_content(
    State(state): State<AppState>,
    Path(product_id): Path<i64>,
    Json(mut req): Json<CreateProductContentRequest>,
) -> AppResult<Json<ApiResponse<ProductContent>>> {
    info!("Creating product content: product_id={}", product_id);

    req.validate().map_err(AppError::from)?;
    req.product_id = product_id;

    let queries = ProductContentQueries::new(state.db.pool());

    // 检查是否已存在
    if queries.get_by_product_id(product_id).await?.is_some() {
        return Err(AppError::Conflict("Product content already exists".to_string()));
    }

    let content = queries.create(&req).await?;

    Ok(Json(ApiResponse::success(content)))
}

/// @api PUT /api/v1/products/:product_id/content
/// @desc 创建或更新产品内容
/// @param product_id: number (产品ID)
/// @body UpdateProductContentRequest
/// @response 200 ProductContent
#[instrument(skip(state))]
pub async fn upsert_product_content(
    State(state): State<AppState>,
    Path(product_id): Path<i64>,
    Json(req): Json<UpdateProductContentRequest>,
) -> AppResult<Json<ApiResponse<ProductContent>>> {
    info!("Upserting product content: product_id={}", product_id);

    req.validate().map_err(AppError::from)?;

    let queries = ProductContentQueries::new(state.db.pool());
    let content = queries.upsert(product_id, &req).await?;

    Ok(Json(ApiResponse::success(content)))
}

/// @api DELETE /api/v1/products/:product_id/content
/// @desc 删除产品内容
/// @param product_id: number (产品ID)
/// @response 200 {"code": 200, "message": "删除成功"}
/// @response 404 内容不存在
#[instrument(skip(state))]
pub async fn delete_product_content(
    State(state): State<AppState>,
    Path(product_id): Path<i64>,
) -> AppResult<Json<ApiResponse<()>>> {
    info!("Deleting product content: product_id={}", product_id);

    let queries = ProductContentQueries::new(state.db.pool());
    let deleted = queries.delete(product_id).await?;

    if !deleted {
        return Err(AppError::NotFound);
    }

    Ok(Json(ApiResponse::success_message("删除成功")))
}
