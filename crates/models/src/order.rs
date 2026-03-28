//! 订单相关数据模型

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use validator::Validate;

/// 订单类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderType {
    #[serde(rename = "1")]
    Normal = 1,      // 普通订单
    #[serde(rename = "2")]
    Preorder = 2,    // 预售
    #[serde(rename = "3")]
    Exchange = 3,    // 换货
}

impl Default for OrderType {
    fn default() -> Self {
        OrderType::Normal
    }
}

/// 订单状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderStatus {
    #[serde(rename = "1")]
    Pending = 1,       // 待审核
    #[serde(rename = "2")]
    Confirmed = 2,     // 待发货
    #[serde(rename = "3")]
    PartialShipped = 3, // 部分发货
    #[serde(rename = "4")]
    Shipped = 4,       // 已发货
    #[serde(rename = "5")]
    Completed = 5,     // 已完成
    #[serde(rename = "6")]
    Cancelled = 6,     // 已取消
    #[serde(rename = "7")]
    AfterSale = 7,     // 售后中
}

impl Default for OrderStatus {
    fn default() -> Self {
        OrderStatus::Pending
    }
}

/// 支付状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaymentStatus {
    #[serde(rename = "1")]
    Unpaid = 1,        // 未支付
    #[serde(rename = "2")]
    PartialPaid = 2,   // 部分支付
    #[serde(rename = "3")]
    Paid = 3,          // 已支付
    #[serde(rename = "4")]
    Refunded = 4,      // 已退款
    #[serde(rename = "5")]
    PartialRefund = 5, // 部分退款
}

impl Default for PaymentStatus {
    fn default() -> Self {
        PaymentStatus::Unpaid
    }
}

/// 履约状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FulfillmentStatus {
    #[serde(rename = "1")]
    Unfulfilled = 1,      // 未发货
    #[serde(rename = "2")]
    PartialFulfilled = 2, // 部分发货
    #[serde(rename = "3")]
    Fulfilled = 3,        // 已发货
    #[serde(rename = "4")]
    Delivered = 4,        // 已签收
}

impl Default for FulfillmentStatus {
    fn default() -> Self {
        FulfillmentStatus::Unfulfilled
    }
}

/// 订单实体
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Order {
    pub id: i64,
    pub order_code: String,
    pub platform: String,
    pub platform_order_id: Option<String>,
    pub customer_id: Option<i64>,
    pub customer_name: Option<String>,
    pub customer_mobile: Option<String>,
    pub customer_email: Option<String>,
    pub order_type: i64,
    pub order_status: i64,
    pub payment_status: i64,
    pub fulfillment_status: i64,
    pub total_amount: f64,
    pub subtotal: f64,
    pub discount_amount: f64,
    pub shipping_fee: f64,
    pub tax_amount: f64,
    pub paid_amount: f64,
    pub refund_amount: f64,
    pub currency: String,
    pub exchange_rate: Option<f64>,
    pub coupon_id: Option<i64>,
    pub coupon_amount: f64,
    pub points_used: i64,
    pub points_discount: f64,
    pub customer_note: Option<String>,
    pub internal_note: Option<String>,
    pub is_rated: bool,
    pub payment_time: Option<DateTime<Utc>>,
    pub ship_time: Option<DateTime<Utc>>,
    pub finish_time: Option<DateTime<Utc>>,
    pub cancel_time: Option<DateTime<Utc>>,
    pub cancel_reason: Option<String>,
    // 条款信息
    pub payment_terms: Option<String>,
    pub delivery_terms: Option<String>,
    pub lead_time: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 订单明细
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct OrderItem {
    pub id: i64,
    pub order_id: i64,
    pub product_id: Option<i64>,
    pub sku_id: Option<i64>,
    pub product_name: String,
    pub product_code: Option<String>,
    pub sku_code: Option<String>,
    pub sku_spec: Option<JsonValue>,
    pub product_image: Option<String>,
    pub quantity: i64,
    pub unit_price: f64,
    pub subtotal: f64,
    pub discount_amount: f64,
    pub total_amount: f64,
    pub cost_price: Option<f64>,
    pub tax_rate: Option<f64>,
    pub tax_amount: Option<f64>,
    pub refund_quantity: i64,
    pub refund_amount: f64,
    pub created_at: DateTime<Utc>,
}

/// 订单收货地址
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct OrderAddress {
    pub id: i64,
    pub order_id: i64,
    pub receiver_name: String,
    pub receiver_phone: String,
    pub country: String,
    pub country_code: Option<String>,
    pub province: Option<String>,
    pub city: Option<String>,
    pub district: Option<String>,
    pub address: String,
    pub postal_code: Option<String>,
    pub address_type: i64,
    pub created_at: String,
}

// ============================================================================
// 请求/响应 DTOs
// ============================================================================

/// 创建订单明细请求
#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct OrderItemRequest {
    pub product_id: Option<i64>,
    pub sku_id: Option<i64>,
    #[validate(length(min = 1))]
    pub product_name: String,
    pub product_code: Option<String>,
    pub sku_code: Option<String>,
    pub sku_spec: Option<JsonValue>,
    pub product_image: Option<String>,
    #[validate(range(min = 1))]
    pub quantity: i64,
    #[validate(range(min = 0.0))]
    pub unit_price: f64,
}

