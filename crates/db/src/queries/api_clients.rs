//! API 客户端相关数据库查询

use anyhow::Result;
use sqlx::{QueryBuilder, SqlitePool};
use serde_json::Value as JsonValue;

use cicierp_models::{
    api_client::{ApiClient, CreateApiClientRequest, UpdateApiClientRequest, ApiClientQuery},
    common::PagedResponse,
};

pub struct ApiClientQueries<'a> {
    pool: &'a SqlitePool,
}

impl<'a> ApiClientQueries<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    /// 生成 API Key
    fn generate_api_key() -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let bytes: [u8; 32] = rng.gen();
        format!("ak_{}", hex::encode(bytes))
    }

    /// 生成 API Secret
    fn generate_api_secret() -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let bytes: [u8; 32] = rng.gen();
        hex::encode(bytes)
    }

    /// 获取 API 客户端列表
    pub async fn list(
        &self,
        page: u32,
        page_size: u32,
        status: Option<i64>,
        keyword: Option<&str>,
    ) -> Result<PagedResponse<ApiClient>> {
        let offset = (page.saturating_sub(1)) * page_size;

        // 构建安全的 count 查询
        let mut count_query = QueryBuilder::new("SELECT COUNT(*) FROM api_clients WHERE 1=1");

        if let Some(s) = status {
            count_query.push(" AND status = ");
            count_query.push_bind(s);
        }
        if let Some(kw) = keyword {
            count_query.push(" AND (client_id LIKE ");
            count_query.push_bind(format!("%{}%", kw));
            count_query.push(" OR client_name LIKE ");
            count_query.push_bind(format!("%{}%", kw));
            count_query.push(")");
        }

        let total: (i64,) = count_query.build_query_as()
            .fetch_one(self.pool)
            .await?;

        // 构建安全的 list 查询
        let mut list_query = QueryBuilder::new("SELECT * FROM api_clients WHERE 1=1");

        if let Some(s) = status {
            list_query.push(" AND status = ");
            list_query.push_bind(s);
        }
        if let Some(kw) = keyword {
            list_query.push(" AND (client_id LIKE ");
            list_query.push_bind(format!("%{}%", kw));
            list_query.push(" OR client_name LIKE ");
            list_query.push_bind(format!("%{}%", kw));
            list_query.push(")");
        }

        list_query.push(" ORDER BY created_at DESC LIMIT ");
        list_query.push_bind(page_size as i64);
        list_query.push(" OFFSET ");
        list_query.push_bind(offset as i64);

        let items: Vec<ApiClient> = list_query.build_query_as()
            .fetch_all(self.pool)
            .await?;

        Ok(PagedResponse::new(items, page, page_size, total.0 as u64))
    }

    /// 根据 ID 获取 API 客户端
    pub async fn get_by_id(&self, id: i64) -> Result<Option<ApiClient>> {
        let client: Option<ApiClient> = sqlx::query_as(
            "SELECT * FROM api_clients WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await?;

        Ok(client)
    }

    /// 根据 client_id 获取 API 客户端
    pub async fn get_by_client_id(&self, client_id: &str) -> Result<Option<ApiClient>> {
        let client: Option<ApiClient> = sqlx::query_as(
            "SELECT * FROM api_clients WHERE client_id = ?"
        )
        .bind(client_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(client)
    }

    /// 根据 API Key 获取 API 客户端
    pub async fn get_by_api_key(&self, api_key: &str) -> Result<Option<ApiClient>> {
        let client: Option<ApiClient> = sqlx::query_as(
            "SELECT * FROM api_clients WHERE api_key = ? AND status = 1"
        )
        .bind(api_key)
        .fetch_optional(self.pool)
        .await?;

        Ok(client)
    }

    /// 创建 API 客户端
    pub async fn create(&self, req: &CreateApiClientRequest) -> Result<ApiClient> {
        let now = chrono::Utc::now().to_rfc3339();
        let api_key = Self::generate_api_key();
        let api_secret = Self::generate_api_secret();
        let permissions = serde_json::to_string(&req.permissions.clone().unwrap_or_default())?;
        let rate_limit = req.rate_limit.unwrap_or(1000);

        let result = sqlx::query(
            r#"
            INSERT INTO api_clients (
                client_id, client_name, api_key, api_secret, permissions, rate_limit,
                status, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, 1, ?, ?)
            "#
        )
        .bind(&req.client_id)
        .bind(&req.client_name)
        .bind(&api_key)
        .bind(&api_secret)
        .bind(&permissions)
        .bind(rate_limit)
        .bind(&now)
        .bind(&now)
        .execute(self.pool)
        .await?;

        let id = result.last_insert_rowid();
        self.get_by_id(id).await?.ok_or_else(|| anyhow::anyhow!("Failed to fetch created API client"))
    }

    /// 更新 API 客户端
    pub async fn update(&self, id: i64, req: &UpdateApiClientRequest) -> Result<Option<ApiClient>> {
        if self.get_by_id(id).await?.is_none() {
            return Ok(None);
        }

        let now = chrono::Utc::now().to_rfc3339();
        let mut updates = vec!["updated_at = ?"];
        let mut bindings: Vec<String> = vec![now.clone()];

        if let Some(ref name) = req.client_name {
            updates.push("client_name = ?");
            bindings.push(name.clone());
        }
        if let Some(ref perms) = req.permissions {
            updates.push("permissions = ?");
            bindings.push(serde_json::to_string(perms)?);
        }
        if let Some(limit) = req.rate_limit {
            updates.push("rate_limit = ?");
            bindings.push(limit.to_string());
        }
        if let Some(s) = req.status {
            updates.push("status = ?");
            bindings.push(s.to_string());
        }

        let sql = format!(
            "UPDATE api_clients SET {} WHERE id = ?",
            updates.join(", ")
        );

        let mut query = sqlx::query(&sql);
        for bind in &bindings {
            query = query.bind(bind);
        }
        query = query.bind(id);
        query.execute(self.pool).await?;

        self.get_by_id(id).await
    }

    /// 重新生成 API Key 和 Secret
    pub async fn regenerate_keys(&self, id: i64) -> Result<Option<(String, String)>> {
        if self.get_by_id(id).await?.is_none() {
            return Ok(None);
        }

        let now = chrono::Utc::now().to_rfc3339();
        let api_key = Self::generate_api_key();
        let api_secret = Self::generate_api_secret();

        sqlx::query(
            "UPDATE api_clients SET api_key = ?, api_secret = ?, updated_at = ? WHERE id = ?"
        )
        .bind(&api_key)
        .bind(&api_secret)
        .bind(&now)
        .bind(id)
        .execute(self.pool)
        .await?;

        Ok(Some((api_key, api_secret)))
    }

    /// 更新最后使用时间
    pub async fn update_last_used(&self, id: i64) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE api_clients SET last_used_at = ? WHERE id = ?"
        )
        .bind(&now)
        .bind(id)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    /// 删除 API 客户端
    pub async fn delete(&self, id: i64) -> Result<bool> {
        let result = sqlx::query(
            "DELETE FROM api_clients WHERE id = ?"
        )
        .bind(id)
        .execute(self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }
}
