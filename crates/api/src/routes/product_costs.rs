//! 产品成本 API 路由

use axum::{
    extract::{Path, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use tracing::{info, instrument};
use validator::Validate;

use crate::state::AppState;
use cicierp_db::queries::product_costs::ProductCostQueries;
use cicierp_models::product::{CreateProductCostRequest, ProductCost, UpdateProductCostRequest};
use cicierp_utils::{AppError, AppResult, ApiResponse};

/// 创建产品成本路由
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/products/:product_id/cost", get(get_product_cost))
        .route("/products/:product_id/cost", post(create_product_cost))
        .route("/products/:product_id/cost", put(update_product_cost))
        .route("/products/:product_id/cost", delete(delete_product_cost))
        .route("/products/:product_id/costs", get(list_product_costs))
        .route("/product-costs/:id", get(get_cost_by_id))
        .route("/product-costs/:id", put(update_cost_by_id))
        .route("/product-costs/:id", delete(delete_cost_by_id))
}

/// @api GET /api/v1/products/:product_id/cost
/// @desc 获取产品当前成本
/// @param product_id: number (产品ID)
/// @response 200 ProductCost
/// @response 404 成本不存在
#[instrument(skip(state))]
pub async fn get_product_cost(
    State(state): State<AppState>,
    Path(product_id): Path<i64>,
) -> AppResult<Json<ApiResponse<ProductCost>>> {
    info!("Getting product cost: product_id={}", product_id);

    let queries = ProductCostQueries::new(state.db.pool());
    let cost = queries.get_by_product_id(product_id).await?.ok_or(AppError::NotFound)?;

    Ok(Json(ApiResponse::success(cost)))
}

/// @api GET /api/v1/products/:product_id/costs
/// @desc 获取产品成本历史
/// @param product_id: number (产品ID)
/// @response 200 Vec<ProductCost>
#[instrument(skip(state))]
pub async fn list_product_costs(
    State(state): State<AppState>,
    Path(product_id): Path<i64>,
) -> AppResult<Json<ApiResponse<Vec<ProductCost>>>> {
    info!("Listing product costs: product_id={}", product_id);

    let queries = ProductCostQueries::new(state.db.pool());
    let costs = queries.list_by_product(product_id).await?;

    Ok(Json(ApiResponse::success(costs)))
}

/// @api POST /api/v1/products/:product_id/cost
/// @desc 创建产品成本
/// @param product_id: number (产品ID)
/// @body CreateProductCostRequest
/// @response 200 ProductCost
#[instrument(skip(state))]
pub async fn create_product_cost(
    State(state): State<AppState>,
    Path(product_id): Path<i64>,
    Json(mut req): Json<CreateProductCostRequest>,
) -> AppResult<Json<ApiResponse<ProductCost>>> {
    info!("Creating product cost: product_id={}", product_id);

    req.validate().map_err(AppError::from)?;
    req.product_id = product_id;

    let queries = ProductCostQueries::new(state.db.pool());
    let cost = queries.create(&req).await?;

    Ok(Json(ApiResponse::success(cost)))
}

/// @api PUT /api/v1/products/:product_id/cost
/// @desc 更新产品当前成本
/// @param product_id: number (产品ID)
/// @body UpdateProductCostRequest
/// @response 200 ProductCost
/// @response 404 成本不存在
#[instrument(skip(state))]
pub async fn update_product_cost(
    State(state): State<AppState>,
    Path(product_id): Path<i64>,
    Json(req): Json<UpdateProductCostRequest>,
) -> AppResult<Json<ApiResponse<ProductCost>>> {
    info!("Updating product cost: product_id={}", product_id);

    req.validate().map_err(AppError::from)?;

    let queries = ProductCostQueries::new(state.db.pool());

    // 获取当前成本记录
    let current_cost = queries.get_by_product_id(product_id).await?.ok_or(AppError::NotFound)?;

    // 更新成本
    let cost = queries.update(current_cost.id, &req).await?.ok_or(AppError::NotFound)?;

    Ok(Json(ApiResponse::success(cost)))
}

/// @api DELETE /api/v1/products/:product_id/cost
/// @desc 删除产品所有成本记录
/// @param product_id: number (产品ID)
/// @response 200 {"code": 200, "message": "删除成功"}
#[instrument(skip(state))]
pub async fn delete_product_cost(
    State(state): State<AppState>,
    Path(product_id): Path<i64>,
) -> AppResult<Json<ApiResponse<()>>> {
    info!("Deleting product costs: product_id={}", product_id);

    let queries = ProductCostQueries::new(state.db.pool());
    queries.delete_by_product(product_id).await?;

    Ok(Json(ApiResponse::success_message("删除成功")))
}

/// @api GET /api/v1/product-costs/:id
/// @desc 根据ID获取成本记录
/// @param id: number (成本ID)
/// @response 200 ProductCost
/// @response 404 成本不存在
#[instrument(skip(state))]
pub async fn get_cost_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<ProductCost>>> {
    info!("Getting cost by id: id={}", id);

    let queries = ProductCostQueries::new(state.db.pool());
    let cost = queries.get_by_id(id).await?.ok_or(AppError::NotFound)?;

    Ok(Json(ApiResponse::success(cost)))
}

/// @api PUT /api/v1/product-costs/:id
/// @desc 根据ID更新成本记录
/// @param id: number (成本ID)
/// @body UpdateProductCostRequest
/// @response 200 ProductCost
/// @response 404 成本不存在
#[instrument(skip(state))]
pub async fn update_cost_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateProductCostRequest>,
) -> AppResult<Json<ApiResponse<ProductCost>>> {
    info!("Updating cost by id: id={}", id);

    req.validate().map_err(AppError::from)?;

    let queries = ProductCostQueries::new(state.db.pool());
    let cost = queries.update(id, &req).await?.ok_or(AppError::NotFound)?;

    Ok(Json(ApiResponse::success(cost)))
}

/// @api DELETE /api/v1/product-costs/:id
/// @desc 根据ID删除成本记录
/// @param id: number (成本ID)
/// @response 200 {"code": 200, "message": "删除成功"}
/// @response 404 成本不存在
#[instrument(skip(state))]
pub async fn delete_cost_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<()>>> {
    info!("Deleting cost by id: id={}", id);

    let queries = ProductCostQueries::new(state.db.pool());
    let deleted = queries.delete(id).await?;

    if !deleted {
        return Err(AppError::NotFound);
    }

    Ok(Json(ApiResponse::success_message("删除成功")))
}
