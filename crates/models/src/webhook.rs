//! Webhook 相关数据模型

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

/// Webhook 订阅状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WebhookSubscriptionStatus {
    #[serde(rename = "1")]
    Active = 1,
    #[serde(rename = "0")]
    Inactive = 0,
}

impl Default for WebhookSubscriptionStatus {
    fn default() -> Self {
        WebhookSubscriptionStatus::Active
    }
}

/// Webhook 事件类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebhookEventType {
    // 订单事件
    OrderShipped,
    OrderCancelled,
    OrderCompleted,
    // 库存事件
    InventoryLowStock,
    InventoryOutOfStock,
    // 其他事件
    #[serde(other)]
    Unknown,
}

impl WebhookEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            WebhookEventType::OrderShipped => "order.shipped",
            WebhookEventType::OrderCancelled => "order.cancelled",
            WebhookEventType::OrderCompleted => "order.completed",
            WebhookEventType::InventoryLowStock => "inventory.low_stock",
            WebhookEventType::InventoryOutOfStock => "inventory.out_of_stock",
            WebhookEventType::Unknown => "unknown",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "order.shipped" => WebhookEventType::OrderShipped,
            "order.cancelled" => WebhookEventType::OrderCancelled,
            "order.completed" => WebhookEventType::OrderCompleted,
            "inventory.low_stock" => WebhookEventType::InventoryLowStock,
            "inventory.out_of_stock" => WebhookEventType::InventoryOutOfStock,
            _ => WebhookEventType::Unknown,
        }
    }
}

/// Webhook 订阅实体
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WebhookSubscription {
    pub id: i64,
    pub client_id: i64,
    pub event_type: String,
    pub endpoint_url: String,
    pub secret: String,
    pub status: i64,
    pub created_at: DateTime<Utc>,
}

/// Webhook 发送记录实体
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WebhookDelivery {
    pub id: i64,
    pub subscription_id: i64,
    pub event_type: String,
    pub payload: String,
    pub response_status: Option<i64>,
    pub response_body: Option<String>,
    pub attempts: i64,
    pub delivered_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// 创建 Webhook 订阅请求
#[derive(Debug, Deserialize, Validate)]
pub struct CreateWebhookSubscriptionRequest {
    pub client_id: i64,
    #[validate(length(min = 1, max = 50))]
    pub event_type: String,
    #[validate(url)]
    pub endpoint_url: String,
    pub secret: Option<String>,
}

/// 更新 Webhook 订阅请求
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateWebhookSubscriptionRequest {
    #[validate(url)]
    pub endpoint_url: Option<String>,
    pub secret: Option<String>,
    pub status: Option<i64>,
}

/// Webhook 查询参数
#[derive(Debug, Deserialize)]
pub struct WebhookSubscriptionQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub client_id: Option<i64>,
    pub event_type: Option<String>,
    pub status: Option<i64>,
}

impl WebhookSubscriptionQuery {
    pub fn page(&self) -> u32 {
        self.page.unwrap_or(1).max(1)
    }

    pub fn page_size(&self) -> u32 {
        self.page_size.unwrap_or(20).min(100).max(1)
    }
}

/// Webhook 推送载荷
#[derive(Debug, Serialize)]
pub struct WebhookPayload {
    pub event: String,
    pub timestamp: DateTime<Utc>,
    pub data: serde_json::Value,
    pub signature: String,
}

impl WebhookPayload {
    pub fn new(event: &str, data: serde_json::Value, secret: &str) -> Self {
        let timestamp = Utc::now();
        let signature = Self::compute_signature(event, &timestamp, &data, secret);
        WebhookPayload {
            event: event.to_string(),
            timestamp,
            data,
            signature,
        }
    }

    /// 计算签名
    pub fn compute_signature(
        event: &str,
        timestamp: &DateTime<Utc>,
        data: &serde_json::Value,
        secret: &str,
    ) -> String {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        type HmacSha256 = Hmac<Sha256>;

        let payload = format!(
            "{}{}{}",
            event,
            timestamp.to_rfc3339(),
            serde_json::to_string(data).unwrap_or_default()
        );

        let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(payload.as_bytes());
        let result = mac.finalize();

        format!("sha256={}", hex::encode(result.into_bytes()))
    }
}

/// 订单发货事件数据
#[derive(Debug, Serialize)]
pub struct OrderShippedData {
    pub order_id: i64,
    pub order_code: String,
    pub platform_order_id: Option<String>,
    pub tracking_number: Option<String>,
    pub logistics_name: Option<String>,
    pub ship_time: DateTime<Utc>,
}

/// 订单取消事件数据
#[derive(Debug, Serialize)]
pub struct OrderCancelledData {
    pub order_id: i64,
    pub order_code: String,
    pub platform_order_id: Option<String>,
    pub cancel_reason: Option<String>,
    pub cancel_time: DateTime<Utc>,
}

/// 库存预警事件数据
#[derive(Debug, Serialize)]
pub struct InventoryLowStockData {
    pub sku_id: i64,
    pub sku_code: String,
    pub product_name: String,
    pub available_quantity: i64,
    pub safety_stock: i64,
    pub warehouse_id: Option<i64>,
}
