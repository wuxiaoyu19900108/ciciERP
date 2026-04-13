//! PI (Proforma Invoice) 模块 API 路由

use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use tracing::{info, instrument};
use validator::Validate;

use crate::state::AppState;
use cicierp_db::queries::{exchange_rates::ExchangeRateQueries, orders::OrderQueries, proforma_invoices::ProformaInvoiceQueries};
use cicierp_models::{
    common::PagedResponse,
    proforma_invoice::{CreatePIRequest, PIDetail, PIListItem, PIQuery, ProformaInvoice, UpdatePIRequest},
};
use cicierp_utils::{AppError, AppResult, ApiResponse};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/proforma-invoices", get(list_pis))
        .route("/proforma-invoices", post(create_pi))
        .route("/proforma-invoices/:id", get(get_pi))
        .route("/proforma-invoices/:id", put(update_pi))
        .route("/proforma-invoices/:id", delete(delete_pi))
        .route("/proforma-invoices/:id/send", post(send_pi))
        .route("/proforma-invoices/:id/confirm", post(confirm_pi))
        .route("/proforma-invoices/:id/convert", post(convert_pi_to_order))
        .route("/proforma-invoices/:id/cancel", post(cancel_pi))
}

/// @api GET /api/v1/proforma-invoices
/// @desc 获取 PI 列表
/// @query page: number (页码，默认1)
/// @query page_size: number (每页数量，默认20)
/// @query status: number (状态，可选)
/// @query customer_id: number (客户ID，可选)
/// @query date_from: string (开始日期，可选)
/// @query date_to: string (结束日期，可选)
/// @query keyword: string (搜索关键词，可选)
/// @response 200 PagedResponse<PIListItem>
#[instrument(skip(state))]
pub async fn list_pis(
    State(state): State<AppState>,
    Query(query): Query<PIQuery>,
) -> AppResult<Json<ApiResponse<PagedResponse<PIListItem>>>> {
    info!("Listing proforma invoices");

    let queries = ProformaInvoiceQueries::new(state.db.pool());
    let result = queries.list(&query).await?;

    Ok(Json(ApiResponse::success(result)))
}

