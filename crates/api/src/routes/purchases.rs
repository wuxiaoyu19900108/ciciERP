//! 采购模块 API 路由

use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Extension, Json, Router,
};
use tracing::{info, warn};
use validator::Validate;

use crate::middleware::auth::AuthUser;
use crate::state::AppState;
use cicierp_db::queries::{purchases::PurchaseQueries, suppliers::SupplierQueries};
use cicierp_models::common::PagedResponse;
use cicierp_models::purchase::{
    ApprovePurchaseRequest, CreatePurchaseOrderRequest, PurchaseOrderDetail, PurchaseOrderListItem,
    PurchaseQuery, ReceivePurchaseRequest, UpdatePurchaseOrderRequest,
};
use cicierp_utils::{AppError, AppResult, ApiResponse};

/// 创建采购模块路由
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/purchases", get(list_purchases))
        .route("/purchases", post(create_purchase))
        .route("/purchases/:id", get(get_purchase))
        .route("/purchases/:id", put(update_purchase))
        .route("/purchases/:id", delete(delete_purchase))
        .route("/purchases/:id/approve", post(approve_purchase))
        .route("/purchases/:id/receive", post(receive_purchase))
}

/// @api GET /api/v1/purchases
/// @desc 获取采购单列表
/// @query page, page_size, supplier_id, status, payment_status, delivery_status, keyword
/// @response 200 PagedResponse<PurchaseOrderListItem>
/// @example curl -X GET "http://localhost:3000/api/v1/purchases?page=1&page_size=20"
pub async fn list_purchases(
    State(state): State<AppState>,
    _auth_user: Extension<AuthUser>,
    Query(query): Query<PurchaseQuery>,
) -> AppResult<Json<ApiResponse<PagedResponse<PurchaseOrderListItem>>>> {
    info!(
        "List purchases: page={}, page_size={}",
        query.page(),
        query.page_size()
    );

    let queries = PurchaseQueries::new(state.db.pool());
    let (items, total) = queries.list(query.page(), query.page_size(), &query).await?;

    let response = PagedResponse::new(items, query.page(), query.page_size(), total);
    Ok(Json(ApiResponse::success(response)))
}

