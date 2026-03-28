//! PI (Proforma Invoice) 形式发票相关数据模型

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

/// PI 状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PIStatus {
    #[serde(rename = "1")]
    Draft = 1,         // 草稿
    #[serde(rename = "2")]
    Sent = 2,          // 已发送
    #[serde(rename = "3")]
    Confirmed = 3,     // 已确认
    #[serde(rename = "4")]
    Converted = 4,     // 已转订单
    #[serde(rename = "5")]
    Cancelled = 5,     // 已取消
}

impl Default for PIStatus {
    fn default() -> Self {
        PIStatus::Draft
    }
}

impl From<i64> for PIStatus {
    fn from(value: i64) -> Self {
        match value {
            1 => PIStatus::Draft,
            2 => PIStatus::Sent,
            3 => PIStatus::Confirmed,
            4 => PIStatus::Converted,
            5 => PIStatus::Cancelled,
            _ => PIStatus::Draft,
        }
    }
}

/// PI 实体
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProformaInvoice {
    pub id: i64,
    pub pi_code: String,
    pub customer_id: Option<i64>,
    pub customer_name: String,
    pub customer_email: Option<String>,
    pub customer_phone: Option<String>,
    pub customer_address: Option<String>,

    // 卖家信息
    pub seller_name: String,
    pub seller_address: Option<String>,
    pub seller_phone: Option<String>,
    pub seller_email: Option<String>,

    // 金额
    pub currency: String,
    pub subtotal: f64,
    pub discount: f64,
    pub total_amount: f64,

    // 状态
    pub status: i64,

    // 日期
    pub pi_date: String,
    pub valid_until: Option<String>,
    pub confirmed_at: Option<String>,
    pub converted_at: Option<String>,

    // 条款
    pub payment_terms: String,
    pub delivery_terms: String,
    pub lead_time: String,
    pub notes: Option<String>,

    // 关联
    pub sales_order_id: Option<i64>,

    pub created_at: String,
    pub updated_at: String,
}

/// PI 明细
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProformaInvoiceItem {
    pub id: i64,
    pub pi_id: i64,
    pub product_id: Option<i64>,
    pub product_name: String,
    pub model: Option<String>,
    pub quantity: i64,
    pub unit_price: f64,
    pub total_price: f64,
    pub notes: Option<String>,
    pub sort_order: i64,
    pub created_at: String,
}

// ============================================================================
// 请求/响应 DTOs
// ============================================================================

/// 创建 PI 明细请求
#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct PIItemRequest {
    pub product_id: Option<i64>,
    #[validate(length(min = 1))]
    pub product_name: String,
    pub model: Option<String>,
    #[validate(range(min = 1))]
    pub quantity: i64,
    #[validate(range(min = 0.0))]
    pub unit_price: f64,
    pub notes: Option<String>,
    pub sort_order: Option<i64>,
}

/// 创建 PI 请求
#[derive(Debug, Deserialize, Validate)]
pub struct CreatePIRequest {
    pub customer_id: Option<i64>,
    #[validate(length(min = 1))]
    pub customer_name: String,
    pub customer_email: Option<String>,
    pub customer_phone: Option<String>,
    pub customer_address: Option<String>,

    // 卖家信息（可选，有默认值）
    pub seller_name: Option<String>,
    pub seller_address: Option<String>,
    pub seller_phone: Option<String>,
    pub seller_email: Option<String>,

    // 金额
    pub currency: Option<String>,
    pub discount: Option<f64>,

    // 日期
    #[validate(length(min = 1))]
    pub pi_date: String,
    pub valid_until: Option<String>,

    // 条款
    pub payment_terms: Option<String>,
    pub delivery_terms: Option<String>,
    pub lead_time: Option<String>,
    pub notes: Option<String>,

    // 明细
    #[validate(length(min = 1))]
    pub items: Vec<PIItemRequest>,
}

/// 更新 PI 请求
#[derive(Debug, Deserialize, Validate)]
pub struct UpdatePIRequest {
    pub customer_id: Option<i64>,
    pub customer_name: Option<String>,
    pub customer_email: Option<String>,
    pub customer_phone: Option<String>,
    pub customer_address: Option<String>,

    pub seller_name: Option<String>,
    pub seller_address: Option<String>,
    pub seller_phone: Option<String>,
    pub seller_email: Option<String>,

    pub currency: Option<String>,
    pub discount: Option<f64>,

    pub pi_date: Option<String>,
    pub valid_until: Option<String>,

    pub payment_terms: Option<String>,
    pub delivery_terms: Option<String>,
    pub lead_time: Option<String>,
    pub notes: Option<String>,

    pub items: Option<Vec<PIItemRequest>>,
}

/// PI 查询参数
#[derive(Debug, Deserialize)]
pub struct PIQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub status: Option<i64>,
    pub customer_id: Option<i64>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub keyword: Option<String>,
}

impl PIQuery {
    pub fn page(&self) -> u32 {
        self.page.unwrap_or(1).max(1)
    }

    pub fn page_size(&self) -> u32 {
        self.page_size.unwrap_or(20).min(100).max(1)
    }
}

/// PI 详情（包含明细）
#[derive(Debug, Serialize)]
pub struct PIDetail {
    #[serde(flatten)]
    pub pi: ProformaInvoice,
    pub items: Vec<ProformaInvoiceItem>,
}

/// PI 列表项（精简版）
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PIListItem {
    pub id: i64,
    pub pi_code: String,
    pub customer_name: String,
    pub total_amount: f64,
    pub currency: String,
    pub status: i64,
    pub pi_date: String,
    pub item_count: i64,
    pub created_at: String,
}