/// 创建订单请求
#[derive(Debug, Deserialize, Validate)]
pub struct CreateOrderRequest {
    pub platform: String,
    pub platform_order_id: Option<String>,
    pub customer_id: Option<i64>,
    pub customer_name: Option<String>,
    pub customer_mobile: Option<String>,
    pub customer_email: Option<String>,
    pub order_type: Option<i64>,
    #[validate(length(min = 1))]
    pub items: Vec<OrderItemRequest>,
    pub shipping_fee: Option<f64>,
    pub discount_amount: Option<f64>,
    pub customer_note: Option<String>,
    // 收货地址
    pub receiver_name: String,
    pub receiver_phone: String,
    pub country: String,
    pub province: Option<String>,
    pub city: Option<String>,
    pub district: Option<String>,
    pub address: String,
    pub postal_code: Option<String>,
    // 条款信息
    pub payment_terms: Option<String>,
    pub delivery_terms: Option<String>,
    pub lead_time: Option<String>,
}

/// 更新订单请求
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateOrderRequest {
    pub internal_note: Option<String>,
    pub order_status: Option<i64>,
}

/// 订单发货请求
#[derive(Debug, Deserialize, Validate)]
pub struct ShipOrderRequest {
    pub logistics_id: Option<i64>,
    pub logistics_name: Option<String>,
    #[validate(length(min = 1))]
    pub tracking_number: String,
    pub shipping_note: Option<String>,
}

/// 订单取消请求
#[derive(Debug, Deserialize, Validate)]
pub struct CancelOrderRequest {
    #[validate(length(min = 1))]
    pub reason: String,
}

/// 订单查询参数
#[derive(Debug, Deserialize)]
pub struct OrderQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub order_status: Option<i64>,
    pub payment_status: Option<i64>,
    pub customer_id: Option<i64>,
    pub platform: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub keyword: Option<String>,
}

impl OrderQuery {
    pub fn page(&self) -> u32 {
        self.page.unwrap_or(1).max(1)
    }

    pub fn page_size(&self) -> u32 {
        self.page_size.unwrap_or(20).min(100).max(1)
    }
}

/// 订单详情（包含明细和地址）
#[derive(Debug, Serialize)]
pub struct OrderDetail {
    #[serde(flatten)]
    pub order: Order,
    pub items: Vec<OrderItem>,
    pub address: Option<OrderAddress>,
}

/// 订单列表项（精简版）
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct OrderListItem {
    pub id: i64,
    pub order_code: String,
    pub customer_name: Option<String>,
    pub total_amount: f64,
    pub order_status: i64,
    pub payment_status: i64,
    pub fulfillment_status: i64,
    pub item_count: i64,
    pub created_at: DateTime<Utc>,
}
