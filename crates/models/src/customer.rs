//! 客户相关数据模型

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use validator::Validate;

/// 客户状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CustomerStatus {
    #[serde(rename = "1")]
    Active = 1,     // 正常
    #[serde(rename = "2")]
    Frozen = 2,     // 冻结
    #[serde(rename = "3")]
    Blacklist = 3,  // 黑名单
}

impl Default for CustomerStatus {
    fn default() -> Self {
        CustomerStatus::Active
    }
}

/// 客户实体
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Customer {
    pub id: i64,
    pub customer_code: String,
    pub name: String,
    pub mobile: Option<String>,
    pub email: Option<String>,
    pub gender: Option<i64>,
    pub birthday: Option<String>,
    pub avatar: Option<String>,
    pub level_id: Option<i64>,
    pub points: i64,
    pub total_orders: i64,
    pub total_amount: f64,
    pub avg_order_amount: Option<f64>,
    pub tags: JsonValue,
    pub attributes: JsonValue,
    pub source: String,
    pub external_id: Option<String>,
    pub external_platform: Option<String>,
    pub status: i64,
    pub lead_status: i64,
    pub notes: Option<String>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub last_order_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

/// 客户地址
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CustomerAddress {
    pub id: i64,
    pub customer_id: i64,
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
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 客户等级
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CustomerLevel {
    pub id: i64,
    pub name: String,
    pub name_en: Option<String>,
    pub level: i64,
    pub min_amount: f64,
    pub min_orders: i64,
    pub min_points: i64,
    pub discount_percent: f64,
    pub free_shipping: bool,
    pub special_services: Option<String>,
    pub sort_order: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ============================================================================
// 请求/响应 DTOs
// ============================================================================

/// 创建客户请求（简化版）
#[derive(Debug, Deserialize, Validate)]
pub struct CreateCustomerRequest {
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    #[validate(length(min = 6, max = 20))]
    pub mobile: String,  // 改为必填
    #[validate(email)]
    pub email: Option<String>,
    pub status: Option<i64>,
    pub lead_status: Option<i64>,
    pub notes: Option<String>,
    pub source: Option<String>,
}

/// 更新客户请求（简化版）
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateCustomerRequest {
    #[validate(length(min = 1, max = 100))]
    pub name: Option<String>,
    #[validate(length(min = 6, max = 20))]
    pub mobile: Option<String>,
    #[validate(email)]
    pub email: Option<String>,
    pub status: Option<i64>,
    pub lead_status: Option<i64>,
    pub notes: Option<String>,
}

/// 客户查询参数
#[derive(Debug, Deserialize)]
pub struct CustomerQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub level_id: Option<i64>,
    pub status: Option<i64>,
    pub lead_status: Option<i64>,
    pub keyword: Option<String>,
    pub source: Option<String>,
}

impl CustomerQuery {
    pub fn page(&self) -> u32 {
        self.page.unwrap_or(1).max(1)
    }

    pub fn page_size(&self) -> u32 {
        self.page_size.unwrap_or(20).min(100).max(1)
    }
}
