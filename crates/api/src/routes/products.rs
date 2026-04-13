//! 产品模块 API 路由

use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::Deserialize;
use tracing::{info, instrument};
use validator::Validate;

use crate::state::AppState;
use cicierp_db::queries::products::ProductQueries;
use cicierp_db::queries::product_prices::ProductPriceQueries;
use cicierp_db::queries::orders::OrderQueries;
use cicierp_models::common::PagedResponse;
use cicierp_models::product::{CreateProductRequest, Product, ProductDetail, ProductListItem, ProductQuery, UpdateProductRequest};
use cicierp_utils::{AppError, AppResult, ApiResponse};

/// 创建产品路由
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/products", get(list_products))
        .route("/products/search", get(search_products))
        .route("/products", post(create_product))
        .route("/products/:id", get(get_product))
        .route("/products/:id", put(update_product))
        .route("/products/:id", delete(delete_product))
        .route("/products/:id/history-prices", get(get_product_history_prices))
        .route("/products/:id/price-summary", get(get_product_price_summary))
}

/// @api GET /api/v1/products
/// @desc 获取产品列表
/// @query page: number (页码，默认1)
/// @query page_size: number (每页数量，默认20，最大100)
/// @query category_id: number (分类ID，可选)
/// @query brand_id: number (品牌ID，可选)
/// @query status: number (状态：1上架 2下架 3草稿，可选)
/// @query keyword: string (搜索关键词，可选)
/// @response 200 PagedResponse<ProductListItem>
/// @example curl -X GET "http://localhost:3000/api/v1/products?page=1&page_size=20"
#[instrument(skip(state))]
pub async fn list_products(
    State(state): State<AppState>,
    Query(query): Query<ProductQuery>,
) -> AppResult<Json<ApiResponse<PagedResponse<ProductListItem>>>> {
    info!("Listing products: page={}, page_size={}", query.page(), query.page_size());

    let queries = ProductQueries::new(state.db.pool());
    let result = queries
        .list(
            query.page(),
            query.page_size(),
            query.category_id,
            query.brand_id,
            query.status,
            query.keyword.as_deref(),
            query.supplier_id,
            query.price_min,
            query.price_max,
        )
        .await?;

    Ok(Json(ApiResponse::success(result)))
}

/// @api GET /api/v1/products/search
/// @desc 全文搜索产品（使用 SQLite FTS5）
/// @query keyword: string (搜索关键词，必填)
/// @query page: number (页码，默认1)
/// @query page_size: number (每页数量，默认20)
/// @response 200 PagedResponse<ProductListItem>
/// @example curl -X GET "http://localhost:3000/api/v1/products/search?keyword=手机"
#[instrument(skip(state))]
pub async fn search_products(
    State(state): State<AppState>,
    Query(query): Query<ProductQuery>,
) -> AppResult<Json<ApiResponse<PagedResponse<ProductListItem>>>> {
    let keyword = query.keyword.as_deref().ok_or_else(|| {
        AppError::BadRequest("keyword parameter is required".to_string())
    })?;

    info!("Searching products: keyword={}", keyword);

    let queries = ProductQueries::new(state.db.pool());
    let result = queries.search(keyword, query.page(), query.page_size()).await?;

    Ok(Json(ApiResponse::success(result)))
}

