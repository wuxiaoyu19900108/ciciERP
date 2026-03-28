//! 产品成本相关数据库查询

use anyhow::Result;
use sqlx::SqlitePool;

use cicierp_models::product::{
    CreateProductCostRequest, ProductCost, UpdateProductCostRequest,
};

/// 产品成本查询结构体
pub struct ProductCostQueries<'a> {
    pool: &'a SqlitePool,
}

impl<'a> ProductCostQueries<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    /// 根据产品ID获取参考成本记录
    pub async fn get_reference_cost(&self, product_id: i64) -> Result<Option<ProductCost>> {
        let cost: Option<ProductCost> = sqlx::query_as(
            "SELECT * FROM product_costs WHERE product_id = ? AND is_reference = 1 LIMIT 1"
        )
        .bind(product_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(cost)
    }

    /// 根据产品ID获取最新成本记录
    pub async fn get_by_product_id(&self, product_id: i64) -> Result<Option<ProductCost>> {
        let cost: Option<ProductCost> = sqlx::query_as(
            "SELECT * FROM product_costs WHERE product_id = ? ORDER BY created_at DESC LIMIT 1"
        )
        .bind(product_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(cost)
    }

    /// 根据ID获取成本记录
    pub async fn get_by_id(&self, id: i64) -> Result<Option<ProductCost>> {
        let cost: Option<ProductCost> = sqlx::query_as(
            "SELECT * FROM product_costs WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await?;

        Ok(cost)
    }

    /// 获取产品的所有成本历史
    pub async fn list_by_product(&self, product_id: i64) -> Result<Vec<ProductCost>> {
        let costs: Vec<ProductCost> = sqlx::query_as(
            "SELECT * FROM product_costs WHERE product_id = ? ORDER BY created_at DESC"
        )
        .bind(product_id)
        .fetch_all(self.pool)
        .await?;

        Ok(costs)
    }

    /// 计算产品的平均成本（基于采购数量加权）
    pub async fn get_average_cost(&self, product_id: i64) -> Result<Option<f64>> {
        let result: Option<(f64,)> = sqlx::query_as(
            r#"
            SELECT SUM(cost_cny * quantity) / SUM(quantity) as avg_cost
            FROM product_costs
            WHERE product_id = ? AND quantity > 0
            "#
        )
        .bind(product_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(result.map(|r| r.0))
    }

    /// 创建成本记录
    pub async fn create(&self, req: &CreateProductCostRequest) -> Result<ProductCost> {
        let now = chrono::Utc::now().to_rfc3339();
        let currency = req.currency.clone().unwrap_or_else(|| "CNY".to_string());
        let exchange_rate = req.exchange_rate.unwrap_or(6.81);
        let profit_margin = req.profit_margin.unwrap_or(0.0);
        let platform_fee_rate = req.platform_fee_rate.unwrap_or(0.025);
        let quantity = req.quantity.unwrap_or(1);
        let is_reference = req.is_reference.unwrap_or(false) as i64;

        let result = sqlx::query(
            r#"
            INSERT INTO product_costs (
                product_id, supplier_id, cost_cny, cost_usd, currency,
                exchange_rate, profit_margin, platform_fee_rate, platform_fee,
                sale_price_usd, quantity, purchase_order_id, is_reference,
                effective_date, notes, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(req.product_id)
        .bind(req.supplier_id)
        .bind(req.cost_cny)
        .bind(req.cost_usd)
        .bind(&currency)
        .bind(exchange_rate)
        .bind(profit_margin)
        .bind(platform_fee_rate)
        .bind(req.platform_fee)
        .bind(req.sale_price_usd)
        .bind(quantity)
        .bind(req.purchase_order_id)
        .bind(is_reference)
        .bind(&req.effective_date)
        .bind(&req.notes)
        .bind(&now)
        .bind(&now)
        .execute(self.pool)
        .await?;

        let id = result.last_insert_rowid();
        self.get_by_id(id).await?.ok_or_else(|| anyhow::anyhow!("Failed to fetch created product cost"))
    }

    /// 更新成本记录
    pub async fn update(&self, id: i64, req: &UpdateProductCostRequest) -> Result<Option<ProductCost>> {
        if self.get_by_id(id).await?.is_none() {
            return Ok(None);
        }

        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            UPDATE product_costs SET
                supplier_id = COALESCE(?, supplier_id),
                cost_cny = COALESCE(?, cost_cny),
                cost_usd = COALESCE(?, cost_usd),
                currency = COALESCE(?, currency),
                exchange_rate = COALESCE(?, exchange_rate),
                profit_margin = COALESCE(?, profit_margin),
                platform_fee_rate = COALESCE(?, platform_fee_rate),
                platform_fee = COALESCE(?, platform_fee),
                sale_price_usd = COALESCE(?, sale_price_usd),
                quantity = COALESCE(?, quantity),
                purchase_order_id = COALESCE(?, purchase_order_id),
                is_reference = COALESCE(?, is_reference),
                effective_date = COALESCE(?, effective_date),
                notes = COALESCE(?, notes),
                updated_at = ?
            WHERE id = ?
            "#
        )
        .bind(req.supplier_id)
        .bind(req.cost_cny)
        .bind(req.cost_usd)
        .bind(req.currency.as_ref())
        .bind(req.exchange_rate)
        .bind(req.profit_margin)
        .bind(req.platform_fee_rate)
        .bind(req.platform_fee)
        .bind(req.sale_price_usd)
        .bind(req.quantity)
        .bind(req.purchase_order_id)
        .bind(req.is_reference)
        .bind(req.effective_date.as_ref())
        .bind(req.notes.as_ref())
        .bind(&now)
        .bind(id)
        .execute(self.pool)
        .await?;

        self.get_by_id(id).await
    }

    /// 删除成本记录
    pub async fn delete(&self, id: i64) -> Result<bool> {
        let result = sqlx::query("DELETE FROM product_costs WHERE id = ?")
            .bind(id)
            .execute(self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// 删除产品的所有成本记录
    pub async fn delete_by_product(&self, product_id: i64) -> Result<u64> {
        let result = sqlx::query("DELETE FROM product_costs WHERE product_id = ?")
            .bind(product_id)
            .execute(self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    /// 更新或创建参考成本
    pub async fn update_reference_cost(
        &self,
        product_id: i64,
        cost_cny: f64,
        cost_usd: Option<f64>,
        exchange_rate: f64,
        notes: Option<String>,
    ) -> Result<ProductCost> {
        // 先尝试获取现有的参考成本
        if let Some(existing) = self.get_reference_cost(product_id).await? {
            // 更新现有记录
            let now = chrono::Utc::now().to_rfc3339();
            sqlx::query(
                r#"
                UPDATE product_costs SET
                    cost_cny = ?,
                    cost_usd = ?,
                    exchange_rate = ?,
                    notes = ?,
                    updated_at = ?
                WHERE id = ?
                "#
            )
            .bind(cost_cny)
            .bind(cost_usd)
            .bind(exchange_rate)
            .bind(&notes)
            .bind(&now)
            .bind(existing.id)
            .execute(self.pool)
            .await?;

            self.get_by_id(existing.id).await?.ok_or_else(|| anyhow::anyhow!("Failed to fetch updated cost"))
        } else {
            // 创建新记录
            let req = CreateProductCostRequest {
                product_id,
                supplier_id: None,
                cost_cny,
                cost_usd,
                currency: Some("CNY".to_string()),
                exchange_rate: Some(exchange_rate),
                profit_margin: Some(0.15),
                platform_fee_rate: Some(0.025),
                platform_fee: None,
                sale_price_usd: None,
                quantity: Some(1),
                purchase_order_id: None,
                is_reference: Some(true),
                effective_date: None,
                notes,
            };
            self.create(&req).await
        }
    }
}
