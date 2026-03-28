//! 订单相关数据库查询

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{QueryBuilder, SqlitePool};

use cicierp_models::{
    order::{CreateOrderRequest, Order, OrderAddress, OrderDetail, OrderItem, OrderListItem, ShipOrderRequest, UpdateOrderRequest},
    common::PagedResponse,
};
use super::inventory::InventoryQueries;

pub struct OrderQueries<'a> {
    pool: &'a SqlitePool,
}

impl<'a> OrderQueries<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    /// 获取订单列表（使用安全的参数化查询）
    pub async fn list(
        &self,
        page: u32,
        page_size: u32,
        order_status: Option<i64>,
        payment_status: Option<i64>,
        customer_id: Option<i64>,
        platform: Option<&str>,
        date_from: Option<&str>,
        date_to: Option<&str>,
        keyword: Option<&str>,
    ) -> Result<PagedResponse<OrderListItem>> {
        let offset = (page.saturating_sub(1)) * page_size;

        // 构建安全的 count 查询
        let mut count_query = QueryBuilder::new("SELECT COUNT(*) FROM orders o WHERE 1=1");

        if let Some(os) = order_status {
            count_query.push(" AND o.order_status = ");
            count_query.push_bind(os);
        }
        if let Some(ps) = payment_status {
            count_query.push(" AND o.payment_status = ");
            count_query.push_bind(ps);
        }
        if let Some(cid) = customer_id {
            count_query.push(" AND o.customer_id = ");
            count_query.push_bind(cid);
        }
        if let Some(p) = platform {
            count_query.push(" AND o.platform = ");
            count_query.push_bind(p);
        }
        if let Some(df) = date_from {
            count_query.push(" AND date(o.created_at) >= date(");
            count_query.push_bind(df);
            count_query.push(")");
        }
        if let Some(dt) = date_to {
            count_query.push(" AND date(o.created_at) <= date(");
            count_query.push_bind(dt);
            count_query.push(")");
        }
        if let Some(kw) = keyword {
            count_query.push(" AND (o.order_code LIKE ");
            count_query.push_bind(format!("%{}%", kw));
            count_query.push(" OR o.customer_name LIKE ");
            count_query.push_bind(format!("%{}%", kw));
            count_query.push(" OR o.customer_mobile LIKE ");
            count_query.push_bind(format!("%{}%", kw));
            count_query.push(")");
        }

        let total: (i64,) = count_query.build_query_as()
            .fetch_one(self.pool)
            .await?;

        // 构建安全的 list 查询
        let mut list_query = QueryBuilder::new(
            r#"SELECT
                o.id, o.order_code, o.customer_name, o.total_amount,
                o.order_status, o.payment_status, o.fulfillment_status,
                o.created_at,
                (SELECT COUNT(*) FROM order_items WHERE order_id = o.id) as item_count
            FROM orders o
            WHERE 1=1"#
        );

        if let Some(os) = order_status {
            list_query.push(" AND o.order_status = ");
            list_query.push_bind(os);
        }
        if let Some(ps) = payment_status {
            list_query.push(" AND o.payment_status = ");
            list_query.push_bind(ps);
        }
        if let Some(cid) = customer_id {
            list_query.push(" AND o.customer_id = ");
            list_query.push_bind(cid);
        }
        if let Some(p) = platform {
            list_query.push(" AND o.platform = ");
            list_query.push_bind(p);
        }
        if let Some(df) = date_from {
            list_query.push(" AND date(o.created_at) >= date(");
            list_query.push_bind(df);
            list_query.push(")");
        }
        if let Some(dt) = date_to {
            list_query.push(" AND date(o.created_at) <= date(");
            list_query.push_bind(dt);
            list_query.push(")");
        }
        if let Some(kw) = keyword {
            list_query.push(" AND (o.order_code LIKE ");
            list_query.push_bind(format!("%{}%", kw));
            list_query.push(" OR o.customer_name LIKE ");
            list_query.push_bind(format!("%{}%", kw));
            list_query.push(" OR o.customer_mobile LIKE ");
            list_query.push_bind(format!("%{}%", kw));
            list_query.push(")");
        }

        list_query.push(" ORDER BY o.created_at DESC LIMIT ");
        list_query.push_bind(page_size as i64);
        list_query.push(" OFFSET ");
        list_query.push_bind(offset as i64);

        let items: Vec<OrderListItem> = list_query.build_query_as()
            .fetch_all(self.pool)
            .await?;

        Ok(PagedResponse::new(items, page, page_size, total.0 as u64))
    }

    /// 根据 ID 获取订单
    pub async fn get_by_id(&self, id: i64) -> Result<Option<Order>> {
        let order: Option<Order> = sqlx::query_as(
            "SELECT * FROM orders WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await?;

        Ok(order)
    }

    /// 获取订单详情（包含明细和地址）
    pub async fn get_detail(&self, id: i64) -> Result<Option<OrderDetail>> {
        let order = self.get_by_id(id).await?;
        if order.is_none() {
            return Ok(None);
        }
        let order = order.unwrap();

        let items: Vec<OrderItem> = sqlx::query_as(
            "SELECT * FROM order_items WHERE order_id = ? ORDER BY id"
        )
        .bind(id)
        .fetch_all(self.pool)
        .await?;

        let address: Option<OrderAddress> = sqlx::query_as(
            "SELECT * FROM order_addresses WHERE order_id = ?"
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await?;

        Ok(Some(OrderDetail {
            order,
            items,
            address,
        }))
    }

    /// 创建订单（含库存锁定）
    pub async fn create(&self, req: &CreateOrderRequest) -> Result<Order> {
        let now = chrono::Utc::now().to_rfc3339();
        let order_code = self.generate_order_code().await?;

        // 计算金额
        let subtotal: f64 = req.items.iter().map(|i| i.unit_price * i.quantity as f64).sum();
        let shipping_fee = req.shipping_fee.unwrap_or(0.0);
        let discount_amount = req.discount_amount.unwrap_or(0.0);
        let total_amount = subtotal + shipping_fee - discount_amount;

        // 条款默认值
        let payment_terms = req.payment_terms.clone().unwrap_or_else(|| "100% before shipment".to_string());
        let delivery_terms = req.delivery_terms.clone().unwrap_or_else(|| "EXW".to_string());
        let lead_time = req.lead_time.clone().unwrap_or_else(|| "3-7 working days".to_string());

        // 开启事务
        let mut tx = self.pool.begin().await?;

        // 创建订单
        let result = sqlx::query(
            r#"
            INSERT INTO orders (
                order_code, platform, platform_order_id, customer_id, customer_name,
                customer_mobile, customer_email, order_type, order_status, payment_status,
                fulfillment_status, total_amount, subtotal, discount_amount, shipping_fee,
                customer_note, payment_terms, delivery_terms, lead_time, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, 1, 1, 1, 1, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&order_code)
        .bind(&req.platform)
        .bind(&req.platform_order_id)
        .bind(req.customer_id)
        .bind(&req.customer_name)
        .bind(&req.customer_mobile)
        .bind(&req.customer_email)
        .bind(total_amount)
        .bind(subtotal)
        .bind(discount_amount)
        .bind(shipping_fee)
        .bind(&req.customer_note)
        .bind(&payment_terms)
        .bind(&delivery_terms)
        .bind(&lead_time)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await?;

        let order_id = result.last_insert_rowid();

        // 创建订单明细并锁定库存
        let inventory_queries = InventoryQueries::new(self.pool);
        for item in &req.items {
            let item_subtotal = item.unit_price * item.quantity as f64;
            sqlx::query(
                r#"
                INSERT INTO order_items (
                    order_id, product_id, sku_id, product_name, product_code,
                    sku_code, sku_spec, product_image, quantity, unit_price,
                    subtotal, total_amount, created_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#
            )
            .bind(order_id)
            .bind(item.product_id)
            .bind(item.sku_id)
            .bind(&item.product_name)
            .bind(&item.product_code)
            .bind(&item.sku_code)
            .bind(&item.sku_spec)
            .bind(&item.product_image)
            .bind(item.quantity)
            .bind(item.unit_price)
            .bind(item_subtotal)
            .bind(item_subtotal)
            .bind(&now)
            .execute(&mut *tx)
            .await?;

            // 锁定库存（如果有 SKU ID）
            if let Some(sku_id) = item.sku_id {
                let locked = inventory_queries.lock(sku_id, item.quantity as i64, Some(order_id)).await?;
                if !locked {
                    return Err(anyhow::anyhow!(
                        "Insufficient inventory for SKU {} (quantity: {})",
                        sku_id, item.quantity
                    ));
                }
            }
        }

        // 创建收货地址
        sqlx::query(
            r#"
            INSERT INTO order_addresses (
                order_id, receiver_name, receiver_phone, country, province,
                city, district, address, postal_code, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(order_id)
        .bind(&req.receiver_name)
        .bind(&req.receiver_phone)
        .bind(&req.country)
        .bind(&req.province)
        .bind(&req.city)
        .bind(&req.district)
        .bind(&req.address)
        .bind(&req.postal_code)
        .bind(&now)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        self.get_by_id(order_id).await?.ok_or_else(|| anyhow::anyhow!("Failed to fetch created order"))
    }

    /// 生成订单编号 ORD-YYYYMMDD-XXXX
    async fn generate_order_code(&self) -> Result<String> {
        let today = chrono::Utc::now().format("%Y%m%d").to_string();
        let prefix = format!("ORD-{}-", today);

        // 查询今天已有的订单数量
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM orders WHERE order_code LIKE ?"
        )
        .bind(format!("{}%", prefix))
        .fetch_one(self.pool)
        .await?;

        let seq = count.0 + 1;
        Ok(format!("{}{:04}", prefix, seq))
    }

    /// 更新订单
    pub async fn update(&self, id: i64, req: &UpdateOrderRequest) -> Result<Option<Order>> {
        if self.get_by_id(id).await?.is_none() {
            return Ok(None);
        }

        let now = chrono::Utc::now().to_rfc3339();
        let mut updates = vec!["updated_at = ?"];
        let mut bindings: Vec<String> = vec![now.clone()];

        if let Some(ref note) = req.internal_note {
            updates.push("internal_note = ?");
            bindings.push(note.clone());
        }
        if let Some(s) = req.order_status {
            updates.push("order_status = ?");
            bindings.push(s.to_string());
        }

        let sql = format!(
            "UPDATE orders SET {} WHERE id = ?",
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

    /// 取消订单（含库存解锁）
    pub async fn cancel(&self, id: i64, reason: &str) -> Result<bool> {
        // 获取订单明细中的 SKU 信息
        let items: Vec<OrderItem> = sqlx::query_as(
            "SELECT * FROM order_items WHERE order_id = ?"
        )
        .bind(id)
        .fetch_all(self.pool)
        .await?;

        let now = chrono::Utc::now().to_rfc3339();
        let result = sqlx::query(
            r#"
            UPDATE orders SET
                order_status = 6,
                cancel_reason = ?,
                cancel_time = ?,
                updated_at = ?
            WHERE id = ? AND order_status IN (1, 2)
            "#
        )
        .bind(reason)
        .bind(&now)
        .bind(&now)
        .bind(id)
        .execute(self.pool)
        .await?;

        if result.rows_affected() > 0 {
            // 解锁库存
            let inventory_queries = InventoryQueries::new(self.pool);
            for item in items {
                if let Some(sku_id) = item.sku_id {
                    let _ = inventory_queries.unlock(sku_id, item.quantity as i64, Some(id)).await;
                }
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// 订单发货（含物流信息保存）
    pub async fn ship(&self, id: i64, req: &ShipOrderRequest) -> Result<bool> {
        let now = chrono::Utc::now().to_rfc3339();

        // 获取订单收货地址信息
        let address = self.get_address(id).await?;

        // 开启事务
        let mut tx = self.pool.begin().await?;

        // 更新订单状态
        let result = sqlx::query(
            r#"
            UPDATE orders SET
                order_status = CASE WHEN order_status = 2 THEN 4 ELSE order_status END,
                fulfillment_status = 3,
                ship_time = ?,
                updated_at = ?
            WHERE id = ? AND order_status IN (2, 3)
            "#
        )
        .bind(&now)
        .bind(&now)
        .bind(id)
        .execute(&mut *tx)
        .await?;

        if result.rows_affected() == 0 {
            return Ok(false);
        }

        // 创建发货单
        let shipment_code = format!("SH{}", chrono::Utc::now().format("%Y%m%d%H%M%S"));

        let (receiver_name, receiver_phone, receiver_address) = match &address {
            Some(addr) => (
                addr.receiver_name.clone(),
                addr.receiver_phone.clone(),
                format!("{}{}{}{}",
                    addr.province.as_deref().unwrap_or(""),
                    addr.city.as_deref().unwrap_or(""),
                    addr.district.as_deref().unwrap_or(""),
                    addr.address
                )
            ),
            None => ("未知".to_string(), "".to_string(), "未知地址".to_string()),
        };

        sqlx::query(
            r#"
            INSERT INTO shipments (
                shipment_code, order_id, logistics_id, logistics_name,
                tracking_number, receiver_name, receiver_phone, receiver_address,
                package_items, package_count, shipping_fee, shipping_note,
                status, ship_time, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, '[]', 1, 0, ?, 1, ?, ?, ?)
            "#
        )
        .bind(&shipment_code)
        .bind(id)
        .bind(req.logistics_id)
        .bind(&req.logistics_name)
        .bind(&req.tracking_number)
        .bind(&receiver_name)
        .bind(&receiver_phone)
        .bind(&receiver_address)
        .bind(&req.shipping_note)
        .bind(&now)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(true)
    }

    /// 获取订单总数
    pub async fn count(&self) -> sqlx::Result<i64> {
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM orders WHERE deleted_at IS NULL"
        )
        .fetch_one(self.pool)
        .await?;
        Ok(count.0)
    }

    /// 获取订单收货地址
    pub async fn get_address(&self, order_id: i64) -> sqlx::Result<Option<OrderAddress>> {
        sqlx::query_as(
            "SELECT id, order_id, receiver_name, receiver_phone, country, country_code,
                    province, city, district, address, postal_code, address_type, created_at
             FROM order_addresses WHERE order_id = ?"
        )
        .bind(order_id)
        .fetch_optional(self.pool)
        .await
    }

    /// 根据平台订单号获取订单
    pub async fn get_by_platform_order_id(&self, platform_order_id: &str) -> Result<Option<Order>> {
        let order: Option<Order> = sqlx::query_as(
            "SELECT * FROM orders WHERE platform_order_id = ?"
        )
        .bind(platform_order_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(order)
    }

    /// 更新订单详情（仅未成交状态）
    pub async fn update_order_detail(
        &self,
        id: i64,
        customer_name: &Option<String>,
        customer_mobile: &Option<String>,
        customer_email: &Option<String>,
        receiver_name: &str,
        receiver_phone: &str,
        country: &str,
        address: &str,
        shipping_fee: f64,
        discount_amount: f64,
        customer_note: Option<&str>,
        payment_terms: Option<&str>,
        delivery_terms: Option<&str>,
        lead_time: Option<&str>,
    ) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();

        // 开启事务
        let mut tx = self.pool.begin().await?;

        // 更新订单主表
        sqlx::query(
            r#"
            UPDATE orders SET
                customer_name = ?,
                customer_mobile = ?,
                customer_email = ?,
                shipping_fee = ?,
                discount_amount = ?,
                customer_note = ?,
                payment_terms = ?,
                delivery_terms = ?,
                lead_time = ?,
                updated_at = ?
            WHERE id = ? AND order_status = 1
            "#
        )
        .bind(customer_name)
        .bind(customer_mobile)
        .bind(customer_email)
        .bind(shipping_fee)
        .bind(discount_amount)
        .bind(customer_note)
        .bind(payment_terms)
        .bind(delivery_terms)
        .bind(lead_time)
        .bind(&now)
        .bind(id)
        .execute(&mut *tx)
        .await?;

        // 更新收货地址
        sqlx::query(
            r#"
            UPDATE order_addresses SET
                receiver_name = ?,
                receiver_phone = ?,
                country = ?,
                address = ?
            WHERE order_id = ?
            "#
        )
        .bind(receiver_name)
        .bind(receiver_phone)
        .bind(country)
        .bind(address)
        .bind(id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    /// 获取产品历史成交价格
    pub async fn get_product_history_prices(&self, product_id: i64, limit: u32) -> Result<Vec<ProductHistoryPrice>> {
        let prices: Vec<ProductHistoryPrice> = sqlx::query_as(
            r#"
            SELECT
                oi.product_id,
                oi.product_name,
                oi.unit_price,
                oi.quantity,
                o.order_code,
                o.customer_name,
                o.created_at as order_date
            FROM order_items oi
            JOIN orders o ON oi.order_id = o.id
            WHERE oi.product_id = ? AND o.order_status IN (3, 4, 5)
            ORDER BY o.created_at DESC
            LIMIT ?
            "#
        )
        .bind(product_id)
        .bind(limit as i64)
        .fetch_all(self.pool)
        .await?;

        Ok(prices)
    }
}

/// 产品历史成交价格
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ProductHistoryPrice {
    pub product_id: i64,
    pub product_name: String,
    pub unit_price: f64,
    pub quantity: i64,
    pub order_code: String,
    pub customer_name: Option<String>,
    pub order_date: DateTime<Utc>,
}
