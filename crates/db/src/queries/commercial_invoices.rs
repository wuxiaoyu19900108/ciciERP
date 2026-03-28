//! CI (Commercial Invoice) 数据库查询

use anyhow::Result;
use sqlx::{QueryBuilder, SqlitePool};

use cicierp_models::{
    commercial_invoice::{
        CIDetail, CIListItem, CIQuery, CIStatus, CommercialInvoice, CommercialInvoiceItem,
        CreateCIFromOrderRequest, MarkPaidRequest,
    },
    common::PagedResponse,
};

pub struct CommercialInvoiceQueries<'a> {
    pool: &'a SqlitePool,
}

impl<'a> CommercialInvoiceQueries<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    /// 获取下一个 CI 编号
    async fn get_next_ci_code(&self, date: &str) -> Result<String> {
        let date_prefix = format!("CI-{}-", date.replace("-", ""));

        let max_code: Option<(String,)> = sqlx::query_as(
            "SELECT ci_code FROM commercial_invoices WHERE ci_code LIKE ? ORDER BY ci_code DESC LIMIT 1"
        )
        .bind(format!("{}%", date_prefix))
        .fetch_optional(self.pool)
        .await?;

        let next_num = match max_code {
            Some((code,)) => {
                let parts: Vec<&str> = code.split('-').collect();
                if parts.len() == 3 {
                    parts[2].parse::<u32>().unwrap_or(0) + 1
                } else {
                    1
                }
            }
            None => 1,
        };

        Ok(format!("CI-{}-{:04}", date.replace("-", ""), next_num))
    }

    /// 从订单创建 CI
    pub async fn create_from_order(&self, order_id: i64, req: &CreateCIFromOrderRequest) -> Result<CommercialInvoice> {
        let now = chrono::Utc::now().to_rfc3339();
        let ci_code = self.get_next_ci_code(&req.ci_date).await?;

        // 获取订单信息
        let order: Option<OrderInfo> = sqlx::query_as(
            r#"SELECT
                id, customer_id, customer_name, customer_email, customer_mobile,
                subtotal, discount_amount, total_amount, currency, pi_id
            FROM orders WHERE id = ?"#
        )
        .bind(order_id)
        .fetch_optional(self.pool)
        .await?;

        let order = order.ok_or_else(|| anyhow::anyhow!("Order not found"))?;

        // 获取订单明细
        let order_items: Vec<OrderItemInfo> = sqlx::query_as(
            r#"SELECT
                product_id, product_name, sku_code as model, quantity, unit_price
            FROM order_items WHERE order_id = ?"#
        )
        .bind(order_id)
        .fetch_all(self.pool)
        .await?;

        // 获取收货地址
        let address: Option<OrderAddressInfo> = sqlx::query_as(
            r#"SELECT
                receiver_phone, province, city, address
            FROM order_addresses WHERE order_id = ?"#
        )
        .bind(order_id)
        .fetch_optional(self.pool)
        .await?;

        // 开启事务
        let mut tx = self.pool.begin().await?;

        // 创建 CI
        let customer_address = address.map(|a| {
            format!("{}{}{}", a.province.unwrap_or_default(), a.city.unwrap_or_default(), a.address)
        });

        let result = sqlx::query(
            r#"
            INSERT INTO commercial_invoices (
                ci_code, sales_order_id, pi_id,
                customer_id, customer_name, customer_email, customer_phone, customer_address,
                currency, subtotal, discount, total_amount, paid_amount, status,
                ci_date, notes, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0, 1, ?, ?, ?, ?)
            "#
        )
        .bind(&ci_code)
        .bind(order_id)
        .bind(order.pi_id)
        .bind(order.customer_id)
        .bind(&order.customer_name)
        .bind(&order.customer_email)
        .bind(&order.customer_mobile)
        .bind(customer_address)
        .bind(&order.currency)
        .bind(order.subtotal)
        .bind(order.discount_amount)
        .bind(order.total_amount)
        .bind(&req.ci_date)
        .bind(&req.notes)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await?;

        let ci_id = result.last_insert_rowid();

        // 创建 CI 明细
        for (idx, item) in order_items.iter().enumerate() {
            let total_price = item.unit_price * item.quantity as f64;
            sqlx::query(
                r#"
                INSERT INTO commercial_invoice_items (
                    ci_id, product_id, product_name, model, quantity, unit_price, total_price, sort_order, created_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#
            )
            .bind(ci_id)
            .bind(item.product_id)
            .bind(&item.product_name)
            .bind(&item.model)
            .bind(item.quantity)
            .bind(item.unit_price)
            .bind(total_price)
            .bind(idx as i64)
            .bind(&now)
            .execute(&mut *tx)
            .await?;
        }

        // 更新订单的 ci_id
        sqlx::query("UPDATE orders SET ci_id = ?, updated_at = ? WHERE id = ?")
            .bind(ci_id)
            .bind(&now)
            .bind(order_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        self.get_by_id(ci_id).await?.ok_or_else(|| anyhow::anyhow!("Failed to fetch created CI"))
    }

    /// 获取 CI 列表
    pub async fn list(&self, query: &CIQuery) -> Result<PagedResponse<CIListItem>> {
        let offset = (query.page().saturating_sub(1)) * query.page_size();

        // 构建 count 查询
        let mut count_query = QueryBuilder::new("SELECT COUNT(*) FROM commercial_invoices WHERE 1=1");

        if let Some(s) = query.status {
            count_query.push(" AND status = ");
            count_query.push_bind(s);
        }
        if let Some(oid) = query.order_id {
            count_query.push(" AND sales_order_id = ");
            count_query.push_bind(oid);
        }
        if let Some(cid) = query.customer_id {
            count_query.push(" AND customer_id = ");
            count_query.push_bind(cid);
        }
        if let Some(df) = &query.date_from {
            count_query.push(" AND date(ci_date) >= date(");
            count_query.push_bind(df);
            count_query.push(")");
        }
        if let Some(dt) = &query.date_to {
            count_query.push(" AND date(ci_date) <= date(");
            count_query.push_bind(dt);
            count_query.push(")");
        }
        if let Some(kw) = &query.keyword {
            count_query.push(" AND (ci_code LIKE ");
            count_query.push_bind(format!("%{}%", kw));
            count_query.push(" OR customer_name LIKE ");
            count_query.push_bind(format!("%{}%", kw));
            count_query.push(")");
        }

        let total: (i64,) = count_query.build_query_as().fetch_one(self.pool).await?;

        // 构建 list 查询
        let mut list_query = QueryBuilder::new(
            r#"SELECT
                ci.id, ci.ci_code, ci.sales_order_id, ci.customer_name, ci.total_amount, ci.paid_amount,
                ci.currency, ci.status, ci.ci_date,
                (SELECT COUNT(*) FROM commercial_invoice_items WHERE ci_id = ci.id) as item_count,
                ci.created_at
            FROM commercial_invoices ci
            WHERE 1=1"#
        );

        if let Some(s) = query.status {
            list_query.push(" AND ci.status = ");
            list_query.push_bind(s);
        }
        if let Some(oid) = query.order_id {
            list_query.push(" AND ci.sales_order_id = ");
            list_query.push_bind(oid);
        }
        if let Some(cid) = query.customer_id {
            list_query.push(" AND ci.customer_id = ");
            list_query.push_bind(cid);
        }
        if let Some(df) = &query.date_from {
            list_query.push(" AND date(ci.ci_date) >= date(");
            list_query.push_bind(df);
            list_query.push(")");
        }
        if let Some(dt) = &query.date_to {
            list_query.push(" AND date(ci.ci_date) <= date(");
            list_query.push_bind(dt);
            list_query.push(")");
        }
        if let Some(kw) = &query.keyword {
            list_query.push(" AND (ci.ci_code LIKE ");
            list_query.push_bind(format!("%{}%", kw));
            list_query.push(" OR ci.customer_name LIKE ");
            list_query.push_bind(format!("%{}%", kw));
            list_query.push(")");
        }

        list_query.push(" ORDER BY ci.created_at DESC LIMIT ");
        list_query.push_bind(query.page_size() as i64);
        list_query.push(" OFFSET ");
        list_query.push_bind(offset as i64);

        let items: Vec<CIListItem> = list_query.build_query_as().fetch_all(self.pool).await?;

        Ok(PagedResponse::new(items, query.page(), query.page_size(), total.0 as u64))
    }

    /// 根据 ID 获取 CI
    pub async fn get_by_id(&self, id: i64) -> Result<Option<CommercialInvoice>> {
        let ci: Option<CommercialInvoice> = sqlx::query_as(
            "SELECT * FROM commercial_invoices WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await?;

        Ok(ci)
    }

    /// 根据编码获取 CI
    pub async fn get_by_code(&self, ci_code: &str) -> Result<Option<CommercialInvoice>> {
        let ci: Option<CommercialInvoice> = sqlx::query_as(
            "SELECT * FROM commercial_invoices WHERE ci_code = ?"
        )
        .bind(ci_code)
        .fetch_optional(self.pool)
        .await?;

        Ok(ci)
    }

    /// 获取 CI 详情（包含明细）
    pub async fn get_detail(&self, id: i64) -> Result<Option<CIDetail>> {
        let ci = self.get_by_id(id).await?;
        if ci.is_none() {
            return Ok(None);
        }
        let ci = ci.unwrap();

        let items: Vec<CommercialInvoiceItem> = sqlx::query_as(
            "SELECT * FROM commercial_invoice_items WHERE ci_id = ? ORDER BY sort_order, id"
        )
        .bind(id)
        .fetch_all(self.pool)
        .await?;

        Ok(Some(CIDetail { ci, items }))
    }

    /// 获取 CI 明细
    pub async fn get_items(&self, ci_id: i64) -> Result<Vec<CommercialInvoiceItem>> {
        let items: Vec<CommercialInvoiceItem> = sqlx::query_as(
            "SELECT * FROM commercial_invoice_items WHERE ci_id = ? ORDER BY sort_order, id"
        )
        .bind(ci_id)
        .fetch_all(self.pool)
        .await?;

        Ok(items)
    }

    /// 发送 CI（状态从草稿变为已发送）
    pub async fn send(&self, id: i64) -> Result<bool> {
        let now = chrono::Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE commercial_invoices SET status = 2, updated_at = ? WHERE id = ? AND status = 1"
        )
        .bind(&now)
        .bind(id)
        .execute(self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// 标记已付款
    pub async fn mark_paid(&self, id: i64, req: &MarkPaidRequest) -> Result<bool> {
        let now = chrono::Utc::now().to_rfc3339();
        let paid_at = req.paid_at.as_deref().unwrap_or(&now);

        let result = sqlx::query(
            "UPDATE commercial_invoices SET status = 3, paid_amount = ?, paid_at = ?, updated_at = ? WHERE id = ? AND status IN (1, 2)"
        )
        .bind(req.paid_amount)
        .bind(paid_at)
        .bind(&now)
        .bind(id)
        .execute(self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// 根据 order_id 获取 CI
    pub async fn get_by_order_id(&self, order_id: i64) -> Result<Option<CommercialInvoice>> {
        let ci: Option<CommercialInvoice> = sqlx::query_as(
            "SELECT * FROM commercial_invoices WHERE sales_order_id = ?"
        )
        .bind(order_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(ci)
    }
}

// 辅助结构体
#[derive(Debug, sqlx::FromRow)]
struct OrderInfo {
    id: i64,
    customer_id: Option<i64>,
    customer_name: Option<String>,
    customer_email: Option<String>,
    customer_mobile: Option<String>,
    subtotal: f64,
    discount_amount: f64,
    total_amount: f64,
    currency: String,
    pi_id: Option<i64>,
}

#[derive(Debug, sqlx::FromRow)]
struct OrderItemInfo {
    product_id: Option<i64>,
    product_name: String,
    model: Option<String>,
    quantity: i64,
    unit_price: f64,
}

#[derive(Debug, sqlx::FromRow)]
struct OrderAddressInfo {
    receiver_phone: Option<String>,
    province: Option<String>,
    city: Option<String>,
    address: String,
}
