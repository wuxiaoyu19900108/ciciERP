//! 产品销售价格相关数据库查询

use anyhow::Result;
use sqlx::{Sqlite, SqlitePool, Transaction};

use cicierp_models::product::{
    CreateProductPriceRequest, ProductPrice, ProductPriceSummary, UpdateProductPriceRequest,
};

const PRODUCT_PRICE_SELECT: &str = r#"
    SELECT
        id,
        product_id,
        platform,
        sale_price_cny,
        sale_price_usd,
        exchange_rate,
        profit_margin,
        platform_fee_rate,
        platform_fee,
        is_reference,
        effective_date,
        notes,
        created_at,
        updated_at,
        pricing_mode,
        input_currency,
        reference_platform,
        adjustment_type,
        adjustment_value
    FROM product_prices
"#;

#[derive(Debug, Clone)]
pub struct ReferencePriceWrite {
    pub product_id: i64,
    pub platform: String,
    pub sale_price_cny: f64,
    pub sale_price_usd: Option<f64>,
    pub exchange_rate: f64,
    pub profit_margin: Option<f64>,
    pub platform_fee_rate: Option<f64>,
    pub notes: Option<String>,
    pub pricing_mode: Option<String>,
    pub input_currency: Option<String>,
    pub reference_platform: Option<String>,
    pub adjustment_type: Option<String>,
    pub adjustment_value: Option<f64>,
}

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
        let price: Option<ProductPrice> = sqlx::query_as(&format!(
            "{} WHERE product_id = ? AND platform = ? AND is_reference = 1 LIMIT 1",
            PRODUCT_PRICE_SELECT
        ))
        .bind(product_id)
        .bind(platform)
        .fetch_optional(self.pool)
        .await?;

        Ok(price)
    }

    /// 根据产品ID获取所有平台价格
    pub async fn list_by_product(&self, product_id: i64) -> Result<Vec<ProductPrice>> {
        let prices: Vec<ProductPrice> = sqlx::query_as(&format!(
            "{} WHERE product_id = ? ORDER BY platform, created_at DESC",
            PRODUCT_PRICE_SELECT
        ))
        .bind(product_id)
        .fetch_all(self.pool)
        .await?;

        Ok(prices)
    }

    /// 根据ID获取价格记录
    pub async fn get_by_id(&self, id: i64) -> Result<Option<ProductPrice>> {
        let price: Option<ProductPrice> = sqlx::query_as(&format!(
            "{} WHERE id = ?",
            PRODUCT_PRICE_SELECT
        ))
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
                effective_date, notes, pricing_mode, input_currency,
                reference_platform, adjustment_type, adjustment_value,
                created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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
        .bind(&req.pricing_mode)
        .bind(&req.input_currency)
        .bind(&req.reference_platform)
        .bind(&req.adjustment_type)
        .bind(req.adjustment_value)
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
                pricing_mode: None,
                input_currency: None,
                reference_platform: None,
                adjustment_type: None,
                adjustment_value: None,
            };
            self.create(&req).await
        }
    }

    /// 更新或创建参考售价（带完整字段）
    pub async fn update_reference_price_full(
        &self,
        product_id: i64,
        platform: &str,
        sale_price_cny: f64,
        sale_price_usd: Option<f64>,
        exchange_rate: f64,
        profit_margin: Option<f64>,
        platform_fee_rate: Option<f64>,
        pricing_mode: Option<String>,
        input_currency: Option<String>,
        reference_platform: Option<String>,
        adjustment_type: Option<String>,
        adjustment_value: Option<f64>,
        notes: Option<String>,
    ) -> Result<ProductPrice> {
        if let Some(existing) = self.get_reference_price(product_id, platform).await? {
            let now = chrono::Utc::now().to_rfc3339();
            sqlx::query(
                r#"
                UPDATE product_prices SET
                    sale_price_cny = ?,
                    sale_price_usd = ?,
                    exchange_rate = ?,
                    profit_margin = COALESCE(?, profit_margin),
                    platform_fee_rate = COALESCE(?, platform_fee_rate),
                    pricing_mode = COALESCE(?, pricing_mode),
                    input_currency = COALESCE(?, input_currency),
                    reference_platform = ?,
                    adjustment_type = COALESCE(?, adjustment_type),
                    adjustment_value = COALESCE(?, adjustment_value),
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
            .bind(&pricing_mode)
            .bind(&input_currency)
            .bind(&reference_platform)
            .bind(&adjustment_type)
            .bind(adjustment_value)
            .bind(&notes)
            .bind(&now)
            .bind(existing.id)
            .execute(self.pool)
            .await?;

            self.get_by_id(existing.id).await?.ok_or_else(|| anyhow::anyhow!("Failed to fetch updated price"))
        } else {
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
                pricing_mode,
                input_currency,
                reference_platform,
                adjustment_type,
                adjustment_value,
            };
            self.create(&req).await
        }
    }

    pub async fn upsert_reference_price_full_tx(
        tx: &mut Transaction<'_, Sqlite>,
        data: &ReferencePriceWrite,
    ) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        let existing_id: Option<i64> = sqlx::query_scalar(
            "SELECT id FROM product_prices WHERE product_id = ? AND platform = ? AND is_reference = 1 LIMIT 1"
        )
        .bind(data.product_id)
        .bind(&data.platform)
        .fetch_optional(&mut **tx)
        .await?;

        if let Some(id) = existing_id {
            sqlx::query(
                r#"
                UPDATE product_prices SET
                    sale_price_cny = ?,
                    sale_price_usd = ?,
                    exchange_rate = ?,
                    profit_margin = ?,
                    platform_fee_rate = ?,
                    platform_fee = NULL,
                    is_reference = 1,
                    effective_date = NULL,
                    notes = ?,
                    pricing_mode = ?,
                    input_currency = ?,
                    reference_platform = ?,
                    adjustment_type = ?,
                    adjustment_value = ?,
                    updated_at = ?
                WHERE id = ?
                "#
            )
            .bind(data.sale_price_cny)
            .bind(data.sale_price_usd)
            .bind(data.exchange_rate)
            .bind(data.profit_margin.unwrap_or(0.15))
            .bind(data.platform_fee_rate.unwrap_or(0.0))
            .bind(&data.notes)
            .bind(&data.pricing_mode)
            .bind(&data.input_currency)
            .bind(&data.reference_platform)
            .bind(&data.adjustment_type)
            .bind(data.adjustment_value)
            .bind(&now)
            .bind(id)
            .execute(&mut **tx)
            .await?;
        } else {
            sqlx::query(
                r#"
                INSERT INTO product_prices (
                    product_id, platform, sale_price_cny, sale_price_usd, exchange_rate,
                    profit_margin, platform_fee_rate, platform_fee, is_reference,
                    effective_date, notes, pricing_mode, input_currency,
                    reference_platform, adjustment_type, adjustment_value,
                    created_at, updated_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, NULL, 1, NULL, ?, ?, ?, ?, ?, ?, ?, ?)
                "#
            )
            .bind(data.product_id)
            .bind(&data.platform)
            .bind(data.sale_price_cny)
            .bind(data.sale_price_usd)
            .bind(data.exchange_rate)
            .bind(data.profit_margin.unwrap_or(0.15))
            .bind(data.platform_fee_rate.unwrap_or(0.0))
            .bind(&data.notes)
            .bind(&data.pricing_mode)
            .bind(&data.input_currency)
            .bind(&data.reference_platform)
            .bind(&data.adjustment_type)
            .bind(data.adjustment_value)
            .bind(&now)
            .bind(&now)
            .execute(&mut **tx)
            .await?;
        }

        Ok(())
    }

    pub async fn delete_reference_price_tx(
        tx: &mut Transaction<'_, Sqlite>,
        product_id: i64,
        platform: &str,
    ) -> Result<()> {
        sqlx::query("DELETE FROM product_prices WHERE product_id = ? AND platform = ? AND is_reference = 1")
            .bind(product_id)
            .bind(platform)
            .execute(&mut **tx)
            .await?;
        Ok(())
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
