//! 库存模块 API 路由

use axum::{
    extract::{Path, Query, State},
    routing::{get, post, put},
    Json, Router,
};
use tracing::{info, instrument};

use crate::state::AppState;
use cicierp_db::queries::inventory::InventoryQueries;
use cicierp_models::common::PagedResponse;
use cicierp_models::inventory::{Inventory, InventoryAlert, InventoryListItem, InventoryQuery, LockInventoryRequest, UnlockInventoryRequest, UpdateInventoryRequest};
use cicierp_utils::{AppError, AppResult, ApiResponse};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/inventory", get(list_inventory))
        .route("/inventory/alerts", get(get_inventory_alerts))
        .route("/inventory/:sku_id", get(get_inventory))
        .route("/inventory/:sku_id", put(update_inventory))
        .route("/inventory/lock", post(lock_inventory))
        .route("/inventory/unlock", post(unlock_inventory))
}

/// @api GET /api/v1/inventory
/// @desc 获取库存列表
/// @query page: number (页码，默认1)
/// @query page_size: number (每页数量，默认20)
/// @query low_stock: boolean (只显示低库存，可选)
/// @query sku_code: string (SKU编码，可选)
/// @query product_name: string (产品名称，可选)
/// @response 200 PagedResponse<InventoryListItem>
/// @example curl -X GET "http://localhost:3000/api/v1/inventory?page=1&page_size=20"
#[instrument(skip(state))]
pub async fn list_inventory(
    State(state): State<AppState>,
    Query(query): Query<InventoryQuery>,
) -> AppResult<Json<ApiResponse<PagedResponse<InventoryListItem>>>> {
    info!("Listing inventory");

    let queries = InventoryQueries::new(state.db.pool());
    let result = queries
        .list(
            query.page(),
            query.page_size(),
            query.low_stock,
            query.sku_code.as_deref(),
            query.product_name.as_deref(),
        )
        .await?;

    Ok(Json(ApiResponse::success(result)))
}

/// @api GET /api/v1/inventory/:sku_id
/// @desc 获取指定 SKU 的库存
/// @param sku_id: number (SKU ID)
/// @response 200 Inventory
/// @response 404 库存记录不存在
/// @example curl -X GET "http://localhost:3000/api/v1/inventory/1"
#[instrument(skip(state))]
pub async fn get_inventory(
    State(state): State<AppState>,
    Path(sku_id): Path<i64>,
) -> AppResult<Json<ApiResponse<Inventory>>> {
    info!("Getting inventory: sku_id={}", sku_id);

    let queries = InventoryQueries::new(state.db.pool());
    let inventory = queries.get_by_sku(sku_id).await?.ok_or(AppError::NotFound)?;

    Ok(Json(ApiResponse::success(inventory)))
}

/// @api PUT /api/v1/inventory/:sku_id
/// @desc 更新库存数量
/// @param sku_id: number (SKU ID)
/// @body UpdateInventoryRequest
/// @response 200 Inventory
/// @example curl -X PUT "http://localhost:3000/api/v1/inventory/1" \
///   -H "Content-Type: application/json" \
///   -d '{"quantity":100,"note":"盘点调整"}'
#[instrument(skip(state))]
pub async fn update_inventory(
    State(state): State<AppState>,
    Path(sku_id): Path<i64>,
    Json(req): Json<UpdateInventoryRequest>,
) -> AppResult<Json<ApiResponse<Inventory>>> {
    info!("Updating inventory: sku_id={}, quantity={}", sku_id, req.quantity);

    let queries = InventoryQueries::new(state.db.pool());
    let inventory = queries.update(sku_id, &req, None).await?.ok_or(AppError::NotFound)?;

    info!("Inventory updated: sku_id={}", sku_id);
    Ok(Json(ApiResponse::success(inventory)))
}

/// @api POST /api/v1/inventory/lock
/// @desc 锁定库存（下单时使用）
/// @body LockInventoryRequest
/// @response 200 {"code": 200, "message": "锁定成功"}
/// @response 400 库存不足
/// @example curl -X POST "http://localhost:3000/api/v1/inventory/lock" \
///   -H "Content-Type: application/json" \
///   -d '{"sku_id":1,"quantity":10}'
#[instrument(skip(state))]
pub async fn lock_inventory(
    State(state): State<AppState>,
    Json(req): Json<LockInventoryRequest>,
) -> AppResult<Json<ApiResponse<()>>> {
    info!("Locking inventory: sku_id={}, quantity={}", req.sku_id, req.quantity);

    let queries = InventoryQueries::new(state.db.pool());
    let locked = queries.lock(req.sku_id, req.quantity, req.order_id).await?;

    if !locked {
        return Err(AppError::BadRequest("Insufficient inventory".to_string()));
    }

    info!("Inventory locked: sku_id={}", req.sku_id);
    Ok(Json(ApiResponse::success_message("锁定成功")))
}

/// @api POST /api/v1/inventory/unlock
/// @desc 解锁库存（订单取消时使用）
/// @body UnlockInventoryRequest
/// @response 200 {"code": 200, "message": "解锁成功"}
/// @response 400 没有足够的锁定库存
/// @example curl -X POST "http://localhost:3000/api/v1/inventory/unlock" \
///   -H "Content-Type: application/json" \
///   -d '{"sku_id":1,"quantity":10}'
#[instrument(skip(state))]
pub async fn unlock_inventory(
    State(state): State<AppState>,
    Json(req): Json<UnlockInventoryRequest>,
) -> AppResult<Json<ApiResponse<()>>> {
    info!("Unlocking inventory: sku_id={}, quantity={}", req.sku_id, req.quantity);

    let queries = InventoryQueries::new(state.db.pool());
    let unlocked = queries.unlock(req.sku_id, req.quantity, req.order_id).await?;

    if !unlocked {
        return Err(AppError::BadRequest("No locked inventory to unlock".to_string()));
    }

    info!("Inventory unlocked: sku_id={}", req.sku_id);
    Ok(Json(ApiResponse::success_message("解锁成功")))
}

/// @api GET /api/v1/inventory/alerts
/// @desc 获取库存预警列表
/// @response 200 [InventoryAlert]
/// @example curl -X GET "http://localhost:3000/api/v1/inventory/alerts"
#[instrument(skip(state))]
pub async fn get_inventory_alerts(
    State(state): State<AppState>,
) -> AppResult<Json<ApiResponse<Vec<InventoryAlert>>>> {
    info!("Getting inventory alerts");

    let queries = InventoryQueries::new(state.db.pool());
    let alerts = queries.get_alerts().await?;

    Ok(Json(ApiResponse::success(alerts)))
}
