//! 供应商相关数据模型

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

/// 供应商状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SupplierStatus {
    #[serde(rename = "1")]
    Active = 1,     // 合作中
    #[serde(rename = "2")]
    Suspended = 2,  // 暂停
    #[serde(rename = "3")]
    Terminated = 3, // 终止
}

impl Default for SupplierStatus {
    fn default() -> Self {
        SupplierStatus::Active
    }
}

/// 供应商实体
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Supplier {
    pub id: i64,
    pub supplier_code: String,
    pub name: String,
    pub name_en: Option<String>,
    pub contact_person: Option<String>,
    pub contact_phone: Option<String>,
    pub contact_email: Option<String>,
    pub address: Option<String>,
    pub credit_code: Option<String>,
    pub tax_id: Option<String>,
    pub bank_name: Option<String>,
    pub bank_account: Option<String>,
    pub rating_level: String,
    pub rating_score: f64,
    pub payment_terms: i64,
    pub payment_method: Option<String>,
    pub total_orders: i64,
    pub total_amount: f64,
    pub status: i64,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

/// 产品-供应商关联
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProductSupplier {
    pub id: i64,
    pub product_id: i64,
    pub supplier_id: i64,
    pub supplier_sku: Option<String>,
    pub purchase_price: Option<f64>,
    pub min_order_qty: i64,
    pub lead_time: Option<i64>,
    pub is_primary: bool,
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// 请求/响应 DTOs
// ============================================================================

/// 创建供应商请求（简化版）
#[derive(Debug, Deserialize, Validate)]
pub struct CreateSupplierRequest {
    #[validate(length(min = 1, max = 50))]
    pub supplier_code: Option<String>,  // 改为可选，自动生成
    #[validate(length(min = 1, max = 200))]
    pub name: String,
    pub name_en: Option<String>,
    pub contact_person: Option<String>,
    pub contact_phone: Option<String>,
    #[validate(email)]
    pub contact_email: Option<String>,
    pub address: Option<String>,
    pub credit_code: Option<String>,
    pub tax_id: Option<String>,
    pub bank_name: Option<String>,
    pub bank_account: Option<String>,
    pub rating_level: Option<String>,
    pub rating_score: Option<f64>,
    pub payment_terms: Option<i64>,
    pub payment_method: Option<String>,
    pub notes: Option<String>,
}

/// 更新供应商请求
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateSupplierRequest {
    #[validate(length(min = 1, max = 200))]
    pub name: Option<String>,
    pub name_en: Option<String>,
    pub contact_person: Option<String>,
    pub contact_phone: Option<String>,
    #[validate(email)]
    pub contact_email: Option<String>,
    pub address: Option<String>,
    pub credit_code: Option<String>,
    pub tax_id: Option<String>,
    pub bank_name: Option<String>,
    pub bank_account: Option<String>,
    pub rating_level: Option<String>,
    pub rating_score: Option<f64>,
    pub payment_terms: Option<i64>,
    pub payment_method: Option<String>,
    pub status: Option<i64>,
    pub notes: Option<String>,
}

/// 供应商查询参数
#[derive(Debug, Deserialize)]
pub struct SupplierQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub status: Option<i64>,
    pub rating_level: Option<String>,
    pub keyword: Option<String>,
}

impl SupplierQuery {
    pub fn page(&self) -> u32 {
        self.page.unwrap_or(1).max(1)
    }

    pub fn page_size(&self) -> u32 {
        self.page_size.unwrap_or(20).min(100).max(1)
    }
}

/// 供应商详情（包含关联的产品）
#[derive(Debug, Serialize)]
pub struct SupplierDetail {
    #[serde(flatten)]
    pub supplier: Supplier,
    pub products: Vec<ProductSupplierInfo>,
}

/// 供应商的产品信息
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ProductSupplierInfo {
    pub product_id: i64,
    pub product_code: String,
    pub product_name: String,
    pub supplier_sku: Option<String>,
    pub purchase_price: Option<f64>,
    pub min_order_qty: i64,
    pub lead_time: Option<i64>,
    pub is_primary: bool,
}
