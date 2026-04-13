//! 分类 API 路由

use axum::{
    extract::{Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::state::AppState;
use cicierp_db::queries::categories::{Category, CategoryQueries};
use cicierp_utils::{AppResult, ApiResponse};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/categories", get(list_categories))
        .route("/categories", post(create_category))
}

#[derive(Debug, Deserialize)]
pub struct CategoryQuery {
    pub q: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: u32,
}
fn default_limit() -> u32 { 20 }

#[derive(Debug, Deserialize)]
pub struct CreateCategoryRequest {
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct CategoryItem {
    pub id: i64,
    pub name: String,
}

/// @api GET /api/v1/categories
/// @desc 搜索分类列表
/// @query q: string (关键字，可选)
/// @query limit: number (最大返回数，默认20)
/// @response 200 [CategoryItem]
pub async fn list_categories(
    State(state): State<AppState>,
    Query(query): Query<CategoryQuery>,
) -> AppResult<Json<ApiResponse<Vec<CategoryItem>>>> {
    let queries = CategoryQueries::new(state.db.pool());
    let cats = queries.search(query.q.as_deref(), query.limit.min(100)).await?;
    let items = cats.into_iter().map(|c| CategoryItem { id: c.id, name: c.name }).collect();
    Ok(Json(ApiResponse::success(items)))
}

/// @api POST /api/v1/categories
/// @desc 创建分类（若同名已存在则返回已有记录）
/// @body CreateCategoryRequest
/// @response 200 CategoryItem
pub async fn create_category(
    State(state): State<AppState>,
    Json(req): Json<CreateCategoryRequest>,
) -> AppResult<Json<ApiResponse<CategoryItem>>> {
    let queries = CategoryQueries::new(state.db.pool());
    let cat = queries.find_or_create(&req.name).await?;
    Ok(Json(ApiResponse::success(CategoryItem { id: cat.id, name: cat.name })))
}