/// @api GET /api/v1/products/:id
/// @desc 获取产品详情
/// @param id: number (产品ID)
/// @response 200 ProductDetail
/// @response 404 产品不存在
/// @example curl -X GET "http://localhost:3000/api/v1/products/1"
#[instrument(skip(state))]
pub async fn get_product(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<ProductDetail>>> {
    info!("Getting product: id={}", id);

    let queries = ProductQueries::new(state.db.pool());
    let product = queries.get_by_id(id).await?.ok_or(AppError::NotFound)?;

    let detail = ProductDetail {
        product,
        category: None,
        brand: None,
    };

    Ok(Json(ApiResponse::success(detail)))
}

/// @api POST /api/v1/products
/// @desc 创建产品
/// @body CreateProductRequest
/// @response 200 Product
/// @response 400 参数错误
/// @response 409 产品编码已存在
/// @example curl -X POST "http://localhost:3000/api/v1/products" \
///   -H "Content-Type: application/json" \
///   -d '{"name":"测试产品","purchase_price":10,"sale_price":20}'
#[instrument(skip(state))]
pub async fn create_product(
    State(state): State<AppState>,
    Json(req): Json<CreateProductRequest>,
) -> AppResult<Json<ApiResponse<Product>>> {
    info!("Creating product: code={:?}", req.product_code);

    // 验证请求
    req.validate().map_err(AppError::from)?;

    let queries = ProductQueries::new(state.db.pool());

    // 如果提供了 product_code，检查编码是否已存在
    if let Some(ref code) = req.product_code {
        if queries.get_by_code(code).await?.is_some() {
            return Err(AppError::Conflict("Product code already exists".to_string()));
        }
    }

    let product = queries.create(&req).await?;
    info!("Product created: id={}, code={}", product.id, product.product_code);

    Ok(Json(ApiResponse::success(product)))
}

/// @api PUT /api/v1/products/:id
/// @desc 更新产品
/// @param id: number (产品ID)
/// @body UpdateProductRequest
/// @response 200 Product
/// @response 404 产品不存在
/// @example curl -X PUT "http://localhost:3000/api/v1/products/1" \
///   -H "Content-Type: application/json" \
///   -d '{"name":"新名称"}'
#[instrument(skip(state))]
pub async fn update_product(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateProductRequest>,
) -> AppResult<Json<ApiResponse<Product>>> {
    info!("Updating product: id={}", id);

    // 添加输入验证
    req.validate().map_err(AppError::from)?;

    let queries = ProductQueries::new(state.db.pool());
    let product = queries.update(id, &req).await?.ok_or(AppError::NotFound)?;

    info!("Product updated: id={}", id);
    Ok(Json(ApiResponse::success(product)))
}

/// @api DELETE /api/v1/products/:id
/// @desc 删除产品（软删除）
/// @param id: number (产品ID)
/// @response 200 {"code": 200, "message": "删除成功"}
/// @response 404 产品不存在
/// @example curl -X DELETE "http://localhost:3000/api/v1/products/1"
#[instrument(skip(state))]
pub async fn delete_product(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<()>>> {
    info!("Deleting product: id={}", id);

    let queries = ProductQueries::new(state.db.pool());
    let deleted = queries.delete(id).await?;

    if !deleted {
        return Err(AppError::NotFound);
    }

    info!("Product deleted: id={}", id);
    Ok(Json(ApiResponse::success_message("删除成功")))
}

/// @api GET /api/v1/products/:id/history-prices
/// @desc 获取产品历史成交价格
/// @param id: number (产品ID)
/// @query limit: number (返回条数，默认10，最大50)
/// @response 200 Vec<ProductHistoryPrice>
/// @response 404 产品不存在
/// @example curl -X GET "http://localhost:3000/api/v1/products/1/history-prices?limit=10"
#[instrument(skip(state))]
pub async fn get_product_history_prices(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Query(query): Query<HistoryPriceQuery>,
) -> AppResult<Json<ApiResponse<Vec<cicierp_db::queries::orders::ProductHistoryPrice>>>> {
    info!("Getting product history prices: id={}", id);

    // 先检查产品是否存在
    let product_queries = ProductQueries::new(state.db.pool());
    let _ = product_queries.get_by_id(id).await?.ok_or(AppError::NotFound)?;

    // 获取历史价格
    let order_queries = OrderQueries::new(state.db.pool());
    let limit = query.limit.unwrap_or(10).min(50).max(1);
    let prices = order_queries.get_product_history_prices(id, limit).await?;

    Ok(Json(ApiResponse::success(prices)))
}

/// 历史价格查询参数
#[derive(Debug, Deserialize)]
pub struct HistoryPriceQuery {
    pub limit: Option<u32>,
}

/// @api GET /api/v1/products/:id/price-summary
/// @desc 获取产品价格统计（包含参考价格）
/// @param id: number (产品ID)
/// @response 200 ProductPriceSummary
/// @response 404 产品不存在
/// @example curl -X GET "http://localhost:3000/api/v1/products/1/price-summary"
#[instrument(skip(state))]
pub async fn get_product_price_summary(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<cicierp_models::product::ProductPriceSummary>>> {
    info!("Getting product price summary: id={}", id);

    // 先检查产品是否存在
    let product_queries = ProductQueries::new(state.db.pool());
    let _ = product_queries.get_by_id(id).await?.ok_or(AppError::NotFound)?;

    // 获取价格统计
    let price_queries = ProductPriceQueries::new(state.db.pool());
    let summary = price_queries.get_price_summary(id).await?;

    Ok(Json(ApiResponse::success(summary)))
}
