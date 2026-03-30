//! 订单模块 API 路由

use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Response},
    routing::{get, post, put},
    Json, Router,
};
use rust_xlsxwriter::{Format, Workbook, XlsxError};
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};
use validator::Validate;

use crate::state::AppState;
use cicierp_db::queries::orders::OrderQueries;
use cicierp_models::common::PagedResponse;
use cicierp_models::order::{CancelOrderRequest, CreateOrderRequest, Order, OrderDetail, OrderListItem, OrderQuery, ShipOrderRequest, UpdateOrderRequest};
use cicierp_utils::{AppError, AppResult, ApiResponse};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/orders", get(list_orders))
        .route("/orders", post(create_order))
        .route("/orders/:id", get(get_order))
        .route("/orders/:id", put(update_order))
        .route("/orders/:id/status", post(update_order_status))
        .route("/orders/:id/ship", post(ship_order))
        .route("/orders/:id/cancel", post(cancel_order))
        .route("/orders/:id/download-pi", get(download_pi))
        .route("/orders/:id/download-ci", get(download_ci))
}

/// @api GET /api/v1/orders
/// @desc 获取订单列表
/// @query page: number (页码，默认1)
/// @query page_size: number (每页数量，默认20)
/// @query order_status: number (订单状态，可选)
/// @query payment_status: number (支付状态，可选)
/// @query customer_id: number (客户ID，可选)
/// @query platform: string (来源平台，可选)
/// @query date_from: string (开始日期 YYYY-MM-DD，可选)
/// @query date_to: string (结束日期 YYYY-MM-DD，可选)
/// @query keyword: string (搜索关键词，可选)
/// @response 200 PagedResponse<OrderListItem>
/// @example curl -X GET "http://localhost:3000/api/v1/orders?page=1&page_size=20"
#[instrument(skip(state))]
pub async fn list_orders(
    State(state): State<AppState>,
    Query(query): Query<OrderQuery>,
) -> AppResult<Json<ApiResponse<PagedResponse<OrderListItem>>>> {
    info!("Listing orders");

    let queries = OrderQueries::new(state.db.pool());
    let result = queries
        .list(
            query.page(),
            query.page_size(),
            query.order_status,
            query.payment_status,
            query.customer_id,
            query.platform.as_deref(),
            query.date_from.as_deref(),
            query.date_to.as_deref(),
            query.keyword.as_deref(),
            query.currency.as_deref(),
        )
        .await?;

    Ok(Json(ApiResponse::success(result)))
}

