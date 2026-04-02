//! 对接 API 路由
//!
//! 用于外部系统（如 cicishop）与 ERP 数据对接
//! 路由前缀: /api/v1/integration

use axum::{
    extract::{Path, Query, State, Extension},
    routing::{get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tracing::{info, warn, instrument};
use validator::Validate;

use crate::state::AppState;
use crate::middleware::integration_auth::{IntegrationClient, require_integration_permission};
use cicierp_db::queries::{
    products::ProductQueries,
    product_prices::ProductPriceQueries,
    product_content::ProductContentQueries,
    customers::CustomerQueries,
    orders::OrderQueries,
    inventory::InventoryQueries,
};
use cicierp_models::{
    common::PagedResponse,
    product::Product,
    customer::{CreateCustomerRequest, Customer},
    order::Order,
};
use cicierp_utils::{AppError, AppResult, ApiResponse};

// ============================================================================
// 查询参数
// ============================================================================

/// 产品同步查询参数
#[derive(Debug, Deserialize)]
pub struct IntegrationProductQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub category_id: Option<i64>,
    pub brand_id: Option<i64>,
    pub status: Option<i64>,
    pub keyword: Option<String>,
    pub updated_after: Option<String>,  // ISO 8601 格式时间戳
}

impl IntegrationProductQuery {
    pub fn page(&self) -> u32 {
        self.page.unwrap_or(1).max(1)
    }

    pub fn page_size(&self) -> u32 {
        self.page_size.unwrap_or(50).min(100).max(1)
    }
}

/// 库存同步查询参数
#[derive(Debug, Deserialize)]
pub struct IntegrationInventoryQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub updated_after: Option<String>,
}

impl IntegrationInventoryQuery {
    pub fn page(&self) -> u32 {
        self.page.unwrap_or(1).max(1)
    }

    pub fn page_size(&self) -> u32 {
        self.page_size.unwrap_or(100).min(500).max(1)
    }
}

/// 批量查询请求
#[derive(Debug, Deserialize)]
pub struct BatchQueryRequest {
    pub ids: Vec<i64>,
}

