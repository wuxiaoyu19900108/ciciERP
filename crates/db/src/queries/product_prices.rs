//! 产品销售价格相关数据库查询

use anyhow::Result;
use sqlx::SqlitePool;

use cicierp_models::product::{
    CreateProductPriceRequest, ProductPrice, ProductPriceSummary, UpdateProductPriceRequest,
};

/// 产品销售价格查询结构体
pub struct ProductPriceQueries<'a> {
    pool: &'a SqlitePool,
}

impl<'a> ProductPriceQueries<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    /// 根据产品ID获取参考售价
    pub async fn get_reference_price(&self, product_id: i64, platform: &str) -> Result<Option<ProductPrice>> {
        let price: Option<ProductPrice> = sqlx::query_as(
            "SELECT * FROM product_prices WHERE product_id = ? AND platform = ? AND is_reference = 1 LIMIT 1"
        )
        .bind(product_id)
        .bind(platform)
        .fetch_optional(self.pool)
        .await?;

        Ok(price)
    }

    /// 根据产品ID获取所有平台价格
    pub async fn list_by_product(&self, product_id: i64) -> Result<Vec<ProductPrice>> {
        let prices: Vec<ProductPrice> = sqlx::query_as(
            "SELECT * FROM product_prices WHERE product_id = ? ORDER BY platform, created_at DESC"
        )
        .bind(product_id)
        .fetch_all(self.pool)
        .await?;

        Ok(prices)
    }

    /// 根据ID获取价格记录
    pub async fn get_by_id(&self, id: i64) -> Result<Option<ProductPrice>> {
        let price: Option<ProductPrice> = sqlx::query_as(
            "SELECT * FROM product_prices WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await?;

        Ok(price)
    }

    /// 获取产品的价格统计
    pub async fn get_price_summary(&self, product_id: i64) -> Result<ProductPriceSummary> {
        let summary: ProductPriceSummary = sqlx::query_as(
            r#"
            SELECT
                ? as product_id,
                (SELECT cost_cny FROM product_costs WHERE product_id = ? AND is_reference = 1 LIMIT 1) as reference_cost_cny,
                (SELECT SUM(cost_cny * quantity) / SUM(quantity) FROM product_costs WHERE product_id = ? AND quantity > 0) as avg_cost_cny,
                (SELECT sale_price_cny FROM product_prices WHERE product_id = ? AND is_reference = 1 AND platform = 'website' LIMIT 1) as reference_price_cny,
                (SELECT MIN(sale_price_cny) FROM product_prices WHERE product_id = ?) as min_price_cny,
                (SELECT MAX(sale_price_cny) FROM product_prices WHERE product_id = ?) as max_price_cny
            "#
        )
        .bind(product_id)
        .bind(product_id)
        .bind(product_id)
        .bind(product_id)
        .bind(product_id)
        .bind(product_id)
        .fetch_one(self.pool)
        .await?;

        Ok(summary)
    }

    /// 创建价格记录
    pub async fn create(&self, req: &CreateProductPriceRequest) -> Result<ProductPrice> {
        let now = chrono::Utc::now().to_rfc3339();
        let platform = req.platform.clone().unwrap_or_else(|| "website".to_string());
        let exchange_rate = req.exchange_rate.unwrap_or(7.2);
        let profit_margin = req.profit_margin.unwrap_or(0.15);
        let platform_fee_rate = req.platform_fee_rate.unwrap_or(0.025);
        let is_reference = req.is_reference.unwrap_or(false) as i64;

        let result = sqlx::query(
            r#"
            INSERT INTO product_prices (
                product_id, platform, sale_price_cny, sale_price_usd, exchange_rate,
                profit_margin, platform_fee_rate, platform_fee, is_reference,
                effective_date, notes, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(req.product_id)
        .bind(&platform)
        .bind(req.sale_price_cny)
        .bind(req.sale_price_usd)
        .bind(exchange_rate)
        .bind(profit_margin)
        .bind(platform_fee_rate)
        .bind(req.platform_fee)
        .bind(is_reference)
        .bind(&req.effective_date)
        .bind(&req.notes)
        .bind(&now)
        .bind(&now)
        .execute(self.pool)
        .await?;

        let id = result.last_insert_rowid();
        self.get_by_id(id).await?.ok_or_else(|| anyhow::anyhow!("Failed to fetch created product price"))
    }

    /// 更新价格记录
    pub async fn update(&self, id: i64, req: &UpdateProductPriceRequest) -> Result<Option<ProductPrice>> {
        if self.get_by_id(id).await?.is_none() {
            return Ok(None);
        }

        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            UPDATE product_prices SET
                platform = COALESCE(?, platform),
                sale_price_cny = COALESCE(?, sale_price_cny),
                sale_price_usd = COALESCE(?, sale_price_usd),
                exchange_rate = COALESCE(?, exchange_rate),
                profit_margin = COALESCE(?, profit_margin),
                platform_fee_rate = COALESCE(?, platform_fee_rate),
                platform_fee = COALESCE(?, platform_fee),
                is_reference = COALESCE(?, is_reference),
                effective_date = COALESCE(?, effective_date),
                notes = COALESCE(?, notes),
                updated_at = ?
            WHERE id = ?
            "#
        )
        .bind(req.platform.as_ref())
        .bind(req.sale_price_cny)
        .bind(req.sale_price_usd)
        .bind(req.exchange_rate)
        .bind(req.profit_margin)
        .bind(req.platform_fee_rate)
        .bind(req.platform_fee)
        .bind(req.is_reference)
        .bind(req.effective_date.as_ref())
        .bind(req.notes.as_ref())
        .bind(&now)
        .bind(id)
        .execute(self.pool)
        .await?;

        self.get_by_id(id).await
    }

    /// 删除价格记录
    pub async fn delete(&self, id: i64) -> Result<bool> {
        let result = sqlx::query("DELETE FROM product_prices WHERE id = ?")
            .bind(id)
            .execute(self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// 删除产品的所有价格记录
    pub async fn delete_by_product(&self, product_id: i64) -> Result<u64> {
        let result = sqlx::query("DELETE FROM product_prices WHERE product_id = ?")
            .bind(product_id)
            .execute(self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    /// 更新或创建参考售价
    pub async fn update_reference_price(
        &self,
        product_id: i64,
        platform: &str,
        sale_price_cny: f64,
        sale_price_usd: Option<f64>,
        exchange_rate: f64,
        profit_margin: Option<f64>,
        platform_fee_rate: Option<f64>,
        notes: Option<String>,
    ) -> Result<ProductPrice> {
        // 先尝试获取现有的参考售价
        if let Some(existing) = self.get_reference_price(product_id, platform).await? {
            // 更新现有记录
            let now = chrono::Utc::now().to_rfc3339();
            sqlx::query(
                r#"
                UPDATE product_prices SET
                    sale_price_cny = ?,
                    sale_price_usd = ?,
                    exchange_rate = ?,
                    profit_margin = COALESCE(?, profit_margin),
                    platform_fee_rate = COALESCE(?, platform_fee_rate),
                    notes = ?,
                    updated_at = ?
                WHERE id = ?
                "#
            )
            .bind(sale_price_cny)
            .bind(sale_price_usd)
            .bind(exchange_rate)
            .bind(profit_margin)
            .bind(platform_fee_rate)
            .bind(&notes)
            .bind(&now)
            .bind(existing.id)
            .execute(self.pool)
            .await?;

            self.get_by_id(existing.id).await?.ok_or_else(|| anyhow::anyhow!("Failed to fetch updated price"))
        } else {
            // 创建新记录
            let req = CreateProductPriceRequest {
                product_id,
                platform: Some(platform.to_string()),
                sale_price_cny,
                sale_price_usd,
                exchange_rate: Some(exchange_rate),
                profit_margin,
                platform_fee_rate,
                platform_fee: None,
                is_reference: Some(true),
                effective_date: None,
                notes,
            };
            self.create(&req).await
        }
    }

    /// 计算建议售价
    /// 公式: suggested_price = (cost_cny / exchange_rate) * (1 + profit_margin) / (1 - platform_fee_rate)
    pub fn calculate_suggested_price(
        cost_cny: f64,
        exchange_rate: f64,
        profit_margin: f64,
        platform_fee_rate: f64,
    ) -> f64 {
        let cost_usd = cost_cny / exchange_rate;
        let with_margin = cost_usd * (1.0 + profit_margin);
        let final_price = with_margin / (1.0 - platform_fee_rate);
        (final_price * 100.0).round() / 100.0  // 保留两位小数
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_suggested_price() {
        // 成本 100 CNY，汇率 7.2，利润率 15%，平台费率 2.5%
        let price = ProductPriceQueries::calculate_suggested_price(100.0, 7.2, 0.15, 0.025);
        // cost_usd = 13.89, with_margin = 15.97, final = 16.38
        assert!(price > 16.0 && price < 17.0);
    }
}
