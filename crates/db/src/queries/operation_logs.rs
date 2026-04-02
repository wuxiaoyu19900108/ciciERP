//! 操作日志查询

use anyhow::Result;
use serde::Serialize;
use sqlx::SqlitePool;

pub struct OperationLogQueries<'a> {
    pool: &'a SqlitePool,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct OperationLog {
    pub id: i64,
    pub user_id: Option<i64>,
    pub username: Option<String>,
    pub action: String,
    pub module: String,
    pub target_id: Option<i64>,
    pub target_code: Option<String>,
    pub description: Option<String>,
    pub ip_address: Option<String>,
    pub created_at: String,
}

impl<'a> OperationLogQueries<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn log(
        &self,
        user_id: Option<i64>,
        username: Option<&str>,
        action: &str,
        module: &str,
        target_id: Option<i64>,
        target_code: Option<&str>,
        description: Option<&str>,
    ) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO operation_logs (user_id, username, action, module, target_id, target_code, description, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(user_id)
        .bind(username)
        .bind(action)
        .bind(module)
        .bind(target_id)
        .bind(target_code)
        .bind(description)
        .bind(&now)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    pub async fn recent(&self, limit: i64) -> Result<Vec<OperationLog>> {
        let logs: Vec<OperationLog> = sqlx::query_as(
            "SELECT * FROM operation_logs ORDER BY created_at DESC LIMIT ?"
        )
        .bind(limit)
        .fetch_all(self.pool)
        .await?;
        Ok(logs)
    }
}
