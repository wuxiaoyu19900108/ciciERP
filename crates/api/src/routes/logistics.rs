//! 物流模块 API 路由

use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Extension, Json, Router,
};
use tracing::{info, warn};
use validator::Validate;

use crate::middleware::auth::AuthUser;
use crate::state::AppState;
use cicierp_db::queries::{
    logistics::{LogisticsCompanyQueries, ShipmentQueries},
    orders::OrderQueries,
};
use cicierp_models::common::PagedResponse;
use cicierp_models::logistics::{
    AddTrackingRequest, CreateLogisticsCompanyRequest, CreateShipmentRequest,
    LogisticsCompany, ShipmentDetail, ShipmentListItem, ShipmentQuery,
    UpdateLogisticsCompanyRequest, UpdateShipmentRequest,
};
use cicierp_utils::{AppError, AppResult, ApiResponse};

/// 创建物流模块路由
pub fn router() -> Router<AppState> {
    Router::new()
        // 物流公司
        .route("/logistics/companies", get(list_logistics_companies))
        .route("/logistics/companies", post(create_logistics_company))
        .route("/logistics/companies/:id", put(update_logistics_company))
        .route("/logistics/companies/:id", delete(delete_logistics_company))
        // 发货单
        .route("/shipments", get(list_shipments))
        .route("/shipments", post(create_shipment))
        .route("/shipments/:id", get(get_shipment))
        .route("/shipments/:id", put(update_shipment))
        .route("/shipments/:id/tracking", get(get_shipment_tracking))
        .route("/shipments/:id/tracking", post(add_shipment_tracking))
}

// ============================================================================
// 物流公司 API
// ============================================================================

/// @api GET /api/v1/logistics/companies
/// @desc 获取物流公司列表
/// @response 200 Vec<LogisticsCompany>
/// @example curl -X GET "http://localhost:3000/api/v1/logistics/companies"
pub async fn list_logistics_companies(
    State(state): State<AppState>,
    _auth_user: Extension<AuthUser>,
) -> AppResult<Json<ApiResponse<Vec<LogisticsCompany>>>> {
    info!("List logistics companies");

    let queries = LogisticsCompanyQueries::new(state.db.pool());
    let companies = queries.list().await?;

    Ok(Json(ApiResponse::success(companies)))
}

/// @api POST /api/v1/logistics/companies
/// @desc 创建物流公司
/// @body CreateLogisticsCompanyRequest
/// @response 200 LogisticsCompany
/// @response 400 参数错误
/// @response 409 编码已存在
/// @example curl -X POST "http://localhost:3000/api/v1/logistics/companies" \
///   -H "Content-Type: application/json" \
///   -d '{"code":"SF","name":"顺丰速运","service_type":"express"}'
pub async fn create_logistics_company(
    State(state): State<AppState>,
    _auth_user: Extension<AuthUser>,
    Json(req): Json<CreateLogisticsCompanyRequest>,
) -> AppResult<Json<ApiResponse<LogisticsCompany>>> {
    info!("Create logistics company: code={}", req.code);

    // 验证请求
    req.validate().map_err(|e| {
        warn!("Create logistics company validation failed: {}", e);
        AppError::BadRequest(e.to_string())
    })?;

    let queries = LogisticsCompanyQueries::new(state.db.pool());

    // 检查编码是否已存在
    if queries.get_by_code(&req.code).await?.is_some() {
        return Err(AppError::Conflict("物流公司编码已存在".to_string()));
    }

    let company = queries.create(&req).await?;
    info!("Logistics company created: id={}", company.id);

    Ok(Json(ApiResponse::success(company)))
}

/// @api PUT /api/v1/logistics/companies/:id
/// @desc 更新物流公司
/// @param id: number (物流公司ID)
/// @body UpdateLogisticsCompanyRequest
/// @response 200 LogisticsCompany
/// @response 404 物流公司不存在
/// @example curl -X PUT "http://localhost:3000/api/v1/logistics/companies/1" \
///   -H "Content-Type: application/json" \
///   -d '{"name":"新名称"}'
pub async fn update_logistics_company(
    State(state): State<AppState>,
    _auth_user: Extension<AuthUser>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateLogisticsCompanyRequest>,
) -> AppResult<Json<ApiResponse<LogisticsCompany>>> {
    info!("Update logistics company: id={}", id);

    let queries = LogisticsCompanyQueries::new(state.db.pool());
    let company = queries.update(id, &req).await?.ok_or(AppError::NotFound)?;

    info!("Logistics company updated: id={}", id);
    Ok(Json(ApiResponse::success(company)))
}

