//! 采购单数据库查询

use sqlx::SqlitePool;
use tracing::info;

use cicierp_models::purchase::{
    CreatePurchaseOrderRequest, PurchaseOrder, PurchaseOrderDetail, PurchaseOrderItem,
    PurchaseOrderListItem, PurchaseQuery, UpdatePurchaseOrderRequest, ApprovePurchaseRequest,
    ReceivePurchaseRequest,
};

/// 采购单查询器
pub struct PurchaseQueries<'a> {
    pool: &'a SqlitePool,
}

impl<'a> PurchaseQueries<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    /// 生成采购单号
    fn generate_order_code() -> String {
        let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S");
        let random: u32 = rand::random::<u32>() % 10000;
        format!("PO{}{:04}", timestamp, random)
    }

    /// 根据ID获取采购单
    pub async fn get_by_id(&self, id: i64) -> sqlx::Result<Option<PurchaseOrder>> {
        sqlx::query_as::<_, PurchaseOrder>(
            "SELECT id, order_code, supplier_id, supplier_name, total_amount, tax_amount,
                    paid_amount, payment_status, delivery_status, expected_date, actual_date,
                    status, approved_by, approved_at, approval_note, supplier_note, internal_note,
                    attachments, created_at, updated_at
             FROM purchase_orders WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await
    }

    /// 根据采购单号获取
    pub async fn get_by_code(&self, order_code: &str) -> sqlx::Result<Option<PurchaseOrder>> {
        sqlx::query_as::<_, PurchaseOrder>(
            "SELECT id, order_code, supplier_id, supplier_name, total_amount, tax_amount,
                    paid_amount, payment_status, delivery_status, expected_date, actual_date,
                    status, approved_by, approved_at, approval_note, supplier_note, internal_note,
                    attachments, created_at, updated_at
             FROM purchase_orders WHERE order_code = ?"
        )
        .bind(order_code)
        .fetch_optional(self.pool)
        .await
    }

    /// 获取采购单详情（含明细）
    pub async fn get_detail(&self, id: i64) -> sqlx::Result<Option<PurchaseOrderDetail>> {
        let order = self.get_by_id(id).await?;
        match order {
            Some(order) => {
                let items = self.get_items(id).await?;
                Ok(Some(PurchaseOrderDetail { order, items }))
            }
            None => Ok(None),
        }
    }

    /// 获取采购单明细
    pub async fn get_items(&self, order_id: i64) -> sqlx::Result<Vec<PurchaseOrderItem>> {
        sqlx::query_as::<_, PurchaseOrderItem>(
            "SELECT id, order_id, product_id, sku_id, product_name, sku_code, spec_values,
                    quantity, received_qty, unit_price, subtotal, expected_qty, expected_date,
                    inspected_qty, qualified_qty, defective_qty, batch_code, production_date,
                    expiry_date, supplier_id, supplier_name, created_at
             FROM purchase_order_items WHERE order_id = ? ORDER BY id"
        )
        .bind(order_id)
        .fetch_all(self.pool)
        .await
    }

    /// 创建采购单（一单多供应商模式）
    pub async fn create(
        &self,
        req: &CreatePurchaseOrderRequest,
    ) -> sqlx::Result<PurchaseOrder> {
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let order_code = Self::generate_order_code();

        // 计算总金额
        let total_amount: f64 = req.items.iter().map(|i| i.quantity as f64 * i.unit_price).sum();
        let tax_amount = req.tax_amount.unwrap_or(0.0);

        // 获取第一个供应商作为主表的默认值（兼容旧代码）
        let first_supplier_id = req.items.first().map(|i| i.supplier_id).unwrap_or(0);

        // 插入采购单（supplier_id 设为 0 表示多供应商模式）
        let result = sqlx::query(
            "INSERT INTO purchase_orders (order_code, supplier_id, supplier_name, total_amount,
             tax_amount, paid_amount, payment_status, delivery_status, expected_date, supplier_note,
             internal_note, status, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, 0, 1, 1, ?, ?, ?, 1, ?, ?)"
        )
        .bind(&order_code)
        .bind(first_supplier_id)
        .bind("")  // 多供应商模式下主表供应商名称为空
        .bind(total_amount)
        .bind(tax_amount)
        .bind(&req.expected_date)
        .bind(&req.supplier_note)
        .bind(&req.internal_note)
        .bind(&now)
        .bind(&now)
        .execute(self.pool)
        .await?;

        let order_id = result.last_insert_rowid();

        // 插入采购明细（每个明细带自己的供应商）
        for item in &req.items {
            let subtotal = item.quantity as f64 * item.unit_price;

            // 获取供应商名称
            let supplier_name: Option<String> = sqlx::query_scalar(
                "SELECT name FROM suppliers WHERE id = ?"
            )
            .bind(item.supplier_id)
            .fetch_optional(self.pool)
            .await?;

            sqlx::query(
                "INSERT INTO purchase_order_items (order_id, product_id, sku_id, product_name,
                 sku_code, spec_values, quantity, received_qty, unit_price, subtotal, expected_date,
                 batch_code, production_date, expiry_date, inspected_qty, qualified_qty, defective_qty,
                 supplier_id, supplier_name, created_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, 0, ?, ?, ?, ?, ?, ?, 0, 0, 0, ?, ?, ?)"
            )
            .bind(order_id)
            .bind(item.product_id)
            .bind(item.sku_id)
            .bind(&item.product_name)
            .bind(&item.sku_code)
            .bind(&item.spec_values)
            .bind(item.quantity)
            .bind(item.unit_price)
            .bind(subtotal)
            .bind(&item.expected_date)
            .bind(&item.batch_code)
            .bind(&item.production_date)
            .bind(&item.expiry_date)
            .bind(item.supplier_id)
            .bind(&supplier_name)
            .bind(&now)
            .execute(self.pool)
            .await?;
        }

        info!("Purchase order created: {} with {} items", order_code, req.items.len());
        self.get_by_id(order_id).await.map(|o| o.unwrap())
    }

    /// 更新采购单
    pub async fn update(&self, id: i64, req: &UpdatePurchaseOrderRequest) -> sqlx::Result<Option<PurchaseOrder>> {
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        sqlx::query(
            "UPDATE purchase_orders SET
                expected_date = COALESCE(?, expected_date),
                supplier_note = COALESCE(?, supplier_note),
                internal_note = COALESCE(?, internal_note),
                updated_at = ?
             WHERE id = ?"
        )
        .bind(&req.expected_date)
        .bind(&req.supplier_note)
        .bind(&req.internal_note)
        .bind(&now)
        .bind(id)
        .execute(self.pool)
        .await?;

        // 如果有更新明细
        if let Some(ref items) = req.items {
            // 删除旧明细
            sqlx::query("DELETE FROM purchase_order_items WHERE order_id = ?")
                .bind(id)
                .execute(self.pool)
                .await?;

            // 插入新明细
            for item in items {
                let subtotal = item.quantity as f64 * item.unit_price;

                // 获取供应商名称
                let supplier_name: Option<String> = sqlx::query_scalar(
                    "SELECT name FROM suppliers WHERE id = ?"
                )
                .bind(item.supplier_id)
                .fetch_optional(self.pool)
                .await?;

                sqlx::query(
                    "INSERT INTO purchase_order_items (order_id, product_id, sku_id, product_name,
                     sku_code, spec_values, quantity, received_qty, unit_price, subtotal, expected_date,
                     batch_code, production_date, expiry_date, inspected_qty, qualified_qty, defective_qty,
                     supplier_id, supplier_name, created_at)
                     VALUES (?, ?, ?, ?, ?, ?, ?, 0, ?, ?, ?, ?, ?, ?, 0, 0, 0, ?, ?, ?)"
                )
                .bind(id)
                .bind(item.product_id)
                .bind(item.sku_id)
                .bind(&item.product_name)
                .bind(&item.sku_code)
                .bind(&item.spec_values)
                .bind(item.quantity)
                .bind(item.unit_price)
                .bind(subtotal)
                .bind(&item.expected_date)
                .bind(&item.batch_code)
                .bind(&item.production_date)
                .bind(&item.expiry_date)
                .bind(item.supplier_id)
                .bind(&supplier_name)
                .bind(&now)
                .execute(self.pool)
                .await?;
            }

            // 重新计算总金额
            let total_amount: f64 = items.iter().map(|i| i.quantity as f64 * i.unit_price).sum();
            sqlx::query("UPDATE purchase_orders SET total_amount = ?, updated_at = ? WHERE id = ?")
                .bind(total_amount)
                .bind(&now)
                .bind(id)
                .execute(self.pool)
                .await?;
        }

        self.get_by_id(id).await
    }

    /// 审批采购单
    pub async fn approve(&self, id: i64, approved_by: i64, req: &ApprovePurchaseRequest) -> sqlx::Result<bool> {
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        let result = sqlx::query(
            "UPDATE purchase_orders SET status = 2, approved_by = ?, approved_at = ?,
                    approval_note = ?, updated_at = ?
             WHERE id = ? AND status = 1"
        )
        .bind(approved_by)
        .bind(&now)
        .bind(&req.approval_note)
        .bind(&now)
        .bind(id)
        .execute(self.pool)
        .await?;

        if result.rows_affected() > 0 {
            info!("Purchase order {} approved by user {}", id, approved_by);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// 审批通过：待审核(2) → 已审核(3)
    pub async fn confirm(&self, id: i64, confirmed_by: i64) -> sqlx::Result<bool> {
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        let result = sqlx::query(
            "UPDATE purchase_orders SET status = 3, approved_by = ?, approved_at = ?, updated_at = ?
             WHERE id = ? AND status = 2"
        )
        .bind(confirmed_by)
        .bind(&now)
        .bind(&now)
        .bind(id)
        .execute(self.pool)
        .await?;

        if result.rows_affected() > 0 {
            info!("Purchase order {} confirmed by user {}", id, confirmed_by);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// 采购入库
    pub async fn receive(&self, order_id: i64, req: &ReceivePurchaseRequest) -> sqlx::Result<bool> {
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        // 查找对应的采购明细（使用 product_id）
        let item = sqlx::query_as::<_, PurchaseOrderItem>(
            "SELECT * FROM purchase_order_items WHERE order_id = ? AND product_id = ?"
        )
        .bind(order_id)
        .bind(req.product_id)
        .fetch_optional(self.pool)
        .await?;

        let item = match item {
            Some(i) => i,
            None => return Ok(false),
        };

        let qualified_qty = req.qualified_qty.unwrap_or(req.received_qty);
        let defective_qty = req.defective_qty.unwrap_or(0);

        // 更新采购明细收货数量
        sqlx::query(
            "UPDATE purchase_order_items SET
                received_qty = received_qty + ?,
                inspected_qty = inspected_qty + ?,
                qualified_qty = qualified_qty + ?,
                defective_qty = defective_qty + ?,
                batch_code = COALESCE(?, batch_code),
                updated_at = ?
             WHERE id = ?"
        )
        .bind(req.received_qty)
        .bind(req.received_qty)
        .bind(qualified_qty)
        .bind(defective_qty)
        .bind(&req.batch_code)
        .bind(&now)
        .bind(item.id)
        .execute(self.pool)
        .await?;

        // 更新采购单的交货状态
        self.update_delivery_status(order_id).await?;

        // 生成库存流水号
        let movement_code = format!("SM{}{:04}", chrono::Utc::now().format("%Y%m%d%H%M%S"), rand::random::<u32>() % 10000);

        // 记录库存流水
        sqlx::query(
            "INSERT INTO stock_movements (movement_code, product_id, movement_type, quantity,
             before_quantity, after_quantity, reference_type, reference_id, reference_code, note, created_at)
             SELECT ?, ?, 1, ?, COALESCE(available_quantity, 0), COALESCE(available_quantity, 0) + ?,
                    'purchase', ?, (SELECT order_code FROM purchase_orders WHERE id = ?), ?, ?
             FROM inventory WHERE product_id = ?"
        )
        .bind(&movement_code)
        .bind(req.product_id)
        .bind(qualified_qty)
        .bind(qualified_qty)
        .bind(order_id)
        .bind(order_id)
        .bind(&req.note)
        .bind(&now)
        .bind(req.product_id)
        .execute(self.pool)
        .await?;

        // 更新库存
        sqlx::query(
            "INSERT INTO inventory (product_id, total_quantity, available_quantity, locked_quantity,
             damaged_quantity, safety_stock, created_at, updated_at)
             VALUES (?, ?, ?, 0, 0, 10, ?, ?)
             ON CONFLICT(product_id) DO UPDATE SET
                total_quantity = total_quantity + ?,
                available_quantity = available_quantity + ?,
                updated_at = excluded.updated_at"
        )
        .bind(req.product_id)
        .bind(qualified_qty)
        .bind(qualified_qty)
        .bind(&now)
        .bind(&now)
        .bind(qualified_qty)
        .bind(qualified_qty)
        .execute(self.pool)
        .await?;

        info!("Purchase order {} received {} units of product {}", order_id, qualified_qty, req.product_id);
        Ok(true)
    }

    /// 更新交货状态
    async fn update_delivery_status(&self, order_id: i64) -> sqlx::Result<()> {
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        // 检查是否所有明细都已收货
        let (total_qty, received_qty): (i64, i64) = sqlx::query_as(
            "SELECT COALESCE(SUM(quantity), 0), COALESCE(SUM(received_qty), 0)
             FROM purchase_order_items WHERE order_id = ?"
        )
        .bind(order_id)
        .fetch_one(self.pool)
        .await?;

        let delivery_status = if received_qty >= total_qty && total_qty > 0 {
            3 // 已收货
        } else if received_qty > 0 {
            2 // 部分收货
        } else {
            1 // 未收货
        };

        // 如果已全部收货，更新状态为已完成
        let status = if delivery_status == 3 { 4 } else { 3 };

        sqlx::query(
            "UPDATE purchase_orders SET delivery_status = ?, status = ?, updated_at = ? WHERE id = ?"
        )
        .bind(delivery_status)
        .bind(status)
        .bind(&now)
        .bind(order_id)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    /// 删除采购单
    pub async fn delete(&self, id: i64) -> sqlx::Result<bool> {
        // 只能删除待审核的采购单
        let result = sqlx::query("DELETE FROM purchase_orders WHERE id = ? AND status = 1")
            .bind(id)
            .execute(self.pool)
            .await?;

        if result.rows_affected() > 0 {
            // 删除明细
            sqlx::query("DELETE FROM purchase_order_items WHERE order_id = ?")
                .bind(id)
                .execute(self.pool)
                .await?;
            info!("Purchase order {} deleted", id);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// 获取采购单列表
    pub async fn list(
        &self,
        page: u32,
        page_size: u32,
        query: &PurchaseQuery,
    ) -> sqlx::Result<(Vec<PurchaseOrderListItem>, u64)> {
        let offset = (page - 1) * page_size;

        // 构建查询条件
        let mut conditions = Vec::new();
        let mut params: Vec<String> = Vec::new();

        if let Some(supplier_id) = query.supplier_id {
            conditions.push("supplier_id = ?");
            params.push(supplier_id.to_string());
        }
        if let Some(status) = query.status {
            conditions.push("status = ?");
            params.push(status.to_string());
        }
        if let Some(payment_status) = query.payment_status {
            conditions.push("payment_status = ?");
            params.push(payment_status.to_string());
        }
        if let Some(delivery_status) = query.delivery_status {
            conditions.push("delivery_status = ?");
            params.push(delivery_status.to_string());
        }
        if let Some(ref keyword) = query.keyword {
            conditions.push("(order_code LIKE ? OR supplier_name LIKE ?)");
            let pattern = format!("%{}%", keyword);
            params.push(pattern.clone());
            params.push(pattern);
        }
        if let Some(ref date_from) = query.date_from {
            conditions.push("created_at >= ?");
            params.push(date_from.clone());
        }
        if let Some(ref date_to) = query.date_to {
            conditions.push("created_at <= ?");
            params.push(date_to.clone());
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        // 查询总数
        let count_sql = format!("SELECT COUNT(*) FROM purchase_orders {}", where_clause);
        let (total,): (i64,) = if params.is_empty() {
            sqlx::query_as(&count_sql).fetch_one(self.pool).await?
        } else {
            let mut query = sqlx::query_as::<_, (i64,)>(&count_sql);
            for param in &params {
                query = query.bind(param);
            }
            query.fetch_one(self.pool).await?
        };

        // 查询列表
        let list_sql = format!(
            "SELECT po.id, po.order_code, po.supplier_id, po.supplier_name, po.total_amount,
                    po.payment_status, po.delivery_status, po.status, po.expected_date, po.created_at,
                    (SELECT COUNT(*) FROM purchase_order_items WHERE order_id = po.id) as item_count
             FROM purchase_orders po {}
             ORDER BY po.id DESC LIMIT ? OFFSET ?",
            where_clause
        );

        let items: Vec<PurchaseOrderListItem> = if params.is_empty() {
            sqlx::query_as(&list_sql)
                .bind(page_size as i32)
                .bind(offset as i32)
                .fetch_all(self.pool)
                .await?
        } else {
            let mut query = sqlx::query_as::<_, PurchaseOrderListItem>(&list_sql);
            for param in &params {
                query = query.bind(param);
            }
            query = query.bind(page_size as i32).bind(offset as i32);
            query.fetch_all(self.pool).await?
        };

        Ok((items, total as u64))
    }

    /// 统计采购单数量
    pub async fn count(&self) -> sqlx::Result<i64> {
        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM purchase_orders")
            .fetch_one(self.pool)
            .await?;
        Ok(count)
    }
}
