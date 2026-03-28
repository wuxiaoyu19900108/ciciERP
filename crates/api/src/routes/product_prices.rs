//! 产品销售价格 API 路由

use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use tracing::{info, instrument};
use validator::Validate;

use crate::state::AppState;
use cicierp_db::queries::product_prices::ProductPriceQueries;
use cicierp_models::product::{
    CreateProductPriceRequest, PriceQuery, ProductPrice, ProductPriceSummary, UpdateProductPriceRequest,
};
use cicierp_utils::{AppError, AppResult, ApiResponse};

/// 创建产品价格路由
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/products/:id/prices", get(list_product_prices))
        .route("/products/:id/prices/summary", get(get_price_summary))
        .route("/products/:id/prices", post(create_product_price))
        .route("/products/:product_id/prices/:price_id", put(update_product_price))
        .route("/products/:product_id/prices/:price_id", delete(delete_product_price))
}

/// @api GET /api/v1/products/:id/prices
/// @desc 获取产品的所有价格记录
/// @param id: number (产品ID)
/// @query platform: string (平台过滤，可选)
/// @query is_reference: boolean (只看参考价，可选)
/// @response 200 Vec<ProductPrice>
#[instrument(skip(state))]
pub async fn list_product_prices(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Query(query): Query<PriceQuery>,
) -> AppResult<Json<ApiResponse<Vec<ProductPrice>>>> {
    info!("Listing prices for product: id={}", id);

    let queries = ProductPriceQueries::new(state.db.pool());
    let mut prices = queries.list_by_product(id).await?;

    // 根据查询条件过滤
    if let Some(ref platform) = query.platform {
        prices.retain(|p| &p.platform == platform);
    }
    if let Some(is_ref) = query.is_reference {
        prices.retain(|p| p.is_reference == is_ref);
    }

    Ok(Json(ApiResponse::success(prices)))
}

/// @api GET /api/v1/products/:id/prices/summary
/// @desc 获取产品的价格统计（参考成本、平均成本、参考售价等）
/// @param id: number (产品ID)
/// @response 200 ProductPriceSummary
#[instrument(skip(state))]
pub async fn get_price_summary(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<ProductPriceSummary>>> {
    info!("Getting price summary for product: id={}", id);

    let queries = ProductPriceQueries::new(state.db.pool());
    let summary = queries.get_price_summary(id).await?;

    Ok(Json(ApiResponse::success(summary)))
}

/// @api POST /api/v1/products/:id/prices
/// @desc 创建产品价格记录
/// @param id: number (产品ID)
/// @body CreateProductPriceRequest
/// @response 200 ProductPrice
#[instrument(skip(state))]
pub async fn create_product_price(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(mut req): Json<CreateProductPriceRequest>,
) -> AppResult<Json<ApiResponse<ProductPrice>>> {
    info!("Creating price for product: id={}, platform={:?}", id, req.platform);

    req.validate().map_err(AppError::from)?;
    req.product_id = id;  // 确保 product_id 与路径参数一致

    let queries = ProductPriceQueries::new(state.db.pool());

    // 如果设置为参考价，检查是否已存在参考价
    if req.is_reference.unwrap_or(false) {
        let platform = req.platform.as_deref().unwrap_or("website");
        if queries.get_reference_price(id, platform).await?.is_some() {
            return Err(AppError::Conflict(
                format!("Reference price already exists for platform: {}", platform)
            ));
        }
    }

    let price = queries.create(&req).await?;
    info!("Price created: id={}", price.id);

    Ok(Json(ApiResponse::success(price)))
}

/// @api PUT /api/v1/products/:product_id/prices/:price_id
/// @desc 更新产品价格记录
/// @param product_id: number (产品ID)
/// @param price_id: number (价格ID)
/// @body UpdateProductPriceRequest
/// @response 200 ProductPrice
#[instrument(skip(state))]
pub async fn update_product_price(
    State(state): State<AppState>,
    Path((product_id, price_id)): Path<(i64, i64)>,
    Json(req): Json<UpdateProductPriceRequest>,
) -> AppResult<Json<ApiResponse<ProductPrice>>> {
    info!("Updating price: product_id={}, price_id={}", product_id, price_id);

    req.validate().map_err(AppError::from)?;

    let queries = ProductPriceQueries::new(state.db.pool());
    let price = queries.update(price_id, &req).await?.ok_or(AppError::NotFound)?;

    info!("Price updated: id={}", price_id);
    Ok(Json(ApiResponse::success(price)))
}

/// @api DELETE /api/v1/products/:product_id/prices/:price_id
/// @desc 删除产品价格记录
/// @param product_id: number (产品ID)
/// @param price_id: number (价格ID)
/// @response 200 {"code": 200, "message": "删除成功"}
#[instrument(skip(state))]
pub async fn delete_product_price(
    State(state): State<AppState>,
    Path((product_id, price_id)): Path<(i64, i64)>,
) -> AppResult<Json<ApiResponse<()>>> {
    info!("Deleting price: product_id={}, price_id={}", product_id, price_id);

    let queries = ProductPriceQueries::new(state.db.pool());
    let deleted = queries.delete(price_id).await?;

    if !deleted {
        return Err(AppError::NotFound);
    }

    info!("Price deleted: id={}", price_id);
    Ok(Json(ApiResponse::success_message("删除成功")))
}
