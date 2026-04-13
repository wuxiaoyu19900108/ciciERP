//! PI (Proforma Invoice) 数据库查询

use anyhow::Result;
use sqlx::{QueryBuilder, SqlitePool};

use cicierp_models::{
    proforma_invoice::{
        CreatePIRequest, PIDetail, PIItemRequest, PIListItem, PIStatus, ProformaInvoice,
        ProformaInvoiceItem, UpdatePIRequest, PIQuery,
    },
    common::PagedResponse,
};

pub struct ProformaInvoiceQueries<'a> {
    pool: &'a SqlitePool,
}

impl<'a> ProformaInvoiceQueries<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    /// 生成 PI 编码
    fn generate_pi_code(date: &str) -> String {
        // PI-YYYYMMDD-XXXX
        let date_part = date.replace("-", "");
        format!("PI-{}-{{seq:04}}", date_part)
    }

    /// 获取下一个 PI 编号
    async fn get_next_pi_code(&self, date: &str) -> Result<String> {
        let date_prefix = format!("PI-{}-", date.replace("-", ""));

        let max_code: Option<(String,)> = sqlx::query_as(
            "SELECT pi_code FROM proforma_invoices WHERE pi_code LIKE ? ORDER BY pi_code DESC LIMIT 1"
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

        Ok(format!("PI-{}-{:04}", date.replace("-", ""), next_num))
    }

    /// 创建 PI
    pub async fn create(&self, req: &CreatePIRequest) -> Result<ProformaInvoice> {
        let now = chrono::Utc::now().to_rfc3339();
        let pi_code = self.get_next_pi_code(&req.pi_date).await?;

        // 计算金额
        let subtotal: f64 = req.items.iter().map(|i| i.unit_price * i.quantity as f64).sum();
        let discount = req.discount.unwrap_or(0.0);
        let total_amount = subtotal - discount;

        // 开启事务
        let mut tx = self.pool.begin().await?;

        // 创建 PI
        let result = sqlx::query(
            r#"
            INSERT INTO proforma_invoices (
                pi_code, customer_id, customer_name, customer_email, customer_phone, customer_address,
                seller_name, seller_address, seller_phone, seller_email,
                currency, subtotal, discount, total_amount, exchange_rate, status,
                pi_date, valid_until, payment_terms, delivery_terms, lead_time, notes,
                created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&pi_code)
        .bind(req.customer_id)
        .bind(&req.customer_name)
        .bind(&req.customer_email)
        .bind(&req.customer_phone)
        .bind(&req.customer_address)
        .bind(req.seller_name.as_deref().unwrap_or("Shenzhen Westway Technology Co., Ltd"))
        .bind(&req.seller_address)
        .bind(&req.seller_phone)
        .bind(&req.seller_email)
        .bind(req.currency.as_deref().unwrap_or("USD"))
        .bind(subtotal)
        .bind(discount)
        .bind(total_amount)
        .bind(req.exchange_rate)
        .bind(&req.pi_date)
        .bind(&req.valid_until)
        .bind(req.payment_terms.as_deref().unwrap_or("100% before shipment"))
        .bind(req.delivery_terms.as_deref().unwrap_or("EXW"))
        .bind(req.lead_time.as_deref().unwrap_or("3-7 working days"))
        .bind(&req.notes)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await?;

        let pi_id = result.last_insert_rowid();

        // 创建 PI 明细
        for (idx, item) in req.items.iter().enumerate() {
            let total_price = item.unit_price * item.quantity as f64;
            sqlx::query(
                r#"
                INSERT INTO proforma_invoice_items (
                    pi_id, product_id, product_name, model, quantity, unit_price, total_price, notes, sort_order, created_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#
            )
            .bind(pi_id)
            .bind(item.product_id)
            .bind(&item.product_name)
            .bind(&item.model)
            .bind(item.quantity)
            .bind(item.unit_price)
            .bind(total_price)
            .bind(&item.notes)
            .bind(item.sort_order.unwrap_or(idx as i64))
            .bind(&now)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        self.get_by_id(pi_id).await?.ok_or_else(|| anyhow::anyhow!("Failed to fetch created PI"))
    }

    /// 获取 PI 列表
    pub async fn list(&self, query: &PIQuery) -> Result<PagedResponse<PIListItem>> {
        let offset = (query.page().saturating_sub(1)) * query.page_size();

        // 构建 count 查询
        let mut count_query = QueryBuilder::new("SELECT COUNT(*) FROM proforma_invoices WHERE 1=1");

        if let Some(s) = query.status {
            count_query.push(" AND status = ");
            count_query.push_bind(s);
        }
        if let Some(cid) = query.customer_id {
            count_query.push(" AND customer_id = ");
            count_query.push_bind(cid);
        }
        if let Some(df) = &query.date_from {
            count_query.push(" AND date(pi_date) >= date(");
            count_query.push_bind(df);
            count_query.push(")");
        }
        if let Some(dt) = &query.date_to {
            count_query.push(" AND date(pi_date) <= date(");
            count_query.push_bind(dt);
            count_query.push(")");
        }
        if let Some(kw) = &query.keyword {
            count_query.push(" AND (pi_code LIKE ");
            count_query.push_bind(format!("%{}%", kw));
            count_query.push(" OR customer_name LIKE ");
            count_query.push_bind(format!("%{}%", kw));
            count_query.push(")");
        }

        let total: (i64,) = count_query.build_query_as().fetch_one(self.pool).await?;

        // 构建 list 查询
        let mut list_query = QueryBuilder::new(
            r#"SELECT
                pi.id, pi.pi_code, pi.customer_name, pi.total_amount, pi.currency, pi.status, pi.pi_date,
                (SELECT COUNT(*) FROM proforma_invoice_items WHERE pi_id = pi.id) as item_count,
                pi.created_at
            FROM proforma_invoices pi
            WHERE 1=1"#
        );

        if let Some(s) = query.status {
            list_query.push(" AND pi.status = ");
            list_query.push_bind(s);
        }
        if let Some(cid) = query.customer_id {
            list_query.push(" AND pi.customer_id = ");
            list_query.push_bind(cid);
        }
        if let Some(df) = &query.date_from {
            list_query.push(" AND date(pi.pi_date) >= date(");
            list_query.push_bind(df);
            list_query.push(")");
        }
        if let Some(dt) = &query.date_to {
            list_query.push(" AND date(pi.pi_date) <= date(");
            list_query.push_bind(dt);
            list_query.push(")");
        }
        if let Some(kw) = &query.keyword {
            list_query.push(" AND (pi.pi_code LIKE ");
            list_query.push_bind(format!("%{}%", kw));
            list_query.push(" OR pi.customer_name LIKE ");
            list_query.push_bind(format!("%{}%", kw));
            list_query.push(")");
        }

        list_query.push(" ORDER BY pi.created_at DESC LIMIT ");
        list_query.push_bind(query.page_size() as i64);
        list_query.push(" OFFSET ");
        list_query.push_bind(offset as i64);

        let items: Vec<PIListItem> = list_query.build_query_as().fetch_all(self.pool).await?;

        Ok(PagedResponse::new(items, query.page(), query.page_size(), total.0 as u64))
    }

    /// 根据 ID 获取 PI
    pub async fn get_by_id(&self, id: i64) -> Result<Option<ProformaInvoice>> {
        let pi: Option<ProformaInvoice> = sqlx::query_as(
            "SELECT * FROM proforma_invoices WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await?;

        Ok(pi)
    }

    /// 根据编码获取 PI
    pub async fn get_by_code(&self, pi_code: &str) -> Result<Option<ProformaInvoice>> {
        let pi: Option<ProformaInvoice> = sqlx::query_as(
            "SELECT * FROM proforma_invoices WHERE pi_code = ?"
        )
        .bind(pi_code)
        .fetch_optional(self.pool)
        .await?;

        Ok(pi)
    }

    /// 获取 PI 详情（包含明细）
    pub async fn get_detail(&self, id: i64) -> Result<Option<PIDetail>> {
        let pi = self.get_by_id(id).await?;
        if pi.is_none() {
            return Ok(None);
        }
        let pi = pi.unwrap();

        let items: Vec<ProformaInvoiceItem> = sqlx::query_as(
            "SELECT * FROM proforma_invoice_items WHERE pi_id = ? ORDER BY sort_order, id"
        )
        .bind(id)
        .fetch_all(self.pool)
        .await?;

        Ok(Some(PIDetail { pi, items }))
    }

    /// 获取 PI 明细
    pub async fn get_items(&self, pi_id: i64) -> Result<Vec<ProformaInvoiceItem>> {
        let items: Vec<ProformaInvoiceItem> = sqlx::query_as(
            "SELECT * FROM proforma_invoice_items WHERE pi_id = ? ORDER BY sort_order, id"
        )
        .bind(pi_id)
        .fetch_all(self.pool)
        .await?;

        Ok(items)
    }

    /// 更新 PI
    pub async fn update(&self, id: i64, req: &UpdatePIRequest) -> Result<Option<ProformaInvoice>> {
        if self.get_by_id(id).await?.is_none() {
            return Ok(None);
        }

        let now = chrono::Utc::now().to_rfc3339();

        // 开启事务
        let mut tx = self.pool.begin().await?;

        // 更新 PI 主表
        sqlx::query(
            r#"
            UPDATE proforma_invoices SET
                customer_id = COALESCE(?, customer_id),
                customer_name = COALESCE(?, customer_name),
                customer_email = COALESCE(?, customer_email),
                customer_phone = COALESCE(?, customer_phone),
                customer_address = COALESCE(?, customer_address),
                seller_name = COALESCE(?, seller_name),
                seller_address = COALESCE(?, seller_address),
                seller_phone = COALESCE(?, seller_phone),
                seller_email = COALESCE(?, seller_email),
                currency = COALESCE(?, currency),
                discount = COALESCE(?, discount),
                pi_date = COALESCE(?, pi_date),
                valid_until = COALESCE(?, valid_until),
                payment_terms = COALESCE(?, payment_terms),
                delivery_terms = COALESCE(?, delivery_terms),
                lead_time = COALESCE(?, lead_time),
                notes = COALESCE(?, notes),
                updated_at = ?
            WHERE id = ?
            "#
        )
        .bind(req.customer_id)
        .bind(&req.customer_name)
        .bind(&req.customer_email)
        .bind(&req.customer_phone)
        .bind(&req.customer_address)
        .bind(&req.seller_name)
        .bind(&req.seller_address)
        .bind(&req.seller_phone)
        .bind(&req.seller_email)
        .bind(&req.currency)
        .bind(req.discount)
        .bind(&req.pi_date)
        .bind(&req.valid_until)
        .bind(&req.payment_terms)
        .bind(&req.delivery_terms)
        .bind(&req.lead_time)
        .bind(&req.notes)
        .bind(&now)
        .bind(id)
        .execute(&mut *tx)
        .await?;

        // 如果有明细更新，则删除旧明细，插入新明细
        if let Some(ref items) = req.items {
            sqlx::query("DELETE FROM proforma_invoice_items WHERE pi_id = ?")
                .bind(id)
                .execute(&mut *tx)
                .await?;

            let subtotal: f64 = items.iter().map(|i| i.unit_price * i.quantity as f64).sum();
            let discount = req.discount.unwrap_or(0.0);
            let total_amount = subtotal - discount;

            // 更新金额
            sqlx::query(
                "UPDATE proforma_invoices SET subtotal = ?, total_amount = ?, updated_at = ? WHERE id = ?"
            )
            .bind(subtotal)
            .bind(total_amount)
            .bind(&now)
            .bind(id)
            .execute(&mut *tx)
            .await?;

            // 插入新明细
            for (idx, item) in items.iter().enumerate() {
                let total_price = item.unit_price * item.quantity as f64;
                sqlx::query(
                    r#"
                    INSERT INTO proforma_invoice_items (
                        pi_id, product_id, product_name, model, quantity, unit_price, total_price, notes, sort_order, created_at
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    "#
                )
                .bind(id)
                .bind(item.product_id)
                .bind(&item.product_name)
                .bind(&item.model)
                .bind(item.quantity)
                .bind(item.unit_price)
                .bind(total_price)
                .bind(&item.notes)
                .bind(item.sort_order.unwrap_or(idx as i64))
                .bind(&now)
                .execute(&mut *tx)
                .await?;
            }
        }

        tx.commit().await?;

        self.get_by_id(id).await
    }

    /// 更新 PI 状态
    pub async fn update_status(&self, id: i64, status: PIStatus) -> Result<bool> {
        let now = chrono::Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE proforma_invoices SET status = ?, updated_at = ? WHERE id = ?"
        )
        .bind(status as i64)
        .bind(&now)
        .bind(id)
        .execute(self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// 发送 PI（状态从草稿变为已发送）
    pub async fn send(&self, id: i64) -> Result<bool> {
        let now = chrono::Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE proforma_invoices SET status = 2, updated_at = ? WHERE id = ? AND status = 1"
        )
        .bind(&now)
        .bind(id)
        .execute(self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// 确认 PI（状态从已发送变为已确认）
    pub async fn confirm(&self, id: i64) -> Result<bool> {
        let now = chrono::Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE proforma_invoices SET status = 3, confirmed_at = ?, updated_at = ? WHERE id = ? AND status = 2"
        )
        .bind(&now)
        .bind(&now)
        .bind(id)
        .execute(self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// 将 PI 转为订单（状态从已确认变为已转订单）
    pub async fn mark_converted(&self, id: i64, order_id: i64) -> Result<bool> {
        let now = chrono::Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE proforma_invoices SET status = 4, sales_order_id = ?, converted_at = ?, updated_at = ? WHERE id = ? AND status = 3"
        )
        .bind(order_id)
        .bind(&now)
        .bind(&now)
        .bind(id)
        .execute(self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// 取消 PI
    pub async fn cancel(&self, id: i64) -> Result<bool> {
        let now = chrono::Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE proforma_invoices SET status = 5, updated_at = ? WHERE id = ? AND status IN (1, 2)"
        )
        .bind(&now)
        .bind(id)
        .execute(self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// 删除 PI（仅草稿状态可删除）
    pub async fn delete(&self, id: i64) -> Result<bool> {
        let result = sqlx::query(
            "DELETE FROM proforma_invoices WHERE id = ? AND status = 1"
        )
        .bind(id)
        .execute(self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }
}
