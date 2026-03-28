//! Webhook 相关数据库查询

use anyhow::Result;
use sqlx::{QueryBuilder, SqlitePool};

use cicierp_models::{
    webhook::{WebhookSubscription, WebhookDelivery, CreateWebhookSubscriptionRequest, UpdateWebhookSubscriptionRequest, WebhookSubscriptionQuery},
    common::PagedResponse,
};

pub struct WebhookQueries<'a> {
    pool: &'a SqlitePool,
}

impl<'a> WebhookQueries<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    /// 生成 Webhook Secret
    fn generate_secret() -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let bytes: [u8; 16] = rng.gen();
        hex::encode(bytes)
    }

    /// 获取 Webhook 订阅列表
    pub async fn list_subscriptions(
        &self,
        page: u32,
        page_size: u32,
        client_id: Option<i64>,
        event_type: Option<&str>,
        status: Option<i64>,
    ) -> Result<PagedResponse<WebhookSubscription>> {
        let offset = (page.saturating_sub(1)) * page_size;

        // 构建安全的 count 查询
        let mut count_query = QueryBuilder::new("SELECT COUNT(*) FROM webhook_subscriptions WHERE 1=1");

        if let Some(cid) = client_id {
            count_query.push(" AND client_id = ");
            count_query.push_bind(cid);
        }
        if let Some(et) = event_type {
            count_query.push(" AND event_type = ");
            count_query.push_bind(et);
        }
        if let Some(s) = status {
            count_query.push(" AND status = ");
            count_query.push_bind(s);
        }

        let total: (i64,) = count_query.build_query_as()
            .fetch_one(self.pool)
            .await?;

        // 构建安全的 list 查询
        let mut list_query = QueryBuilder::new("SELECT * FROM webhook_subscriptions WHERE 1=1");

        if let Some(cid) = client_id {
            list_query.push(" AND client_id = ");
            list_query.push_bind(cid);
        }
        if let Some(et) = event_type {
            list_query.push(" AND event_type = ");
            list_query.push_bind(et);
        }
        if let Some(s) = status {
            list_query.push(" AND status = ");
            list_query.push_bind(s);
        }

        list_query.push(" ORDER BY created_at DESC LIMIT ");
        list_query.push_bind(page_size as i64);
        list_query.push(" OFFSET ");
        list_query.push_bind(offset as i64);

        let items: Vec<WebhookSubscription> = list_query.build_query_as()
            .fetch_all(self.pool)
            .await?;

        Ok(PagedResponse::new(items, page, page_size, total.0 as u64))
    }

    /// 根据 ID 获取 Webhook 订阅
    pub async fn get_subscription_by_id(&self, id: i64) -> Result<Option<WebhookSubscription>> {
        let sub: Option<WebhookSubscription> = sqlx::query_as(
            "SELECT * FROM webhook_subscriptions WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await?;

        Ok(sub)
    }

    /// 获取指定客户端的所有活跃订阅
    pub async fn get_active_subscriptions(&self, client_id: i64) -> Result<Vec<WebhookSubscription>> {
        let subs: Vec<WebhookSubscription> = sqlx::query_as(
            "SELECT * FROM webhook_subscriptions WHERE client_id = ? AND status = 1"
        )
        .bind(client_id)
        .fetch_all(self.pool)
        .await?;

        Ok(subs)
    }

    /// 获取指定事件类型的所有活跃订阅
    pub async fn get_active_subscriptions_by_event(&self, event_type: &str) -> Result<Vec<WebhookSubscription>> {
        let subs: Vec<WebhookSubscription> = sqlx::query_as(
            "SELECT * FROM webhook_subscriptions WHERE event_type = ? AND status = 1"
        )
        .bind(event_type)
        .fetch_all(self.pool)
        .await?;

        Ok(subs)
    }

    /// 创建 Webhook 订阅
    pub async fn create_subscription(&self, req: &CreateWebhookSubscriptionRequest) -> Result<WebhookSubscription> {
        let now = chrono::Utc::now().to_rfc3339();
        let secret = req.secret.clone().unwrap_or_else(|| Self::generate_secret());

        let result = sqlx::query(
            r#"
            INSERT INTO webhook_subscriptions (
                client_id, event_type, endpoint_url, secret, status, created_at
            ) VALUES (?, ?, ?, ?, 1, ?)
            "#
        )
        .bind(req.client_id)
        .bind(&req.event_type)
        .bind(&req.endpoint_url)
        .bind(&secret)
        .bind(&now)
        .execute(self.pool)
        .await?;

        let id = result.last_insert_rowid();
        self.get_subscription_by_id(id).await?.ok_or_else(|| anyhow::anyhow!("Failed to fetch created webhook subscription"))
    }

    /// 更新 Webhook 订阅
    pub async fn update_subscription(&self, id: i64, req: &UpdateWebhookSubscriptionRequest) -> Result<Option<WebhookSubscription>> {
        if self.get_subscription_by_id(id).await?.is_none() {
            return Ok(None);
        }

        let mut updates = vec![];
        let mut bindings: Vec<String> = vec![];

        if let Some(ref url) = req.endpoint_url {
            updates.push("endpoint_url = ?");
            bindings.push(url.clone());
        }
        if let Some(ref secret) = req.secret {
            updates.push("secret = ?");
            bindings.push(secret.clone());
        }
        if let Some(s) = req.status {
            updates.push("status = ?");
            bindings.push(s.to_string());
        }

        if updates.is_empty() {
            return self.get_subscription_by_id(id).await;
        }

        let sql = format!(
            "UPDATE webhook_subscriptions SET {} WHERE id = ?",
            updates.join(", ")
        );

        let mut query = sqlx::query(&sql);
        for bind in &bindings {
            query = query.bind(bind);
        }
        query = query.bind(id);
        query.execute(self.pool).await?;

        self.get_subscription_by_id(id).await
    }

    /// 删除 Webhook 订阅
    pub async fn delete_subscription(&self, id: i64) -> Result<bool> {
        let result = sqlx::query(
            "DELETE FROM webhook_subscriptions WHERE id = ?"
        )
        .bind(id)
        .execute(self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    // ========================================
    // Webhook 发送记录
    // ========================================

    /// 创建发送记录
    pub async fn create_delivery(
        &self,
        subscription_id: i64,
        event_type: &str,
        payload: &str,
    ) -> Result<WebhookDelivery> {
        let now = chrono::Utc::now().to_rfc3339();

        let result = sqlx::query(
            r#"
            INSERT INTO webhook_deliveries (
                subscription_id, event_type, payload, attempts, created_at
            ) VALUES (?, ?, ?, 1, ?)
            "#
        )
        .bind(subscription_id)
        .bind(event_type)
        .bind(payload)
        .bind(&now)
        .execute(self.pool)
        .await?;

        let id = result.last_insert_rowid();
        self.get_delivery_by_id(id).await?.ok_or_else(|| anyhow::anyhow!("Failed to fetch created webhook delivery"))
    }

    /// 根据 ID 获取发送记录
    pub async fn get_delivery_by_id(&self, id: i64) -> Result<Option<WebhookDelivery>> {
        let delivery: Option<WebhookDelivery> = sqlx::query_as(
            "SELECT * FROM webhook_deliveries WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await?;

        Ok(delivery)
    }

    /// 更新发送结果
    pub async fn update_delivery_result(
        &self,
        id: i64,
        response_status: Option<i64>,
        response_body: Option<&str>,
        error_message: Option<&str>,
    ) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();

        if response_status.map(|s| s >= 200 && s < 300).unwrap_or(false) {
            // 成功
            sqlx::query(
                r#"
                UPDATE webhook_deliveries SET
                    response_status = ?,
                    response_body = ?,
                    delivered_at = ?,
                    error_message = NULL
                WHERE id = ?
                "#
            )
            .bind(response_status)
            .bind(response_body)
            .bind(&now)
            .bind(id)
            .execute(self.pool)
            .await?;
        } else {
            // 失败，增加重试次数
            sqlx::query(
                r#"
                UPDATE webhook_deliveries SET
                    response_status = ?,
                    response_body = ?,
                    error_message = ?,
                    attempts = attempts + 1
                WHERE id = ?
                "#
            )
            .bind(response_status)
            .bind(response_body)
            .bind(error_message)
            .bind(id)
            .execute(self.pool)
            .await?;
        }

        Ok(())
    }

    /// 获取订阅的发送记录列表
    pub async fn list_deliveries(
        &self,
        subscription_id: i64,
        page: u32,
        page_size: u32,
    ) -> Result<PagedResponse<WebhookDelivery>> {
        let offset = (page.saturating_sub(1)) * page_size;

        let total: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM webhook_deliveries WHERE subscription_id = ?"
        )
        .bind(subscription_id)
        .fetch_one(self.pool)
        .await?;

        let items: Vec<WebhookDelivery> = sqlx::query_as(
            "SELECT * FROM webhook_deliveries WHERE subscription_id = ? ORDER BY created_at DESC LIMIT ? OFFSET ?"
        )
        .bind(subscription_id)
        .bind(page_size as i64)
        .bind(offset as i64)
        .fetch_all(self.pool)
        .await?;

        Ok(PagedResponse::new(items, page, page_size, total.0 as u64))
    }
}
