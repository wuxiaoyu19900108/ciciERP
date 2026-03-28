//! 采购模块数据模型

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

/// 采购单付款状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PurchasePaymentStatus {
    #[serde(rename = "1")]
    Unpaid = 1,      // 未付款
    #[serde(rename = "2")]
    PartialPaid = 2, // 部分付款
    #[serde(rename = "3")]
    Paid = 3,        // 已付款
}

impl Default for PurchasePaymentStatus {
    fn default() -> Self {
        PurchasePaymentStatus::Unpaid
    }
}

/// 采购单交货状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeliveryStatus {
    #[serde(rename = "1")]
    Unreceived = 1,      // 未收货
    #[serde(rename = "2")]
    PartialReceived = 2, // 部分收货
    #[serde(rename = "3")]
    Received = 3,        // 已收货
}

impl Default for DeliveryStatus {
    fn default() -> Self {
        DeliveryStatus::Unreceived
    }
}

/// 采购单状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PurchaseOrderStatus {
    #[serde(rename = "1")]
    Pending = 1,    // 待审核
    #[serde(rename = "2")]
    Approved = 2,   // 已审核
    #[serde(rename = "3")]
    Processing = 3, // 执行中
    #[serde(rename = "4")]
    Completed = 4,  // 已完成
    #[serde(rename = "5")]
    Cancelled = 5,  // 已取消
}

impl Default for PurchaseOrderStatus {
    fn default() -> Self {
        PurchaseOrderStatus::Pending
    }
}

/// 采购单实体
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PurchaseOrder {
    pub id: i64,
    pub order_code: String,
    pub supplier_id: i64,
    pub supplier_name: Option<String>,
    pub total_amount: f64,
    pub tax_amount: f64,
    pub paid_amount: f64,
    pub payment_status: i64,
    pub delivery_status: i64,
    pub expected_date: Option<String>,
    pub actual_date: Option<String>,
    pub status: i64,
    pub approved_by: Option<i64>,
    pub approved_at: Option<String>,
    pub approval_note: Option<String>,
    pub supplier_note: Option<String>,
    pub internal_note: Option<String>,
    pub attachments: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 采购单明细实体
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PurchaseOrderItem {
    pub id: i64,
    pub order_id: i64,
    pub product_id: Option<i64>,
    pub sku_id: Option<i64>,
    pub product_name: String,
    pub sku_code: Option<String>,
    pub spec_values: Option<String>,
    pub quantity: i64,
    pub received_qty: i64,
    pub unit_price: f64,
    pub subtotal: f64,
    pub expected_qty: Option<i64>,
    pub expected_date: Option<String>,
    pub inspected_qty: i64,
    pub qualified_qty: i64,
    pub defective_qty: i64,
    pub batch_code: Option<String>,
    pub production_date: Option<String>,
    pub expiry_date: Option<String>,
    pub supplier_id: Option<i64>,
    pub supplier_name: Option<String>,
    pub created_at: String,
}

// ============================================================================
// 请求/响应 DTOs
// ============================================================================

/// 创建采购单明细请求
#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct PurchaseItemRequest {
    pub product_id: Option<i64>,
    pub sku_id: Option<i64>,
    #[validate(length(min = 1, message = "产品名称不能为空"))]
    pub product_name: String,
    pub sku_code: Option<String>,
    pub spec_values: Option<String>,
    #[validate(range(min = 1, message = "采购数量必须大于0"))]
    pub quantity: i64,
    #[validate(range(min = 0.0, message = "单价不能为负"))]
    pub unit_price: f64,
    pub expected_date: Option<String>,
    pub batch_code: Option<String>,
    pub production_date: Option<String>,
    pub expiry_date: Option<String>,
    /// 供应商ID（一单多供应商模式）
    #[validate(range(min = 1, message = "供应商ID无效"))]
    pub supplier_id: i64,
}

/// 创建采购单请求
#[derive(Debug, Deserialize, Validate)]
pub struct CreatePurchaseOrderRequest {
    #[validate(length(min = 1, message = "采购明细不能为空"))]
    pub items: Vec<PurchaseItemRequest>,
    pub expected_date: Option<String>,
    pub supplier_note: Option<String>,
    pub internal_note: Option<String>,
    pub tax_amount: Option<f64>,
}

/// 更新采购单请求
#[derive(Debug, Deserialize, Validate)]
pub struct UpdatePurchaseOrderRequest {
    pub expected_date: Option<String>,
    pub supplier_note: Option<String>,
    pub internal_note: Option<String>,
    pub items: Option<Vec<PurchaseItemRequest>>,
}

/// 审批采购单请求
#[derive(Debug, Deserialize, Validate)]
pub struct ApprovePurchaseRequest {
    pub approval_note: Option<String>,
}

/// 采购入库请求
#[derive(Debug, Deserialize, Validate)]
pub struct ReceivePurchaseRequest {
    pub sku_id: i64,
    #[validate(range(min = 1, message = "收货数量必须大于0"))]
    pub received_qty: i64,
    #[validate(range(min = 0, message = "质检合格数量不能为负"))]
    pub qualified_qty: Option<i64>,
    #[validate(range(min = 0, message = "质检不合格数量不能为负"))]
    pub defective_qty: Option<i64>,
    pub batch_code: Option<String>,
    pub note: Option<String>,
}

/// 采购单查询参数
#[derive(Debug, Deserialize)]
pub struct PurchaseQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub supplier_id: Option<i64>,
    pub status: Option<i64>,
    pub payment_status: Option<i64>,
    pub delivery_status: Option<i64>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub keyword: Option<String>,
}

impl PurchaseQuery {
    pub fn page(&self) -> u32 {
        self.page.unwrap_or(1).max(1)
    }

    pub fn page_size(&self) -> u32 {
        self.page_size.unwrap_or(20).min(100).max(1)
    }
}

/// 采购单详情（包含明细）
#[derive(Debug, Serialize)]
pub struct PurchaseOrderDetail {
    #[serde(flatten)]
    pub order: PurchaseOrder,
    pub items: Vec<PurchaseOrderItem>,
}

/// 采购单列表项
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PurchaseOrderListItem {
    pub id: i64,
    pub order_code: String,
    pub supplier_id: i64,
    pub supplier_name: Option<String>,
    pub total_amount: f64,
    pub payment_status: i64,
    pub delivery_status: i64,
    pub status: i64,
    pub item_count: i64,
    pub expected_date: Option<String>,
    pub created_at: String,
}

/// 获取采购单状态文本
pub fn purchase_status_text(status: i64) -> &'static str {
    match status {
        1 => "草稿",
        2 => "待审核",
        3 => "已审核",
        4 => "部分入库",
        5 => "已完成",
        6 => "已取消",
        _ => "未知",
    }
}

/// 获取付款状态文本
pub fn payment_status_text(status: i64) -> &'static str {
    match status {
        1 => "未付款",
        2 => "部分付款",
        3 => "已付款",
        _ => "未知",
    }
}

/// 获取交货状态文本
pub fn delivery_status_text(status: i64) -> &'static str {
    match status {
        1 => "未收货",
        2 => "部分收货",
        3 => "已收货",
        _ => "未知",
    }
}
