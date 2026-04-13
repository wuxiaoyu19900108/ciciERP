//! 库存相关数据模型

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

/// 库存变动类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MovementType {
    #[serde(rename = "1")]
    Inbound = 1,
    #[serde(rename = "2")]
    Outbound = 2,
    #[serde(rename = "3")]
    Transfer = 3,
    #[serde(rename = "4")]
    Adjustment = 4,
    #[serde(rename = "5")]
    Damage = 5,
    #[serde(rename = "6")]
    Lock = 6,
    #[serde(rename = "7")]
    Unlock = 7,
}

/// 库存实体
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Inventory {
    pub id: i64,
    pub product_id: i64,
    pub total_quantity: i64,
    pub available_quantity: i64,
    pub locked_quantity: i64,
    pub damaged_quantity: i64,
    pub safety_stock: i64,
    pub max_stock: Option<i64>,
    pub warehouse_id: Option<i64>,
    pub location: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Inventory {
    pub fn is_low_stock(&self) -> bool {
        self.available_quantity < self.safety_stock
    }

    pub fn can_lock(&self, quantity: i64) -> bool {
        self.available_quantity >= quantity
    }
}

/// 库存流水
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct StockMovement {
    pub id: i64,
    pub movement_code: String,
    pub product_id: i64,
    pub warehouse_id: Option<i64>,
    pub movement_type: i64,
    pub quantity: i64,
    pub before_quantity: i64,
    pub after_quantity: i64,
    pub reference_type: Option<String>,
    pub reference_id: Option<i64>,
    pub reference_code: Option<String>,
    pub note: Option<String>,
    pub operator_id: Option<i64>,
    pub operator_name: Option<String>,
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// 请求/响应 DTOs
// ============================================================================

/// 更新库存请求
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateInventoryRequest {
    #[validate(range(min = 0))]
    pub quantity: i64,
    pub note: Option<String>,
    #[validate(range(min = 0))]
    pub damaged_quantity: Option<i64>,
}

/// 锁定库存请求
#[derive(Debug, Deserialize, Validate)]
pub struct LockInventoryRequest {
    pub product_id: i64,
    #[validate(range(min = 1))]
    pub quantity: i64,
    pub order_id: Option<i64>,
}

/// 解锁库存请求
#[derive(Debug, Deserialize, Validate)]
pub struct UnlockInventoryRequest {
    pub product_id: i64,
    #[validate(range(min = 1))]
    pub quantity: i64,
    pub order_id: Option<i64>,
}

/// 库存查询参数
#[derive(Debug, Deserialize)]
pub struct InventoryQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub low_stock: Option<bool>,
    pub product_code: Option<String>,
    pub product_name: Option<String>,
}

impl InventoryQuery {
    pub fn page(&self) -> u32 {
        self.page.unwrap_or(1).max(1)
    }

    pub fn page_size(&self) -> u32 {
        self.page_size.unwrap_or(20).min(100).max(1)
    }
}

/// 库存列表项（包含产品信息）
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct InventoryListItem {
    pub id: i64,
    pub product_id: i64,
    pub product_code: String,
    pub product_name: String,
    pub total_quantity: i64,
    pub available_quantity: i64,
    pub locked_quantity: i64,
    pub safety_stock: i64,
    pub is_low_stock: bool,
}

/// 库存预警项
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct InventoryAlert {
    pub product_id: i64,
    pub product_code: String,
    pub product_name: String,
    pub available_quantity: i64,
    pub safety_stock: i64,
    pub shortage: i64,
}
