//! 汇率相关数据库查询

use anyhow::Result;
use chrono::Utc;
use sqlx::SqlitePool;

use cicierp_models::exchange_rate::{ExchangeRate, CreateExchangeRateRequest, ExchangeRateHistoryQuery};

/// 汇率查询结构体
pub struct ExchangeRateQueries<'a> {
    pool: &'a SqlitePool,
}

impl<'a> ExchangeRateQueries<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    /// 获取当日汇率（如果没有则返回最新历史汇率）
    pub async fn get_current_rate(&self, from_currency: &str, to_currency: &str) -> Result<Option<ExchangeRate>> {
        let today = Utc::now().format("%Y-%m-%d").to_string();

        // 先尝试获取当日汇率
        let rate: Option<ExchangeRate> = sqlx::query_as(
            "SELECT * FROM exchange_rates WHERE from_currency = ? AND to_currency = ? AND effective_date = ? LIMIT 1"
        )
        .bind(from_currency)
        .bind(to_currency)
        .bind(&today)
        .fetch_optional(self.pool)
        .await?;

        if rate.is_some() {
            return Ok(rate);
        }

        // 如果没有当日汇率，返回最新历史汇率
        self.get_latest_rate(from_currency, to_currency).await
    }

    /// 获取最新汇率
    pub async fn get_latest_rate(&self, from_currency: &str, to_currency: &str) -> Result<Option<ExchangeRate>> {
        let rate: Option<ExchangeRate> = sqlx::query_as(
            "SELECT * FROM exchange_rates WHERE from_currency = ? AND to_currency = ? ORDER BY effective_date DESC LIMIT 1"
        )
        .bind(from_currency)
        .bind(to_currency)
        .fetch_optional(self.pool)
        .await?;

        Ok(rate)
    }

    /// 根据ID获取汇率
    pub async fn get_by_id(&self, id: i64) -> Result<Option<ExchangeRate>> {
        let rate: Option<ExchangeRate> = sqlx::query_as(
            "SELECT * FROM exchange_rates WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await?;

        Ok(rate)
    }

    /// 创建汇率记录
    pub async fn create(&self, req: &CreateExchangeRateRequest) -> Result<ExchangeRate> {
        let now = Utc::now().to_rfc3339();
        let source = req.source.clone().unwrap_or_else(|| "api".to_string());
        let effective_date = req.effective_date.clone().unwrap_or_else(|| Utc::now().format("%Y-%m-%d").to_string());

        // 使用 INSERT OR REPLACE 处理唯一约束冲突
        let result = sqlx::query(
            r#"
            INSERT OR REPLACE INTO exchange_rates (
                from_currency, to_currency, rate, source, effective_date, created_at
            ) VALUES (?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&req.from_currency)
        .bind(&req.to_currency)
        .bind(req.rate)
        .bind(&source)
        .bind(&effective_date)
        .bind(&now)
        .execute(self.pool)
        .await?;

        let id = result.last_insert_rowid();

        // 如果是 REPLACE，last_insert_rowid 可能返回旧的 id，需要重新查询
        self.get_by_date(&req.from_currency, &req.to_currency, &effective_date).await?
            .ok_or_else(|| anyhow::anyhow!("Failed to fetch created exchange rate"))
    }

    /// 根据日期获取汇率
    pub async fn get_by_date(&self, from_currency: &str, to_currency: &str, date: &str) -> Result<Option<ExchangeRate>> {
        let rate: Option<ExchangeRate> = sqlx::query_as(
            "SELECT * FROM exchange_rates WHERE from_currency = ? AND to_currency = ? AND effective_date = ? LIMIT 1"
        )
        .bind(from_currency)
        .bind(to_currency)
        .bind(date)
        .fetch_optional(self.pool)
        .await?;

        Ok(rate)
    }

    /// 获取汇率历史
    pub async fn list_history(&self, query: &ExchangeRateHistoryQuery) -> Result<Vec<ExchangeRate>> {
        let from = query.from_currency.clone().unwrap_or_else(|| "USD".to_string());
        let to = query.to_currency.clone().unwrap_or_else(|| "CNY".to_string());
        let limit = query.limit.unwrap_or(30);

        let rates: Vec<ExchangeRate> = if let (Some(start), Some(end)) = (&query.start_date, &query.end_date) {
            sqlx::query_as(
                "SELECT * FROM exchange_rates WHERE from_currency = ? AND to_currency = ? AND effective_date BETWEEN ? AND ? ORDER BY effective_date DESC LIMIT ?"
            )
            .bind(&from)
            .bind(&to)
            .bind(start)
            .bind(end)
            .bind(limit)
            .fetch_all(self.pool)
            .await?
        } else {
            sqlx::query_as(
                "SELECT * FROM exchange_rates WHERE from_currency = ? AND to_currency = ? ORDER BY effective_date DESC LIMIT ?"
            )
            .bind(&from)
            .bind(&to)
            .bind(limit)
            .fetch_all(self.pool)
            .await?
        };

        Ok(rates)
    }

    /// 删除旧汇率记录（保留最近 N 天）
    pub async fn cleanup_old_rates(&self, days_to_keep: i32) -> Result<u64> {
        let cutoff_date = Utc::now() - chrono::Duration::days(days_to_keep as i64);
        let cutoff_str = cutoff_date.format("%Y-%m-%d").to_string();

        let result = sqlx::query(
            "DELETE FROM exchange_rates WHERE effective_date < ?"
        )
        .bind(&cutoff_str)
        .execute(self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}
