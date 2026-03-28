//! Webhook 推送服务
//!
//! 负责向订阅的客户端发送事件通知

use std::sync::Arc;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::Serialize;
use tracing::{info, warn, error, debug};

use cicierp_db::queries::webhooks::WebhookQueries;
use cicierp_models::webhook::{WebhookPayload, WebhookEventType, OrderShippedData, OrderCancelledData, InventoryLowStockData};

/// Webhook 推送服务
pub struct WebhookService {
    db_pool: sqlx::SqlitePool,
    http_client: Client,
}

impl WebhookService {
    pub fn new(db_pool: sqlx::SqlitePool) -> Self {
        let http_client = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        Self { db_pool, http_client }
    }

    /// 发送订单发货通知
    pub async fn notify_order_shipped(
        &self,
        order_id: i64,
        order_code: &str,
        platform_order_id: Option<&str>,
        tracking_number: Option<&str>,
        logistics_name: Option<&str>,
    ) -> anyhow::Result<()> {
        let event_type = WebhookEventType::OrderShipped.as_str();
        let ship_time = Utc::now();

        let data = OrderShippedData {
            order_id,
            order_code: order_code.to_string(),
            platform_order_id: platform_order_id.map(|s| s.to_string()),
            tracking_number: tracking_number.map(|s| s.to_string()),
            logistics_name: logistics_name.map(|s| s.to_string()),
            ship_time,
        };

        self.send_webhook(event_type, &data).await
    }

    /// 发送订单取消通知
    pub async fn notify_order_cancelled(
        &self,
        order_id: i64,
        order_code: &str,
        platform_order_id: Option<&str>,
        cancel_reason: Option<&str>,
    ) -> anyhow::Result<()> {
        let event_type = WebhookEventType::OrderCancelled.as_str();
        let cancel_time = Utc::now();

        let data = OrderCancelledData {
            order_id,
            order_code: order_code.to_string(),
            platform_order_id: platform_order_id.map(|s| s.to_string()),
            cancel_reason: cancel_reason.map(|s| s.to_string()),
            cancel_time,
        };

        self.send_webhook(event_type, &data).await
    }

    /// 发送库存预警通知
    pub async fn notify_inventory_low_stock(
        &self,
        sku_id: i64,
        sku_code: &str,
        product_name: &str,
        available_quantity: i64,
        safety_stock: i64,
        warehouse_id: Option<i64>,
    ) -> anyhow::Result<()> {
        let event_type = WebhookEventType::InventoryLowStock.as_str();

        let data = InventoryLowStockData {
            sku_id,
            sku_code: sku_code.to_string(),
            product_name: product_name.to_string(),
            available_quantity,
            safety_stock,
            warehouse_id,
        };

        self.send_webhook(event_type, &data).await
    }

    /// 发送 Webhook 通知
    async fn send_webhook<T: Serialize>(&self, event_type: &str, data: &T) -> anyhow::Result<()> {
        let queries = WebhookQueries::new(&self.db_pool);

        // 获取所有订阅此事件的活跃订阅
        let subscriptions = queries.get_active_subscriptions_by_event(event_type).await?;

        if subscriptions.is_empty() {
            debug!("No active subscriptions for event: {}", event_type);
            return Ok(());
        }

        info!("Sending webhook {} to {} subscribers", event_type, subscriptions.len());

        // 并发发送到所有订阅者
        let mut handles = vec![];

        for subscription in subscriptions {
            let http_client = self.http_client.clone();
            let queries_clone = WebhookQueries::new(&self.db_pool);
            let event_type = event_type.to_string();
            let data_json = serde_json::to_value(data)?;
            let secret = subscription.secret.clone();
            let endpoint_url = subscription.endpoint_url.clone();
            let subscription_id = subscription.id;

            let handle = tokio::spawn(async move {
                let payload = WebhookPayload::new(&event_type, data_json, &secret);
                let payload_json = serde_json::to_string(&payload).unwrap_or_default();

                // 创建发送记录
                let delivery = match queries_clone.create_delivery(subscription_id, &event_type, &payload_json).await {
                    Ok(d) => d,
                    Err(e) => {
                        error!("Failed to create delivery record: {}", e);
                        return;
                    }
                };

                // 发送请求
                match http_client
                    .post(&endpoint_url)
                    .header("Content-Type", "application/json")
                    .header("X-Webhook-Event", &event_type)
                    .header("X-Webhook-Signature", &payload.signature)
                    .body(payload_json.clone())
                    .send()
                    .await
                {
                    Ok(response) => {
                        let status = response.status().as_u16() as i64;
                        let body = response.text().await.unwrap_or_default();

                        // 更新发送记录
                        if let Err(e) = queries_clone
                            .update_delivery_result(delivery.id, Some(status), Some(&body), None)
                            .await
                        {
                            error!("Failed to update delivery record: {}", e);
                        }

                        if status >= 200 && status < 300 {
                            info!("Webhook delivered successfully to {} (status={})", endpoint_url, status);
                        } else {
                            warn!("Webhook delivery failed to {} (status={})", endpoint_url, status);
                        }
                    }
                    Err(e) => {
                        let error_msg = e.to_string();
                        error!("Webhook delivery error to {}: {}", endpoint_url, error_msg);

                        // 更新发送记录
                        if let Err(e) = queries_clone
                            .update_delivery_result(delivery.id, None, None, Some(&error_msg))
                            .await
                        {
                            error!("Failed to update delivery record: {}", e);
                        }
                    }
                }
            });

            handles.push(handle);
        }

        // 等待所有发送完成
        for handle in handles {
            let _ = handle.await;
        }

        Ok(())
    }
}

/// 全局 Webhook 服务实例
static WEBHOOK_SERVICE: std::sync::OnceLock<Arc<WebhookService>> = std::sync::OnceLock::new();

/// 初始化 Webhook 服务
pub fn init_webhook_service(db_pool: sqlx::SqlitePool) {
    let service = Arc::new(WebhookService::new(db_pool));
    let _ = WEBHOOK_SERVICE.set(service);
}

/// 获取 Webhook 服务实例
pub fn get_webhook_service() -> Option<Arc<WebhookService>> {
    WEBHOOK_SERVICE.get().cloned()
}