/// @api GET /api/v1/orders/:id
/// @desc 获取订单详情
/// @param id: number (订单ID)
/// @response 200 OrderDetail
/// @response 404 订单不存在
/// @example curl -X GET "http://localhost:3000/api/v1/orders/1"
#[instrument(skip(state))]
pub async fn get_order(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<OrderDetail>>> {
    info!("Getting order: id={}", id);

    let queries = OrderQueries::new(state.db.pool());
    let order = queries.get_detail(id).await?.ok_or(AppError::NotFound)?;

    Ok(Json(ApiResponse::success(order)))
}

/// @api POST /api/v1/orders
/// @desc 创建订单
/// @body CreateOrderRequest
/// @response 200 Order
/// @example curl -X POST "http://localhost:3000/api/v1/orders" \
///   -H "Content-Type: application/json" \
///   -d '{"platform":"manual","items":[{"product_name":"产品A","quantity":1,"unit_price":100}],"receiver_name":"张三","receiver_phone":"13800138000","country":"CN","address":"测试地址"}'
#[instrument(skip(state))]
pub async fn create_order(
    State(state): State<AppState>,
    Json(req): Json<CreateOrderRequest>,
) -> AppResult<Json<ApiResponse<Order>>> {
    info!("Creating order: platform={}", req.platform);

    req.validate().map_err(AppError::from)?;

    let queries = OrderQueries::new(state.db.pool());
    let order = queries.create(&req).await?;

    info!("Order created: id={}, code={}", order.id, order.order_code);
    Ok(Json(ApiResponse::success(order)))
}

/// @api PUT /api/v1/orders/:id
/// @desc 更新订单（内部备注、状态等）
/// @param id: number (订单ID)
/// @body UpdateOrderRequest
/// @response 200 Order
/// @response 404 订单不存在
/// @example curl -X PUT "http://localhost:3000/api/v1/orders/1" \
///   -H "Content-Type: application/json" \
///   -d '{"internal_note":"备注信息"}'
#[instrument(skip(state))]
pub async fn update_order(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateOrderRequest>,
) -> AppResult<Json<ApiResponse<Order>>> {
    info!("Updating order: id={}", id);

    // 添加输入验证
    req.validate().map_err(AppError::from)?;

    let queries = OrderQueries::new(state.db.pool());
    let order = queries.update(id, &req).await?.ok_or(AppError::NotFound)?;

    info!("Order updated: id={}", id);
    Ok(Json(ApiResponse::success(order)))
}

/// @api POST /api/v1/orders/:id/ship
/// @desc 订单发货
/// @param id: number (订单ID)
/// @body ShipOrderRequest
/// @response 200 {"code": 200, "message": "发货成功"}
/// @response 400 订单状态不允许发货
/// @example curl -X POST "http://localhost:3000/api/v1/orders/1/ship" \
///   -H "Content-Type: application/json" \
///   -d '{"tracking_number":"SF1234567890"}'
#[instrument(skip(state))]
pub async fn ship_order(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<ShipOrderRequest>,
) -> AppResult<Json<ApiResponse<()>>> {
    info!("Shipping order: id={}, tracking={}", id, req.tracking_number);

    let queries = OrderQueries::new(state.db.pool());
    let shipped = queries.ship(id, &req).await?;

    if !shipped {
        return Err(AppError::BadRequest("Order cannot be shipped in current status".to_string()));
    }

    info!("Order shipped: id={}", id);
    Ok(Json(ApiResponse::success_message("发货成功")))
}

/// @api POST /api/v1/orders/:id/cancel
/// @desc 取消订单
/// @param id: number (订单ID)
/// @body CancelOrderRequest
/// @response 200 {"code": 200, "message": "订单已取消"}
/// @response 400 订单状态不允许取消
/// @example curl -X POST "http://localhost:3000/api/v1/orders/1/cancel" \
///   -H "Content-Type: application/json" \
///   -d '{"reason":"客户要求取消"}'
#[instrument(skip(state))]
pub async fn cancel_order(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<CancelOrderRequest>,
) -> AppResult<Json<ApiResponse<()>>> {
    info!("Cancelling order: id={}", id);

    let queries = OrderQueries::new(state.db.pool());
    let cancelled = queries.cancel(id, &req.reason).await?;

    if !cancelled {
        return Err(AppError::BadRequest("Order cannot be cancelled in current status".to_string()));
    }

    info!("Order cancelled: id={}", id);
    Ok(Json(ApiResponse::success_message("订单已取消")))
}

// ============================================================================
// 新增：状态流转、PI/CI 下载
// ============================================================================

/// 状态更新请求
#[derive(Debug, Deserialize)]
pub struct UpdateStatusRequest {
    pub status: i64,
}

/// @api POST /api/v1/orders/:id/status
/// @desc 更新订单状态
/// @param id: number (订单ID)
/// @body UpdateStatusRequest
/// @response 200 {"code": 200, "message": "状态更新成功"}
/// @example curl -X POST "http://localhost:3000/api/v1/orders/1/status" \
///   -H "Content-Type: application/json" \
///   -d '{"status": 2}'
#[instrument(skip(state))]
pub async fn update_order_status(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateStatusRequest>,
) -> AppResult<Json<ApiResponse<Order>>> {
    info!("Updating order status: id={}, new_status={}", id, req.status);

    let queries = OrderQueries::new(state.db.pool());

    // 验证状态值
    if !(1..=6).contains(&req.status) {
        return Err(AppError::BadRequest("Invalid status value".to_string()));
    }

    // 更新状态
    let update_req = UpdateOrderRequest {
        internal_note: None,
        order_status: Some(req.status),
    };

    let order = queries.update(id, &update_req).await?.ok_or(AppError::NotFound)?;

    info!("Order status updated: id={}, status={}", id, req.status);
    Ok(Json(ApiResponse::success(order)))
}

/// @api GET /api/v1/orders/:id/download-pi
/// @desc 下载 PI（形式发票）
/// @param id: number (订单ID)
/// @response 200 Excel 文件
/// @response 400 订单状态不允许下载 PI
/// @example curl -X GET "http://localhost:3000/api/v1/orders/1/download-pi" -o pi.xlsx
#[instrument(skip(state))]
pub async fn download_pi(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Response> {
    info!("Downloading PI for order: id={}", id);

    let queries = OrderQueries::new(state.db.pool());
    let order = queries.get_detail(id).await?.ok_or(AppError::NotFound)?;

    // 检查是否可以下载 PI（状态 1 或 2）
    if !crate::templates::orders::can_download_pi(order.order.order_status) {
        return Err(AppError::BadRequest("PI can only be downloaded for unconfirmed or price-locked orders".to_string()));
    }

    // 生成 Excel 格式的 PI
    let excel_data = generate_pi_excel(&order).map_err(|e| {
        AppError::InternalError(anyhow::anyhow!("Failed to generate Excel: {}", e))
    })?;

    Ok((
        [("Content-Type", "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"),
         ("Content-Disposition", &format!("attachment; filename=\"PI_{}.xlsx\"", order.order.order_code))],
        excel_data
    ).into_response())
}

/// @api GET /api/v1/orders/:id/download-ci
/// @desc 下载 CI（商业发票）
/// @param id: number (订单ID)
/// @response 200 Excel 文件
/// @response 400 订单状态不允许下载 CI
/// @example curl -X GET "http://localhost:3000/api/v1/orders/1/download-ci" -o ci.xlsx
#[instrument(skip(state))]
pub async fn download_ci(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Response> {
    info!("Downloading CI for order: id={}", id);

    let queries = OrderQueries::new(state.db.pool());
    let order = queries.get_detail(id).await?.ok_or(AppError::NotFound)?;

    // 检查是否可以下载 CI（状态 3、4 或 5）
    if !crate::templates::orders::can_download_ci(order.order.order_status) {
        return Err(AppError::BadRequest("CI can only be downloaded for paid, shipped, or delivered orders".to_string()));
    }

    // 生成 Excel 格式的 CI
    let excel_data = generate_ci_excel(&order).map_err(|e| {
        AppError::InternalError(anyhow::anyhow!("Failed to generate Excel: {}", e))
    })?;

    Ok((
        [("Content-Type", "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"),
         ("Content-Disposition", &format!("attachment; filename=\"CI_{}.xlsx\"", order.order.order_code))],
        excel_data
    ).into_response())
}

/// 生成 PI Excel 内容
fn generate_pi_excel(order: &OrderDetail) -> Result<Vec<u8>, XlsxError> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet().set_name("Proforma Invoice")?;

    // 定义格式
    let title_format = Format::new()
        .set_font_size(18)
        .set_bold()
        .set_align(rust_xlsxwriter::FormatAlign::Center);
    let header_format = Format::new()
        .set_font_size(12)
        .set_bold()
        .set_background_color("#4472C4")
        .set_font_color("#FFFFFF")
        .set_align(rust_xlsxwriter::FormatAlign::Center);
    let label_format = Format::new().set_bold();
    let money_format = Format::new().set_num_format("$#,##0.00");
    let border_format = Format::new().set_border(rust_xlsxwriter::FormatBorder::Thin);

    // 标题
    worksheet.merge_range(0, 0, 0, 5, "PROFORMA INVOICE", &title_format)?;
    worksheet.set_row_height(0, 30)?;

    // PI 信息
    worksheet.write_string_with_format(2, 0, "PI Number:", &label_format)?;
    worksheet.write_string(2, 1, &order.order.order_code)?;
    worksheet.write_string_with_format(2, 3, "Date:", &label_format)?;
    worksheet.write_string(2, 4, &order.order.created_at.format("%Y-%m-%d").to_string())?;

    // 客户信息
    worksheet.write_string_with_format(4, 0, "Bill To:", &label_format)?;
    let mut row = 5;
    if let Some(ref name) = order.order.customer_name {
        worksheet.write_string(row, 0, "Name:")?;
        worksheet.write_string(row, 1, name)?;
        row += 1;
    }
    if let Some(ref email) = order.order.customer_email {
        worksheet.write_string(row, 0, "Email:")?;
        worksheet.write_string(row, 1, email)?;
        row += 1;
    }
    if let Some(ref mobile) = order.order.customer_mobile {
        worksheet.write_string(row, 0, "Phone:")?;
        worksheet.write_string(row, 1, mobile)?;
        row += 1;
    }
    if let Some(ref addr) = order.address {
        worksheet.write_string(row, 0, "Address:")?;
        let full_addr = format!("{} {} {} {}",
            addr.country,
            addr.city.as_deref().unwrap_or(""),
            addr.province.as_deref().unwrap_or(""),
            addr.address
        );
        worksheet.write_string(row, 1, &full_addr)?;
        row += 1;
    }

    // 商品表格
    row += 2;
    worksheet.write_string_with_format(row, 0, "Description", &header_format)?;
    worksheet.write_string_with_format(row, 1, "Quantity", &header_format)?;
    worksheet.write_string_with_format(row, 2, "Unit Price", &header_format)?;
    worksheet.write_string_with_format(row, 3, "Total", &header_format)?;
    row += 1;

    let item_start_row = row;
    for item in &order.items {
        worksheet.write_string_with_format(row, 0, &item.product_name, &border_format)?;
        worksheet.write_number_with_format(row, 1, item.quantity as f64, &border_format)?;
        worksheet.write_number_with_format(row, 2, item.unit_price, &border_format)?;
        worksheet.write_number_with_format(row, 3, item.total_amount, &border_format)?;
        row += 1;
    }

    // 金额汇总
    row += 1;
    worksheet.write_string_with_format(row, 2, "Subtotal:", &label_format)?;
    worksheet.write_number_with_format(row, 3, order.order.subtotal, &money_format)?;
    row += 1;
    worksheet.write_string_with_format(row, 2, "Shipping:", &label_format)?;
    worksheet.write_number_with_format(row, 3, order.order.shipping_fee, &money_format)?;
    row += 1;
    worksheet.write_string_with_format(row, 2, "Discount:", &label_format)?;
    worksheet.write_number_with_format(row, 3, order.order.discount_amount, &money_format)?;
    row += 1;
    worksheet.write_string_with_format(row, 2, "Total:", &label_format)?;
    worksheet.write_number_with_format(row, 3, order.order.total_amount, &money_format)?;

    // 条款
    row += 2;
    worksheet.write_string_with_format(row, 0, "Terms:", &label_format)?;
    row += 1;
    worksheet.write_string(row, 0, "Payment: 100% before shipment")?;
    row += 1;
    worksheet.write_string(row, 0, "Delivery: EXW")?;
    row += 1;
    worksheet.write_string(row, 0, "Lead Time: 3-7 working days")?;

    // 设置列宽
    worksheet.set_column_width(0, 40)?;
    worksheet.set_column_width(1, 12)?;
    worksheet.set_column_width(2, 15)?;
    worksheet.set_column_width(3, 15)?;

    workbook.save_to_buffer()
}

/// 生成 CI Excel 内容
fn generate_ci_excel(order: &OrderDetail) -> Result<Vec<u8>, XlsxError> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet().set_name("Commercial Invoice")?;

    // 定义格式
    let title_format = Format::new()
        .set_font_size(18)
        .set_bold()
        .set_align(rust_xlsxwriter::FormatAlign::Center);
    let header_format = Format::new()
        .set_font_size(12)
        .set_bold()
        .set_background_color("#4472C4")
        .set_font_color("#FFFFFF")
        .set_align(rust_xlsxwriter::FormatAlign::Center);
    let label_format = Format::new().set_bold();
    let money_format = Format::new().set_num_format("$#,##0.00");
    let border_format = Format::new().set_border(rust_xlsxwriter::FormatBorder::Thin);

    // 标题
    worksheet.merge_range(0, 0, 0, 5, "COMMERCIAL INVOICE", &title_format)?;
    worksheet.set_row_height(0, 30)?;

    // CI 信息
    worksheet.write_string_with_format(2, 0, "CI Number:", &label_format)?;
    worksheet.write_string(2, 1, &order.order.order_code)?;
    worksheet.write_string_with_format(2, 3, "Date:", &label_format)?;
    worksheet.write_string(2, 4, &order.order.created_at.format("%Y-%m-%d").to_string())?;

    // 卖家信息
    worksheet.write_string_with_format(4, 0, "Seller:", &label_format)?;
    worksheet.write_string(5, 0, "Shenzhen Westway Technology Co., Ltd")?;

    // 客户信息
    worksheet.write_string_with_format(7, 0, "Buyer:", &label_format)?;
    let mut row = 8;
    if let Some(ref name) = order.order.customer_name {
        worksheet.write_string(row, 0, "Name:")?;
        worksheet.write_string(row, 1, name)?;
        row += 1;
    }
    if let Some(ref email) = order.order.customer_email {
        worksheet.write_string(row, 0, "Email:")?;
        worksheet.write_string(row, 1, email)?;
        row += 1;
    }
    if let Some(ref mobile) = order.order.customer_mobile {
        worksheet.write_string(row, 0, "Phone:")?;
        worksheet.write_string(row, 1, mobile)?;
        row += 1;
    }
    if let Some(ref addr) = order.address {
        worksheet.write_string(row, 0, "Address:")?;
        let full_addr = format!("{} {} {} {}",
            addr.country,
            addr.city.as_deref().unwrap_or(""),
            addr.province.as_deref().unwrap_or(""),
            addr.address
        );
        worksheet.write_string(row, 1, &full_addr)?;
        row += 1;
    }

    // 商品表格
    row += 2;
    worksheet.write_string_with_format(row, 0, "Description", &header_format)?;
    worksheet.write_string_with_format(row, 1, "Quantity", &header_format)?;
    worksheet.write_string_with_format(row, 2, "Unit Price", &header_format)?;
    worksheet.write_string_with_format(row, 3, "Total", &header_format)?;
    row += 1;

    for item in &order.items {
        worksheet.write_string_with_format(row, 0, &item.product_name, &border_format)?;
        worksheet.write_number_with_format(row, 1, item.quantity as f64, &border_format)?;
        worksheet.write_number_with_format(row, 2, item.unit_price, &border_format)?;
        worksheet.write_number_with_format(row, 3, item.total_amount, &border_format)?;
        row += 1;
    }

    // 金额汇总
    row += 1;
    worksheet.write_string_with_format(row, 2, "Subtotal:", &label_format)?;
    worksheet.write_number_with_format(row, 3, order.order.subtotal, &money_format)?;
    row += 1;
    worksheet.write_string_with_format(row, 2, "Shipping:", &label_format)?;
    worksheet.write_number_with_format(row, 3, order.order.shipping_fee, &money_format)?;
    row += 1;
    worksheet.write_string_with_format(row, 2, "Discount:", &label_format)?;
    worksheet.write_number_with_format(row, 3, order.order.discount_amount, &money_format)?;
    row += 1;
    worksheet.write_string_with_format(row, 2, "Total Amount:", &label_format)?;
    worksheet.write_number_with_format(row, 3, order.order.total_amount, &money_format)?;

    // 收货地址
    row += 2;
    worksheet.write_string_with_format(row, 0, "Shipping Address:", &label_format)?;
    if let Some(ref addr) = order.address {
        row += 1;
        worksheet.write_string(row, 0, &format!("{} {}", addr.receiver_name, addr.receiver_phone))?;
        row += 1;
        worksheet.write_string(row, 0, &format!("{} {} {} {}",
            addr.country,
            addr.province.as_deref().unwrap_or(""),
            addr.city.as_deref().unwrap_or(""),
            addr.address
        ))?;
    }

    // 设置列宽
    worksheet.set_column_width(0, 40)?;
    worksheet.set_column_width(1, 12)?;
    worksheet.set_column_width(2, 15)?;
    worksheet.set_column_width(3, 15)?;

    workbook.save_to_buffer()
}