/// @api GET /api/v1/proforma-invoices/:id
/// @desc 获取 PI 详情
/// @param id: number (PI ID)
/// @response 200 PIDetail
/// @response 404 PI 不存在
#[instrument(skip(state))]
pub async fn get_pi(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<PIDetail>>> {
    info!("Getting proforma invoice: id={}", id);

    let queries = ProformaInvoiceQueries::new(state.db.pool());
    let pi = queries.get_detail(id).await?.ok_or(AppError::NotFound)?;

    Ok(Json(ApiResponse::success(pi)))
}

/// @api POST /api/v1/proforma-invoices
/// @desc 创建 PI
/// @body CreatePIRequest
/// @response 200 ProformaInvoice
#[instrument(skip(state))]
pub async fn create_pi(
    State(state): State<AppState>,
    Json(mut req): Json<CreatePIRequest>,
) -> AppResult<Json<ApiResponse<ProformaInvoice>>> {
    info!("Creating proforma invoice: customer={}", req.customer_name);

    req.validate().map_err(AppError::from)?;

    // 如果调用方没有传 exchange_rate，自动填入当前缓冲汇率快照
    if req.exchange_rate.is_none() {
        let eq = ExchangeRateQueries::new(state.db.pool());
        let rate = eq.get_latest_rate("USD", "CNY").await
            .ok().flatten().map(|r| r.rate).unwrap_or(7.2);
        req.exchange_rate = Some(((rate - 0.05) * 100.0).round() / 100.0);
    }

    let queries = ProformaInvoiceQueries::new(state.db.pool());
    let pi = queries.create(&req).await?;

    info!("PI created: id={}, code={}", pi.id, pi.pi_code);
    Ok(Json(ApiResponse::success(pi)))
}

/// @api PUT /api/v1/proforma-invoices/:id
/// @desc 更新 PI
/// @param id: number (PI ID)
/// @body UpdatePIRequest
/// @response 200 ProformaInvoice
/// @response 404 PI 不存在
#[instrument(skip(state))]
pub async fn update_pi(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdatePIRequest>,
) -> AppResult<Json<ApiResponse<ProformaInvoice>>> {
    info!("Updating proforma invoice: id={}", id);

    req.validate().map_err(AppError::from)?;

    let queries = ProformaInvoiceQueries::new(state.db.pool());
    let pi = queries.update(id, &req).await?.ok_or(AppError::NotFound)?;

    info!("PI updated: id={}", id);
    Ok(Json(ApiResponse::success(pi)))
}

/// @api DELETE /api/v1/proforma-invoices/:id
/// @desc 删除 PI（仅草稿状态可删除）
/// @param id: number (PI ID)
/// @response 200 {"code": 200, "message": "删除成功"}
/// @response 400 PI 状态不允许删除
#[instrument(skip(state))]
pub async fn delete_pi(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<()>>> {
    info!("Deleting proforma invoice: id={}", id);

    let queries = ProformaInvoiceQueries::new(state.db.pool());
    let deleted = queries.delete(id).await?;

    if !deleted {
        return Err(AppError::BadRequest("Only draft PI can be deleted".to_string()));
    }

    info!("PI deleted: id={}", id);
    Ok(Json(ApiResponse::success_message("删除成功")))
}

/// @api POST /api/v1/proforma-invoices/:id/send
/// @desc 发送 PI（状态从草稿变为已发送）
/// @param id: number (PI ID)
/// @response 200 {"code": 200, "message": "PI 已发送"}
/// @response 400 PI 状态不允许发送
#[instrument(skip(state))]
pub async fn send_pi(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<()>>> {
    info!("Sending proforma invoice: id={}", id);

    let queries = ProformaInvoiceQueries::new(state.db.pool());
    let sent = queries.send(id).await?;

    if !sent {
        return Err(AppError::BadRequest("Only draft PI can be sent".to_string()));
    }

    info!("PI sent: id={}", id);
    Ok(Json(ApiResponse::success_message("PI 已发送")))
}

/// @api POST /api/v1/proforma-invoices/:id/confirm
/// @desc 确认 PI（状态从已发送变为已确认）
/// @param id: number (PI ID)
/// @response 200 {"code": 200, "message": "PI 已确认"}
/// @response 400 PI 状态不允许确认
#[instrument(skip(state))]
pub async fn confirm_pi(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<()>>> {
    info!("Confirming proforma invoice: id={}", id);

    let queries = ProformaInvoiceQueries::new(state.db.pool());
    let confirmed = queries.confirm(id).await?;

    if !confirmed {
        return Err(AppError::BadRequest("Only sent PI can be confirmed".to_string()));
    }

    info!("PI confirmed: id={}", id);
    Ok(Json(ApiResponse::success_message("PI 已确认")))
}

/// @api POST /api/v1/proforma-invoices/:id/convert
/// @desc 将 PI 转为订单
/// @param id: number (PI ID)
/// @response 200 {"code": 200, "message": "PI 已转订单", "data": {"order_id": 1, "order_code": "ORD..."}}
/// @response 400 PI 状态不允许转换
#[instrument(skip(state))]
pub async fn convert_pi_to_order(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    info!("Converting proforma invoice to order: id={}", id);

    let pi_queries = ProformaInvoiceQueries::new(state.db.pool());
    let order_queries = OrderQueries::new(state.db.pool());

    // 获取 PI 详情
    let pi_detail = pi_queries.get_detail(id).await?.ok_or(AppError::NotFound)?;

    // 检查状态
    if pi_detail.pi.status != 3 {
        return Err(AppError::BadRequest("Only confirmed PI can be converted to order".to_string()));
    }

    // 检查是否已转换
    if pi_detail.pi.sales_order_id.is_some() {
        return Err(AppError::BadRequest("PI already converted to order".to_string()));
    }

    // 创建订单请求
    let create_order_req = cicierp_models::order::CreateOrderRequest {
        platform: "pi".to_string(),
        platform_order_id: Some(pi_detail.pi.pi_code.clone()),
        customer_id: pi_detail.pi.customer_id,
        customer_name: Some(pi_detail.pi.customer_name.clone()),
        customer_mobile: pi_detail.pi.customer_phone.clone(),
        customer_email: pi_detail.pi.customer_email.clone(),
        order_type: Some(1),
        items: pi_detail.items.iter().map(|item| {
            cicierp_models::order::OrderItemRequest {
                product_id: item.product_id,
                sku_id: None,
                product_name: item.product_name.clone(),
                product_code: None,
                sku_code: item.model.clone(),
                sku_spec: None,
                product_image: None,
                quantity: item.quantity,
                unit_price: item.unit_price,
            }
        }).collect(),
        shipping_fee: Some(0.0),
        discount_amount: Some(pi_detail.pi.discount),
        customer_note: pi_detail.pi.notes.clone(),
        receiver_name: pi_detail.pi.customer_name.clone(),
        receiver_phone: pi_detail.pi.customer_phone.clone().unwrap_or_default(),
        country: "CN".to_string(),
        province: None,
        city: None,
        district: None,
        address: pi_detail.pi.customer_address.clone().unwrap_or_default(),
        postal_code: None,
        payment_terms: Some(pi_detail.pi.payment_terms.clone()),
        delivery_terms: Some(pi_detail.pi.delivery_terms.clone()),
        lead_time: Some(pi_detail.pi.lead_time.clone()),
        exchange_rate: pi_detail.pi.exchange_rate,  // 沿用 PI 的汇率快照
        currency: Some(pi_detail.pi.currency.clone()),     // 沿用 PI 的货币
    };

    // 创建订单
    let order = order_queries.create(&create_order_req).await?;

    // 更新 PI 状态为已转订单
    pi_queries.mark_converted(id, order.id).await?;

    info!("PI converted to order: pi_id={}, order_id={}", id, order.id);

    Ok(Json(ApiResponse::success(serde_json::json!({
        "order_id": order.id,
        "order_code": order.order_code
    }))))
}

/// @api POST /api/v1/proforma-invoices/:id/cancel
/// @desc 取消 PI
/// @param id: number (PI ID)
/// @response 200 {"code": 200, "message": "PI 已取消"}
/// @response 400 PI 状态不允许取消
#[instrument(skip(state))]
pub async fn cancel_pi(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<()>>> {
    info!("Cancelling proforma invoice: id={}", id);

    let queries = ProformaInvoiceQueries::new(state.db.pool());
    let cancelled = queries.cancel(id).await?;

    if !cancelled {
        return Err(AppError::BadRequest("PI cannot be cancelled in current status".to_string()));
    }

    info!("PI cancelled: id={}", id);
    Ok(Json(ApiResponse::success_message("PI 已取消")))
}
