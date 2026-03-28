//! CI (Commercial Invoice) 商业发票相关数据模型

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

/// CI 状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CIStatus {
    #[serde(rename = "1")]
    Draft = 1,         // 草稿
    #[serde(rename = "2")]
    Sent = 2,          // 已发送
    #[serde(rename = "3")]
    Paid = 3,          // 已付款
}

impl Default for CIStatus {
    fn default() -> Self {
        CIStatus::Draft
    }
}

impl From<i64> for CIStatus {
    fn from(value: i64) -> Self {
        match value {
            1 => CIStatus::Draft,
            2 => CIStatus::Sent,
            3 => CIStatus::Paid,
            _ => CIStatus::Draft,
        }
    }
}

/// CI 实体
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CommercialInvoice {
    pub id: i64,
    pub ci_code: String,
    pub sales_order_id: i64,
    pub pi_id: Option<i64>,

    // 客户信息
    pub customer_id: Option<i64>,
    pub customer_name: String,
    pub customer_email: Option<String>,
    pub customer_phone: Option<String>,
    pub customer_address: Option<String>,

    // 金额
    pub currency: String,
    pub subtotal: f64,
    pub discount: f64,
    pub total_amount: f64,
    pub paid_amount: f64,

    // 状态
    pub status: i64,

    // 日期
    pub ci_date: String,
    pub paid_at: Option<String>,

    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// CI 明细
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CommercialInvoiceItem {
    pub id: i64,
    pub ci_id: i64,
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

/// 从订单创建 CI 请求
#[derive(Debug, Deserialize, Validate)]
pub struct CreateCIFromOrderRequest {
    pub ci_date: String,
    pub notes: Option<String>,
}

/// 更新 CI 请求
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateCIRequest {
    pub notes: Option<String>,
}

/// 标记已付款请求
#[derive(Debug, Deserialize, Validate)]
pub struct MarkPaidRequest {
    pub paid_amount: f64,
    pub paid_at: Option<String>,
}

/// CI 查询参数
#[derive(Debug, Deserialize)]
pub struct CIQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub status: Option<i64>,
    pub order_id: Option<i64>,
    pub customer_id: Option<i64>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub keyword: Option<String>,
}

impl CIQuery {
    pub fn page(&self) -> u32 {
        self.page.unwrap_or(1).max(1)
    }

    pub fn page_size(&self) -> u32 {
        self.page_size.unwrap_or(20).min(100).max(1)
    }
}

/// CI 详情（包含明细）
#[derive(Debug, Serialize)]
pub struct CIDetail {
    #[serde(flatten)]
    pub ci: CommercialInvoice,
    pub items: Vec<CommercialInvoiceItem>,
}

/// CI 列表项（精简版）
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct CIListItem {
    pub id: i64,
    pub ci_code: String,
    pub sales_order_id: i64,
    pub customer_name: String,
    pub total_amount: f64,
    pub paid_amount: f64,
    pub currency: String,
    pub status: i64,
    pub ci_date: String,
    pub item_count: i64,
    pub created_at: String,
}