/// 库存预留请求
#[derive(Debug, Deserialize, Validate)]
pub struct ReserveInventoryRequest {
    pub idempotency_key: String,
    pub items: Vec<ReserveItem>,
    pub reference_type: Option<String>,
    pub reference_id: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct ReserveItem {
    #[validate(length(min = 1))]
    pub sku_code: String,
    #[validate(range(min = 1))]
    pub quantity: i64,
}

/// 库存释放请求
#[derive(Debug, Deserialize, Validate)]
pub struct ReleaseInventoryRequest {
    pub idempotency_key: String,
    pub items: Vec<ReleaseItem>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct ReleaseItem {
    #[validate(length(min = 1))]
    pub sku_code: String,
    #[validate(range(min = 1))]
    pub quantity: i64,
}

// ============================================================================
// 响应结构
// ============================================================================

/// 产品同步响应（包含 SKU、价格、库存）
#[derive(Debug, Serialize)]
pub struct IntegrationProductItem {
    pub id: i64,
    pub product_code: String,
    pub name: String,
    pub name_en: Option<String>,
    pub main_image: Option<String>,
    pub images: Vec<String>,
    pub category: Option<CategoryInfo>,
    pub brand: Option<BrandInfo>,
    pub skus: Vec<IntegrationSkuItem>,
    pub prices: serde_json::Value,
    pub content: Option<ProductContentInfo>,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct CategoryInfo {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct BrandInfo {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct IntegrationSkuItem {
    pub id: i64,
    pub sku_code: String,
    pub spec_values: serde_json::Value,
    pub sale_price: f64,
    pub available_quantity: i64,
    pub status: i64,
}

#[derive(Debug, Serialize)]
pub struct ProductContentInfo {
    pub title_en: Option<String>,
    pub description_en: Option<String>,
    pub meta_title: Option<String>,
    pub meta_description: Option<String>,
}

/// 库存信息
#[derive(Debug, Serialize)]
pub struct IntegrationInventoryItem {
    pub sku_id: i64,
    pub sku_code: String,
    pub product_name: String,
    pub total_quantity: i64,
    pub available_quantity: i64,
    pub locked_quantity: i64,
    pub damaged_quantity: i64,
    pub safety_stock: i64,
    pub updated_at: String,
}

/// 库存预留结果
#[derive(Debug, Serialize)]
pub struct ReserveResult {
    pub success: bool,
    pub reserved_items: Vec<ReservedItemInfo>,
    pub failed_items: Vec<FailedItemInfo>,
}

#[derive(Debug, Serialize)]
pub struct ReservedItemInfo {
    pub sku_code: String,
    pub reserved_quantity: i64,
}

#[derive(Debug, Serialize)]
pub struct FailedItemInfo {
    pub sku_code: String,
    pub reason: String,
}

// ============================================================================
// 路由定义
// ============================================================================

/// 创建对接 API 路由
pub fn router() -> Router<AppState> {
    Router::new()
        // 产品同步
        .route("/products", get(list_products_for_sync))
        .route("/products/updated", get(list_updated_products))
        .route("/products/batch", post(batch_get_products))
        .route("/products/:id", get(get_product_for_sync))
        // 订单接收
        .route("/orders", post(create_order_from_platform))
        .route("/orders/:platform_order_id", get(get_order_by_platform_id))
        .route("/orders/:platform_order_id", put(update_order_from_platform))
        // 库存同步
        .route("/inventory", get(list_inventory_for_sync))
        .route("/inventory/sku/:sku_code", get(get_inventory_by_sku))
        .route("/inventory/reserve", post(reserve_inventory))
        .route("/inventory/release", post(release_inventory))
        // 客户同步
        .route("/customers", post(sync_customer))
        .route("/customers/:external_id", get(get_customer_by_external_id))
        .route("/customers/batch", post(batch_sync_customers))
}

// ============================================================================
// 产品同步 API
// ============================================================================

/// @api GET /api/v1/integration/products
/// @desc 批量获取产品（含SKU、价格、库存）
/// @auth API Key + HMAC 签名
/// @response 200 PagedResponse<IntegrationProductItem>
#[instrument(skip(state, client))]
pub async fn list_products_for_sync(
    State(state): State<AppState>,
    Extension(client): Extension<IntegrationClient>,
    Query(query): Query<IntegrationProductQuery>,
) -> AppResult<Json<ApiResponse<PagedResponse<IntegrationProductItem>>>> {
    require_integration_permission(&client, "products:read")?;

    info!("Integration: listing products for sync");

    let product_queries = ProductQueries::new(state.db.pool());
    let price_queries = ProductPriceQueries::new(state.db.pool());
    let content_queries = ProductContentQueries::new(state.db.pool());

    let result = product_queries
        .list(
            query.page(),
            query.page_size(),
            query.category_id,
            query.brand_id,
            Some(1), // 只返回上架产品
            query.keyword.as_deref(),
            None,
            None,
            None,
        )
        .await?;

    // 对于列表，需要单独获取每个产品的详情
    let mut items = Vec::with_capacity(result.items.len());
    for item in result.items {
        // 获取完整产品信息
        if let Some(product) = product_queries.get_by_id(item.id).await? {
            let int_product = build_integration_product(&product_queries, &price_queries, &content_queries, product).await;
            items.push(int_product);
        }
    }

    Ok(Json(ApiResponse::success(PagedResponse::new(
        items,
        result.pagination.page,
        result.pagination.page_size,
        result.pagination.total,
    ))))
}

/// @api GET /api/v1/integration/products/updated
/// @desc 获取增量更新产品（按时间戳）
/// @auth API Key + HMAC 签名
/// @response 200 Vec<IntegrationProductItem>
#[instrument(skip(state, client))]
pub async fn list_updated_products(
    State(state): State<AppState>,
    Extension(client): Extension<IntegrationClient>,
    Query(query): Query<IntegrationProductQuery>,
) -> AppResult<Json<ApiResponse<Vec<IntegrationProductItem>>>> {
    require_integration_permission(&client, "products:read")?;

    let updated_after = query.updated_after.as_deref().ok_or_else(|| {
        AppError::BadRequest("updated_after parameter is required".to_string())
    })?;

    info!("Integration: listing updated products since {}", updated_after);

    let product_queries = ProductQueries::new(state.db.pool());
    let price_queries = ProductPriceQueries::new(state.db.pool());
    let content_queries = ProductContentQueries::new(state.db.pool());

    let products = product_queries.list_updated_since(updated_after, query.page_size()).await?;

    let items = futures::future::join_all(products.into_iter().map(|p| async {
        build_integration_product(&product_queries, &price_queries, &content_queries, p).await
    })).await;

    Ok(Json(ApiResponse::success(items)))
}

/// @api POST /api/v1/integration/products/batch
/// @desc 批量查询产品
/// @auth API Key + HMAC 签名
/// @response 200 Vec<IntegrationProductItem>
#[instrument(skip(state, client))]
pub async fn batch_get_products(
    State(state): State<AppState>,
    Extension(client): Extension<IntegrationClient>,
    Json(req): Json<BatchQueryRequest>,
) -> AppResult<Json<ApiResponse<Vec<IntegrationProductItem>>>> {
    require_integration_permission(&client, "products:read")?;

    if req.ids.is_empty() {
        return Ok(Json(ApiResponse::success(vec![])));
    }

    if req.ids.len() > 100 {
        return Err(AppError::BadRequest("Maximum 100 IDs per batch".to_string()));
    }

    info!("Integration: batch getting {} products", req.ids.len());

    let product_queries = ProductQueries::new(state.db.pool());
    let price_queries = ProductPriceQueries::new(state.db.pool());
    let content_queries = ProductContentQueries::new(state.db.pool());

    let mut items = Vec::with_capacity(req.ids.len());
    for id in req.ids {
        if let Some(product) = product_queries.get_by_id(id).await? {
            let item = build_integration_product(&product_queries, &price_queries, &content_queries, product).await;
            items.push(item);
        }
    }

    Ok(Json(ApiResponse::success(items)))
}

/// @api GET /api/v1/integration/products/:id
/// @desc 获取单个产品详情
/// @auth API Key + HMAC 签名
/// @response 200 IntegrationProductItem
#[instrument(skip(state, client))]
pub async fn get_product_for_sync(
    State(state): State<AppState>,
    Extension(client): Extension<IntegrationClient>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<IntegrationProductItem>>> {
    require_integration_permission(&client, "products:read")?;

    info!("Integration: getting product {}", id);

    let product_queries = ProductQueries::new(state.db.pool());
    let price_queries = ProductPriceQueries::new(state.db.pool());
    let content_queries = ProductContentQueries::new(state.db.pool());

    let product = product_queries.get_by_id(id).await?.ok_or(AppError::NotFound)?;

    let item = build_integration_product(&product_queries, &price_queries, &content_queries, product).await;

    Ok(Json(ApiResponse::success(item)))
}

/// 构建集成产品响应
async fn build_integration_product(
    product_queries: &ProductQueries<'_>,
    price_queries: &ProductPriceQueries<'_>,
    content_queries: &ProductContentQueries<'_>,
    product: Product,
) -> IntegrationProductItem {
    // 获取 SKU 列表
    let skus = product_queries.get_skus(product.id).await.unwrap_or_default();

    // 获取价格
    let prices = price_queries.list_by_product(product.id).await.unwrap_or_default();

    // 构建价格映射
    let mut price_map = serde_json::Map::new();
    for price in prices {
        let platform = price.platform.clone();
        let mut platform_prices = serde_json::Map::new();
        platform_prices.insert("sale_price_cny".to_string(), serde_json::json!(price.sale_price_cny));
        if let Some(sale_usd) = price.sale_price_usd {
            platform_prices.insert("sale_price_usd".to_string(), serde_json::json!(sale_usd));
        }
        price_map.insert(platform, serde_json::Value::Object(platform_prices));
    }

    // 获取内容
    let content = content_queries.get_by_product_id(product.id).await.ok().flatten();

    // 构建 SKU 列表
    let sku_items: Vec<IntegrationSkuItem> = skus.into_iter().map(|sku| {
        IntegrationSkuItem {
            id: sku.id,
            sku_code: sku.sku_code,
            spec_values: sku.spec_values,
            sale_price: sku.sale_price,
            available_quantity: 0, // TODO: 从库存获取
            status: sku.status,
        }
    }).collect();

    // 解析图片
    let images: Vec<String> = if product.images.is_array() {
        serde_json::from_value(product.images.clone()).unwrap_or_default()
    } else {
        vec![]
    };

    IntegrationProductItem {
        id: product.id,
        product_code: product.product_code,
        name: product.name,
        name_en: product.name_en,
        main_image: product.main_image,
        images,
        category: product.category_id.map(|id| CategoryInfo { id, name: String::new() }),
        brand: product.brand_id.map(|id| BrandInfo { id, name: String::new() }),
        skus: sku_items,
        prices: serde_json::Value::Object(price_map),
        content: content.map(|c| ProductContentInfo {
            title_en: c.title_en,
            description_en: c.description_en,
            meta_title: c.meta_title,
            meta_description: c.meta_description,
        }),
        updated_at: product.updated_at.to_rfc3339(),
    }
}

// ============================================================================
// 订单接收 API
// ============================================================================

/// 来自平台的订单创建请求
#[derive(Debug, Deserialize, Validate)]
pub struct CreateOrderFromPlatformRequest {
    pub idempotency_key: String,
    pub platform: String,
    pub platform_order_id: String,
    pub customer: CustomerInfo,
    pub items: Vec<OrderItemInfo>,
    pub shipping: ShippingInfo,
    pub total_amount: f64,
    pub shipping_fee: Option<f64>,
    pub discount_amount: Option<f64>,
    pub customer_note: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CustomerInfo {
    pub external_id: Option<String>,
    pub name: String,
    pub email: Option<String>,
    pub mobile: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct OrderItemInfo {
    #[validate(length(min = 1))]
    pub sku_code: String,
    #[validate(range(min = 1))]
    pub quantity: i64,
    pub unit_price: f64,
    pub subtotal: f64,
}

#[derive(Debug, Deserialize, Validate)]
pub struct ShippingInfo {
    #[validate(length(min = 1))]
    pub receiver_name: String,
    pub receiver_phone: String,
    pub country: String,
    pub province: Option<String>,
    pub city: Option<String>,
    pub address: String,
    pub postal_code: Option<String>,
}

/// @api POST /api/v1/integration/orders
/// @desc 创建订单（来自商城）
/// @auth API Key + HMAC 签名
/// @response 200 Order
#[instrument(skip(state, client))]
pub async fn create_order_from_platform(
    State(state): State<AppState>,
    Extension(client): Extension<IntegrationClient>,
    Json(req): Json<CreateOrderFromPlatformRequest>,
) -> AppResult<Json<ApiResponse<Order>>> {
    require_integration_permission(&client, "orders:write")?;

    req.validate().map_err(AppError::from)?;

    info!("Integration: creating order from platform {} - {}", req.platform, req.platform_order_id);

    // TODO: 实现订单创建逻辑
    // 1. 检查幂等键
    // 2. 查找或创建客户
    // 3. 验证 SKU 和库存
    // 4. 创建订单

    Err(AppError::InternalError(anyhow::anyhow!("Not implemented yet")))
}

/// @api GET /api/v1/integration/orders/:platform_order_id
/// @desc 查询订单状态
/// @auth API Key + HMAC 签名
/// @response 200 Order
#[instrument(skip(state, client))]
pub async fn get_order_by_platform_id(
    State(state): State<AppState>,
    Extension(client): Extension<IntegrationClient>,
    Path(platform_order_id): Path<String>,
) -> AppResult<Json<ApiResponse<Order>>> {
    require_integration_permission(&client, "orders:read")?;

    info!("Integration: getting order by platform_id {}", platform_order_id);

    let queries = OrderQueries::new(state.db.pool());
    let order = queries.get_by_platform_order_id(&platform_order_id).await?
        .ok_or(AppError::NotFound)?;

    Ok(Json(ApiResponse::success(order)))
}

/// @api PUT /api/v1/integration/orders/:platform_order_id
/// @desc 更新订单状态
/// @auth API Key + HMAC 签名
/// @response 200 Order
#[instrument(skip(state, client))]
pub async fn update_order_from_platform(
    State(state): State<AppState>,
    Extension(client): Extension<IntegrationClient>,
    Path(platform_order_id): Path<String>,
) -> AppResult<Json<ApiResponse<Order>>> {
    require_integration_permission(&client, "orders:write")?;

    info!("Integration: updating order {}", platform_order_id);

    // TODO: 实现订单更新逻辑

    Err(AppError::InternalError(anyhow::anyhow!("Not implemented yet")))
}

// ============================================================================
// 库存同步 API
// ============================================================================

/// @api GET /api/v1/integration/inventory
/// @desc 批量获取库存
/// @auth API Key + HMAC 签名
/// @response 200 PagedResponse<IntegrationInventoryItem>
#[instrument(skip(state, client))]
pub async fn list_inventory_for_sync(
    State(state): State<AppState>,
    Extension(client): Extension<IntegrationClient>,
    Query(query): Query<IntegrationInventoryQuery>,
) -> AppResult<Json<ApiResponse<PagedResponse<IntegrationInventoryItem>>>> {
    require_integration_permission(&client, "inventory:read")?;

    info!("Integration: listing inventory");

    // TODO: 实现库存列表查询

    Ok(Json(ApiResponse::success(PagedResponse::new(
        vec![],
        query.page(),
        query.page_size(),
        0,
    ))))
}

/// @api GET /api/v1/integration/inventory/sku/:sku_code
/// @desc 按SKU查询库存
/// @auth API Key + HMAC 签名
/// @response 200 IntegrationInventoryItem
#[instrument(skip(state, client))]
pub async fn get_inventory_by_sku(
    State(state): State<AppState>,
    Extension(client): Extension<IntegrationClient>,
    Path(sku_code): Path<String>,
) -> AppResult<Json<ApiResponse<IntegrationInventoryItem>>> {
    require_integration_permission(&client, "inventory:read")?;

    info!("Integration: getting inventory for SKU {}", sku_code);

    // TODO: 实现单个 SKU 库存查询

    Err(AppError::NotFound)
}

/// @api POST /api/v1/integration/inventory/reserve
/// @desc 预留库存（商城下单前）
/// @auth API Key + HMAC 签名
/// @response 200 ReserveResult
#[instrument(skip(state, client))]
pub async fn reserve_inventory(
    State(state): State<AppState>,
    Extension(client): Extension<IntegrationClient>,
    Json(req): Json<ReserveInventoryRequest>,
) -> AppResult<Json<ApiResponse<ReserveResult>>> {
    require_integration_permission(&client, "inventory:write")?;

    req.validate().map_err(AppError::from)?;

    info!("Integration: reserving inventory, key={}", req.idempotency_key);

    // TODO: 实现库存预留逻辑
    // 1. 检查幂等键
    // 2. 验证库存充足
    // 3. 锁定库存

    Err(AppError::InternalError(anyhow::anyhow!("Not implemented yet")))
}

/// @api POST /api/v1/integration/inventory/release
/// @desc 释放预留库存
/// @auth API Key + HMAC 签名
/// @response 200 ReserveResult
#[instrument(skip(state, client))]
pub async fn release_inventory(
    State(state): State<AppState>,
    Extension(client): Extension<IntegrationClient>,
    Json(req): Json<ReleaseInventoryRequest>,
) -> AppResult<Json<ApiResponse<ReserveResult>>> {
    require_integration_permission(&client, "inventory:write")?;

    req.validate().map_err(AppError::from)?;

    info!("Integration: releasing inventory, key={}", req.idempotency_key);

    // TODO: 实现库存释放逻辑

    Err(AppError::InternalError(anyhow::anyhow!("Not implemented yet")))
}

// ============================================================================
// 客户同步 API
// ============================================================================

/// 客户同步请求
#[derive(Debug, Deserialize, Validate)]
pub struct SyncCustomerRequest {
    pub external_id: String,
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    #[validate(email)]
    pub email: Option<String>,
    #[validate(length(min = 6, max = 20))]
    pub mobile: Option<String>,
    pub source: Option<String>,
}

/// 批量客户同步请求
#[derive(Debug, Deserialize)]
pub struct BatchSyncCustomersRequest {
    pub customers: Vec<SyncCustomerRequest>,
}

/// @api POST /api/v1/integration/customers
/// @desc 创建/更新客户
/// @auth API Key + HMAC 签名
/// @response 200 Customer
#[instrument(skip(state, client))]
pub async fn sync_customer(
    State(state): State<AppState>,
    Extension(client): Extension<IntegrationClient>,
    Json(req): Json<SyncCustomerRequest>,
) -> AppResult<Json<ApiResponse<Customer>>> {
    require_integration_permission(&client, "customers:write")?;

    req.validate().map_err(AppError::from)?;

    info!("Integration: syncing customer {}", req.external_id);

    let queries = CustomerQueries::new(state.db.pool());

    // 尝试按 external_id 查找客户
    // TODO: 需要在 CustomerQueries 中添加 get_by_external_id 方法

    // 创建新客户
    let create_req = CreateCustomerRequest {
        name: req.name,
        mobile: req.mobile.unwrap_or_default(),
        email: req.email,
        status: Some(1),
        lead_status: None,
        notes: None,
        next_followup_date: None,
        followup_notes: None,
        source: Some(req.source.unwrap_or_else(|| "integration".to_string())),
    };

    let customer = queries.create(&create_req).await?;

    Ok(Json(ApiResponse::success(customer)))
}

/// @api GET /api/v1/integration/customers/:external_id
/// @desc 查询客户
/// @auth API Key + HMAC 签名
/// @response 200 Customer
#[instrument(skip(state, client))]
pub async fn get_customer_by_external_id(
    State(state): State<AppState>,
    Extension(client): Extension<IntegrationClient>,
    Path(external_id): Path<String>,
) -> AppResult<Json<ApiResponse<Customer>>> {
    require_integration_permission(&client, "customers:read")?;

    info!("Integration: getting customer {}", external_id);

    // TODO: 实现按 external_id 查询

    Err(AppError::NotFound)
}

/// @api POST /api/v1/integration/customers/batch
/// @desc 批量同步客户
/// @auth API Key + HMAC 签名
/// @response 200 Vec<Customer>
#[instrument(skip(state, client))]
pub async fn batch_sync_customers(
    State(state): State<AppState>,
    Extension(client): Extension<IntegrationClient>,
    Json(req): Json<BatchSyncCustomersRequest>,
) -> AppResult<Json<ApiResponse<Vec<Customer>>>> {
    require_integration_permission(&client, "customers:write")?;

    if req.customers.len() > 100 {
        return Err(AppError::BadRequest("Maximum 100 customers per batch".to_string()));
    }

    info!("Integration: batch syncing {} customers", req.customers.len());

    // TODO: 实现批量同步逻辑

    Ok(Json(ApiResponse::success(vec![])))
}