/// @api DELETE /api/v1/logistics/companies/:id
/// @desc 删除物流公司（设置状态为禁用）
/// @param id: number (物流公司ID)
/// @response 200 { "code": 200, "message": "删除成功" }
/// @response 404 物流公司不存在
/// @example curl -X DELETE "http://localhost:3000/api/v1/logistics/companies/1"
pub async fn delete_logistics_company(
    State(state): State<AppState>,
    _auth_user: Extension<AuthUser>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<()>>> {
    info!("Delete logistics company: id={}", id);

    let queries = LogisticsCompanyQueries::new(state.db.pool());
    let deleted = queries.delete(id).await?;

    if !deleted {
        return Err(AppError::NotFound);
    }

    info!("Logistics company deleted: id={}", id);
    Ok(Json(ApiResponse::success_message("删除成功")))
}

// ============================================================================
// 发货单 API
// ============================================================================

/// @api GET /api/v1/shipments
/// @desc 获取发货单列表
/// @query page, page_size, order_id, logistics_id, status, tracking_number
/// @response 200 PagedResponse<ShipmentListItem>
/// @example curl -X GET "http://localhost:3000/api/v1/shipments?page=1&page_size=20"
pub async fn list_shipments(
    State(state): State<AppState>,
    _auth_user: Extension<AuthUser>,
    Query(query): Query<ShipmentQuery>,
) -> AppResult<Json<ApiResponse<PagedResponse<ShipmentListItem>>>> {
    info!(
        "List shipments: page={}, page_size={}",
        query.page(),
        query.page_size()
    );

    let queries = ShipmentQueries::new(state.db.pool());
    let (items, total) = queries.list(query.page(), query.page_size(), &query).await?;

    let response = PagedResponse::new(items, query.page(), query.page_size(), total);
    Ok(Json(ApiResponse::success(response)))
}

/// @api GET /api/v1/shipments/:id
/// @desc 获取发货单详情（含物流轨迹）
/// @param id: number (发货单ID)
/// @response 200 ShipmentDetail
/// @response 404 发货单不存在
/// @example curl -X GET "http://localhost:3000/api/v1/shipments/1"
pub async fn get_shipment(
    State(state): State<AppState>,
    _auth_user: Extension<AuthUser>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<ShipmentDetail>>> {
    info!("Get shipment: id={}", id);

    let queries = ShipmentQueries::new(state.db.pool());
    let detail = queries.get_detail(id).await?.ok_or(AppError::NotFound)?;

    Ok(Json(ApiResponse::success(detail)))
}

/// @api POST /api/v1/shipments
/// @desc 创建发货单（订单发货）
/// @body CreateShipmentRequest
/// @response 200 Shipment
/// @response 400 参数错误或订单状态不正确
/// @response 404 订单不存在
/// @example curl -X POST "http://localhost:3000/api/v1/shipments" \
///   -H "Content-Type: application/json" \
///   -d '{"order_id":1,"logistics_name":"顺丰速运","tracking_number":"SF1234567890","package_items":[{"product_name":"产品A","quantity":2}]}'
pub async fn create_shipment(
    State(state): State<AppState>,
    _auth_user: Extension<AuthUser>,
    Json(req): Json<CreateShipmentRequest>,
) -> AppResult<Json<ApiResponse<cicierp_models::logistics::Shipment>>> {
    info!("Create shipment: order_id={}", req.order_id);

    // 验证请求
    req.validate().map_err(|e| {
        warn!("Create shipment validation failed: {}", e);
        AppError::BadRequest(e.to_string())
    })?;

    // 获取订单信息和收货地址
    let order_queries = OrderQueries::new(state.db.pool());
    let order = order_queries
        .get_by_id(req.order_id)
        .await?
        .ok_or_else(|| AppError::BadRequest("订单不存在".to_string()))?;

    // 检查订单状态
    if order.order_status != 2 && order.order_status != 3 {
        return Err(AppError::BadRequest(
            "订单状态不正确，无法发货".to_string(),
        ));
    }

    // 获取收货地址
    let address = order_queries.get_address(req.order_id).await?;
    let receiver_info = match address {
        Some(addr) => (addr.receiver_name, addr.receiver_phone, format!("{} {} {}", addr.province.unwrap_or_default(), addr.city.unwrap_or_default(), addr.address)),
        None => (
            order.customer_name.unwrap_or_default(),
            order.customer_mobile.unwrap_or_default(),
            String::new(),
        ),
    };

    let queries = ShipmentQueries::new(state.db.pool());
    let shipment = queries.create(&req, &receiver_info).await?;

    info!(
        "Shipment created: id={}, code={}",
        shipment.id, shipment.shipment_code
    );
    Ok(Json(ApiResponse::success(shipment)))
}

/// @api PUT /api/v1/shipments/:id
/// @desc 更新发货单
/// @param id: number (发货单ID)
/// @body UpdateShipmentRequest
/// @response 200 Shipment
/// @response 404 发货单不存在
/// @example curl -X PUT "http://localhost:3000/api/v1/shipments/1" \
///   -H "Content-Type: application/json" \
///   -d '{"tracking_number":"SF1234567891"}'
pub async fn update_shipment(
    State(state): State<AppState>,
    _auth_user: Extension<AuthUser>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateShipmentRequest>,
) -> AppResult<Json<ApiResponse<cicierp_models::logistics::Shipment>>> {
    info!("Update shipment: id={}", id);

    let queries = ShipmentQueries::new(state.db.pool());
    let shipment = queries.update(id, &req).await?.ok_or(AppError::NotFound)?;

    info!("Shipment updated: id={}", id);
    Ok(Json(ApiResponse::success(shipment)))
}

/// @api GET /api/v1/shipments/:id/tracking
/// @desc 获取物流轨迹
/// @param id: number (发货单ID)
/// @response 200 Vec<ShipmentTracking>
/// @response 404 发货单不存在
/// @example curl -X GET "http://localhost:3000/api/v1/shipments/1/tracking"
pub async fn get_shipment_tracking(
    State(state): State<AppState>,
    _auth_user: Extension<AuthUser>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Vec<cicierp_models::logistics::ShipmentTracking>>>> {
    info!("Get shipment tracking: id={}", id);

    let queries = ShipmentQueries::new(state.db.pool());

    // 检查发货单是否存在
    if queries.get_by_id(id).await?.is_none() {
        return Err(AppError::NotFound);
    }

    let tracking = queries.get_tracking(id).await?;
    Ok(Json(ApiResponse::success(tracking)))
}

/// @api POST /api/v1/shipments/:id/tracking
/// @desc 添加物流轨迹
/// @param id: number (发货单ID)
/// @body AddTrackingRequest
/// @response 200 ShipmentTracking
/// @response 404 发货单不存在
/// @example curl -X POST "http://localhost:3000/api/v1/shipments/1/tracking" \
///   -H "Content-Type: application/json" \
///   -d '{"tracking_time":"2026-02-27 10:00:00","tracking_status":"已签收","tracking_description":"本人签收"}'
pub async fn add_shipment_tracking(
    State(state): State<AppState>,
    _auth_user: Extension<AuthUser>,
    Path(id): Path<i64>,
    Json(req): Json<AddTrackingRequest>,
) -> AppResult<Json<ApiResponse<cicierp_models::logistics::ShipmentTracking>>> {
    info!("Add shipment tracking: shipment_id={}", id);

    // 验证请求
    req.validate().map_err(|e| {
        warn!("Add tracking validation failed: {}", e);
        AppError::BadRequest(e.to_string())
    })?;

    let queries = ShipmentQueries::new(state.db.pool());

    // 检查发货单是否存在
    if queries.get_by_id(id).await?.is_none() {
        return Err(AppError::NotFound);
    }

    let tracking = queries.add_tracking(id, &req).await?;

    info!("Tracking added: shipment_id={}", id);
    Ok(Json(ApiResponse::success(tracking)))
}
