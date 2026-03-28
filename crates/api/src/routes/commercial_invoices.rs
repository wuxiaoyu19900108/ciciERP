//! CI (Commercial Invoice) 模块 API 路由

use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use tracing::{info, instrument};
use validator::Validate;

use crate::state::AppState;
use cicierp_db::queries::commercial_invoices::CommercialInvoiceQueries;
use cicierp_models::{
    commercial_invoice::{CIDetail, CIListItem, CIQuery, CommercialInvoice, CreateCIFromOrderRequest, MarkPaidRequest},
    common::PagedResponse,
};
use cicierp_utils::{AppError, AppResult, ApiResponse};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/commercial-invoices", get(list_cis))
        .route("/commercial-invoices/:id", get(get_ci))
        .route("/commercial-invoices/from-order/:order_id", post(create_ci_from_order))
        .route("/commercial-invoices/:id/send", post(send_ci))
        .route("/commercial-invoices/:id/mark-paid", post(mark_ci_paid))
}

/// @api GET /api/v1/commercial-invoices
/// @desc 获取 CI 列表
/// @query page: number (页码，默认1)
/// @query page_size: number (每页数量，默认20)
/// @query status: number (状态，可选)
/// @query order_id: number (订单ID，可选)
/// @query customer_id: number (客户ID，可选)
/// @query date_from: string (开始日期，可选)
/// @query date_to: string (结束日期，可选)
/// @query keyword: string (搜索关键词，可选)
/// @response 200 PagedResponse<CIListItem>
#[instrument(skip(state))]
pub async fn list_cis(
    State(state): State<AppState>,
    Query(query): Query<CIQuery>,
) -> AppResult<Json<ApiResponse<PagedResponse<CIListItem>>>> {
    info!("Listing commercial invoices");

    let queries = CommercialInvoiceQueries::new(state.db.pool());
    let result = queries.list(&query).await?;

    Ok(Json(ApiResponse::success(result)))
}

/// @api GET /api/v1/commercial-invoices/:id
/// @desc 获取 CI 详情
/// @param id: number (CI ID)
/// @response 200 CIDetail
/// @response 404 CI 不存在
#[instrument(skip(state))]
pub async fn get_ci(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<CIDetail>>> {
    info!("Getting commercial invoice: id={}", id);

    let queries = CommercialInvoiceQueries::new(state.db.pool());
    let ci = queries.get_detail(id).await?.ok_or(AppError::NotFound)?;

    Ok(Json(ApiResponse::success(ci)))
}

/// @api POST /api/v1/commercial-invoices/from-order/:order_id
/// @desc 从订单创建 CI
/// @param order_id: number (订单ID)
/// @body CreateCIFromOrderRequest
/// @response 200 CommercialInvoice
/// @response 404 订单不存在
/// @response 400 订单已创建 CI
#[instrument(skip(state))]
pub async fn create_ci_from_order(
    State(state): State<AppState>,
    Path(order_id): Path<i64>,
    Json(req): Json<CreateCIFromOrderRequest>,
) -> AppResult<Json<ApiResponse<CommercialInvoice>>> {
    info!("Creating commercial invoice from order: order_id={}", order_id);

    req.validate().map_err(AppError::from)?;

    let queries = CommercialInvoiceQueries::new(state.db.pool());

    // 检查是否已创建 CI
    if queries.get_by_order_id(order_id).await?.is_some() {
        return Err(AppError::BadRequest("CI already exists for this order".to_string()));
    }

    let ci = queries.create_from_order(order_id, &req).await?;

    info!("CI created from order: ci_id={}, ci_code={}", ci.id, ci.ci_code);
    Ok(Json(ApiResponse::success(ci)))
}

/// @api POST /api/v1/commercial-invoices/:id/send
/// @desc 发送 CI（状态从草稿变为已发送）
/// @param id: number (CI ID)
/// @response 200 {"code": 200, "message": "CI 已发送"}
/// @response 400 CI 状态不允许发送
#[instrument(skip(state))]
pub async fn send_ci(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<()>>> {
    info!("Sending commercial invoice: id={}", id);

    let queries = CommercialInvoiceQueries::new(state.db.pool());
    let sent = queries.send(id).await?;

    if !sent {
        return Err(AppError::BadRequest("Only draft CI can be sent".to_string()));
    }

    info!("CI sent: id={}", id);
    Ok(Json(ApiResponse::success_message("CI 已发送")))
}

/// @api POST /api/v1/commercial-invoices/:id/mark-paid
/// @desc 标记 CI 已付款
/// @param id: number (CI ID)
/// @body MarkPaidRequest
/// @response 200 {"code": 200, "message": "CI 已标记为已付款"}
/// @response 400 CI 状态不允许标记
#[instrument(skip(state))]
pub async fn mark_ci_paid(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<MarkPaidRequest>,
) -> AppResult<Json<ApiResponse<()>>> {
    info!("Marking commercial invoice as paid: id={}, amount={}", id, req.paid_amount);

    req.validate().map_err(AppError::from)?;

    let queries = CommercialInvoiceQueries::new(state.db.pool());
    let marked = queries.mark_paid(id, &req).await?;

    if !marked {
        return Err(AppError::BadRequest("CI cannot be marked as paid in current status".to_string()));
    }

    info!("CI marked as paid: id={}", id);
    Ok(Json(ApiResponse::success_message("CI 已标记为已付款")))
}
