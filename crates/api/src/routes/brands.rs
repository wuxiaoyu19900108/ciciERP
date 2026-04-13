//! 品牌 API 路由

use axum::{
    extract::{Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::state::AppState;
use cicierp_db::queries::brands::{Brand, BrandQueries};
use cicierp_utils::{AppResult, ApiResponse};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/brands", get(list_brands))
        .route("/brands", post(create_brand))
}

#[derive(Debug, Deserialize)]
pub struct BrandQuery {
    pub q: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: u32,
}
fn default_limit() -> u32 { 20 }

#[derive(Debug, Deserialize)]
pub struct CreateBrandRequest {
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct BrandItem {
    pub id: i64,
    pub name: String,
}

/// @api GET /api/v1/brands
/// @desc 搜索品牌列表
/// @query q: string (关键字，可选)
/// @query limit: number (最大返回数，默认20)
/// @response 200 [BrandItem]
pub async fn list_brands(
    State(state): State<AppState>,
    Query(query): Query<BrandQuery>,
) -> AppResult<Json<ApiResponse<Vec<BrandItem>>>> {
    let queries = BrandQueries::new(state.db.pool());
    let brands = queries.search(query.q.as_deref(), query.limit.min(100)).await?;
    let items = brands.into_iter().map(|b| BrandItem { id: b.id, name: b.name }).collect();
    Ok(Json(ApiResponse::success(items)))
}

/// @api POST /api/v1/brands
/// @desc 创建品牌（若同名已存在则返回已有记录）
/// @body CreateBrandRequest
/// @response 200 BrandItem
pub async fn create_brand(
    State(state): State<AppState>,
    Json(req): Json<CreateBrandRequest>,
) -> AppResult<Json<ApiResponse<BrandItem>>> {
    let queries = BrandQueries::new(state.db.pool());
    let brand = queries.find_or_create(&req.name).await?;
    Ok(Json(ApiResponse::success(BrandItem { id: brand.id, name: brand.name })))
}