/// @api GET /api/v1/purchases/:id
/// @desc 获取采购单详情
/// @param id: number (采购单ID)
/// @response 200 PurchaseOrderDetail
/// @response 404 采购单不存在
/// @example curl -X GET "http://localhost:3000/api/v1/purchases/1"
pub async fn get_purchase(
    State(state): State<AppState>,
    _auth_user: Extension<AuthUser>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<PurchaseOrderDetail>>> {
    info!("Get purchase: id={}", id);

    let queries = PurchaseQueries::new(state.db.pool());
    let detail = queries.get_detail(id).await?.ok_or(AppError::NotFound)?;

    Ok(Json(ApiResponse::success(detail)))
}

/// @api POST /api/v1/purchases
/// @desc 创建采购单（一单多供应商模式）
/// @body CreatePurchaseOrderRequest
/// @response 200 PurchaseOrder
/// @response 400 参数错误
/// @example curl -X POST "http://localhost:3000/api/v1/purchases" \
///   -H "Content-Type: application/json" \
///   -d '{"items":[{"product_name":"产品A","quantity":100,"unit_price":10,"supplier_id":1}]}'
pub async fn create_purchase(
    State(state): State<AppState>,
    _auth_user: Extension<AuthUser>,
    Json(req): Json<CreatePurchaseOrderRequest>,
) -> AppResult<Json<ApiResponse<cicierp_models::purchase::PurchaseOrder>>> {
    info!("Create purchase with {} items", req.items.len());

    // 验证请求
    req.validate().map_err(|e| {
        warn!("Create purchase validation failed: {}", e);
        AppError::BadRequest(e.to_string())
    })?;

    // 验证所有供应商是否存在
    let supplier_queries = SupplierQueries::new(state.db.pool());
    for item in &req.items {
        let supplier = supplier_queries.get_by_id(item.supplier_id).await?
            .ok_or_else(|| AppError::BadRequest(format!("供应商 {} 不存在", item.supplier_id)))?;
        if supplier.status != 1 {
            return Err(AppError::BadRequest(format!("供应商 {} 已暂停或终止合作", item.supplier_id)));
        }
    }

    let queries = PurchaseQueries::new(state.db.pool());
    let order = queries.create(&req).await?;

    info!("Purchase created: id={}, code={}", order.id, order.order_code);
    Ok(Json(ApiResponse::success(order)))
}

/// @api PUT /api/v1/purchases/:id
/// @desc 更新采购单（仅待审核状态可更新）
/// @param id: number (采购单ID)
/// @body UpdatePurchaseOrderRequest
/// @response 200 PurchaseOrder
/// @response 404 采购单不存在
/// @example curl -X PUT "http://localhost:3000/api/v1/purchases/1" \
///   -H "Content-Type: application/json" \
///   -d '{"internal_note":"备注信息"}'
pub async fn update_purchase(
    State(state): State<AppState>,
    _auth_user: Extension<AuthUser>,
    Path(id): Path<i64>,
    Json(req): Json<UpdatePurchaseOrderRequest>,
) -> AppResult<Json<ApiResponse<cicierp_models::purchase::PurchaseOrder>>> {
    info!("Update purchase: id={}", id);

    let queries = PurchaseQueries::new(state.db.pool());

    // 检查采购单是否存在且状态为待审核
    let existing = queries.get_by_id(id).await?.ok_or(AppError::NotFound)?;
    if existing.status != 1 {
        return Err(AppError::BadRequest("只能修改待审核的采购单".to_string()));
    }

    let order = queries.update(id, &req).await?.ok_or(AppError::NotFound)?;

    info!("Purchase updated: id={}", id);
    Ok(Json(ApiResponse::success(order)))
}

/// @api DELETE /api/v1/purchases/:id
/// @desc 删除采购单（仅待审核状态可删除）
/// @param id: number (采购单ID)
/// @response 200 { "code": 200, "message": "删除成功" }
/// @response 404 采购单不存在
/// @example curl -X DELETE "http://localhost:3000/api/v1/purchases/1"
pub async fn delete_purchase(
    State(state): State<AppState>,
    _auth_user: Extension<AuthUser>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<()>>> {
    info!("Delete purchase: id={}", id);

    let queries = PurchaseQueries::new(state.db.pool());
    let deleted = queries.delete(id).await?;

    if !deleted {
        return Err(AppError::BadRequest("只能删除待审核的采购单".to_string()));
    }

    info!("Purchase deleted: id={}", id);
    Ok(Json(ApiResponse::success_message("删除成功")))
}

/// @api POST /api/v1/purchases/:id/approve
/// @desc 审批采购单
/// @param id: number (采购单ID)
/// @body ApprovePurchaseRequest
/// @response 200 { "code": 200, "message": "审批成功" }
/// @response 400 采购单状态不正确
/// @example curl -X POST "http://localhost:3000/api/v1/purchases/1/approve" \
///   -H "Content-Type: application/json" \
///   -d '{"approval_note":"同意采购"}'
pub async fn approve_purchase(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<i64>,
    Json(req): Json<ApprovePurchaseRequest>,
) -> AppResult<Json<ApiResponse<()>>> {
    info!("Approve purchase: id={}", id);

    let queries = PurchaseQueries::new(state.db.pool());
    let approved = queries.approve(id, auth_user.user_id, &req).await?;

    if !approved {
        return Err(AppError::BadRequest("审批失败：采购单状态不正确".to_string()));
    }

    info!("Purchase approved: id={}", id);
    Ok(Json(ApiResponse::success_message("审批成功")))
}

/// @api POST /api/v1/purchases/:id/receive
/// @desc 采购入库
/// @param id: number (采购单ID)
/// @body ReceivePurchaseRequest
/// @response 200 { "code": 200, "message": "入库成功" }
/// @response 400 入库失败
/// @example curl -X POST "http://localhost:3000/api/v1/purchases/1/receive" \
///   -H "Content-Type: application/json" \
///   -d '{"sku_id":1,"received_qty":100,"qualified_qty":98,"defective_qty":2}'
pub async fn receive_purchase(
    State(state): State<AppState>,
    _auth_user: Extension<AuthUser>,
    Path(id): Path<i64>,
    Json(req): Json<ReceivePurchaseRequest>,
) -> AppResult<Json<ApiResponse<()>>> {
    info!(
        "Receive purchase: order_id={}, sku_id={}, qty={}",
        id, req.sku_id, req.received_qty
    );

    // 验证请求
    req.validate().map_err(|e| {
        warn!("Receive purchase validation failed: {}", e);
        AppError::BadRequest(e.to_string())
    })?;

    let queries = PurchaseQueries::new(state.db.pool());

    // 检查采购单状态
    let order = queries.get_by_id(id).await?.ok_or(AppError::NotFound)?;
    if order.status != 2 && order.status != 3 {
        return Err(AppError::BadRequest("采购单未审批或已完成".to_string()));
    }

    let received = queries.receive(id, &req).await?;

    if !received {
        return Err(AppError::BadRequest("入库失败：SKU不在采购单中".to_string()));
    }

    info!("Purchase received: order_id={}, sku_id={}", id, req.sku_id);
    Ok(Json(ApiResponse::success_message("入库成功")))
}
