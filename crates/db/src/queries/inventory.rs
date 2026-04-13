//! 库存相关数据库查询

use anyhow::Result;
use sqlx::{QueryBuilder, SqlitePool};

use cicierp_models::{
    inventory::{Inventory, InventoryAlert, InventoryListItem, StockMovement, UpdateInventoryRequest},
    common::PagedResponse,
};

pub struct InventoryQueries<'a> {
    pool: &'a SqlitePool,
}

impl<'a> InventoryQueries<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn list(
        &self,
        page: u32,
        page_size: u32,
        low_stock: Option<bool>,
        product_code: Option<&str>,
        product_name: Option<&str>,
    ) -> Result<PagedResponse<InventoryListItem>> {
        let offset = (page.saturating_sub(1)) * page_size;

        let mut count_query = QueryBuilder::new(
            "SELECT COUNT(*) FROM inventory i
            JOIN products p ON p.id = i.product_id
            WHERE p.deleted_at IS NULL"
        );

        if let Some(true) = low_stock {
            count_query.push(" AND i.available_quantity < i.safety_stock");
        }
        if let Some(code) = product_code {
            count_query.push(" AND p.product_code LIKE ");
            count_query.push_bind(format!("%{}%", code));
        }
        if let Some(name) = product_name {
            count_query.push(" AND p.name LIKE ");
            count_query.push_bind(format!("%{}%", name));
        }

        let total: (i64,) = count_query.build_query_as()
            .fetch_one(self.pool)
            .await?;

        let mut list_query = QueryBuilder::new(
            r#"SELECT
                i.id, i.product_id, p.product_code, p.name as product_name,
                i.total_quantity, i.available_quantity, i.locked_quantity,
                i.safety_stock,
                CASE WHEN i.available_quantity < i.safety_stock THEN 1 ELSE 0 END as is_low_stock
            FROM inventory i
            JOIN products p ON p.id = i.product_id
            WHERE p.deleted_at IS NULL"#
        );

        if let Some(true) = low_stock {
            list_query.push(" AND i.available_quantity < i.safety_stock");
        }
        if let Some(code) = product_code {
            list_query.push(" AND p.product_code LIKE ");
            list_query.push_bind(format!("%{}%", code));
        }
        if let Some(name) = product_name {
            list_query.push(" AND p.name LIKE ");
            list_query.push_bind(format!("%{}%", name));
        }

        list_query.push(" ORDER BY is_low_stock DESC, p.name LIMIT ");
        list_query.push_bind(page_size as i64);
        list_query.push(" OFFSET ");
        list_query.push_bind(offset as i64);

        let items: Vec<InventoryListItem> = list_query.build_query_as()
            .fetch_all(self.pool)
            .await?;

        Ok(PagedResponse::new(items, page, page_size, total.0 as u64))
    }

    pub async fn get_by_product(&self, product_id: i64) -> Result<Option<Inventory>> {
        let inventory: Option<Inventory> = sqlx::query_as(
            "SELECT * FROM inventory WHERE product_id = ?"
        )
        .bind(product_id)
        .fetch_optional(self.pool)
        .await?;
        Ok(inventory)
    }

    pub async fn update(&self, product_id: i64, req: &UpdateInventoryRequest, operator_id: Option<i64>) -> Result<Option<Inventory>> {
        let mut tx = self.pool.begin().await?;

        let current: Option<Inventory> = sqlx::query_as(
            "SELECT * FROM inventory WHERE product_id = ?"
        )
        .bind(product_id)
        .fetch_optional(&mut *tx)
        .await?;

        if current.is_none() {
            let now = chrono::Utc::now().to_rfc3339();
            sqlx::query(
                "INSERT INTO inventory (product_id, total_quantity, available_quantity, created_at, updated_at) VALUES (?, ?, ?, ?, ?)"
            )
            .bind(product_id)
            .bind(req.quantity)
            .bind(req.quantity)
            .bind(&now)
            .bind(&now)
            .execute(&mut *tx)
            .await?;

            tx.commit().await?;
            return self.get_by_product(product_id).await;
        }

        let current = current.unwrap();
        let before_qty = current.available_quantity;
        let after_qty = req.quantity;
        let quantity_change = after_qty - before_qty;

        if quantity_change != 0 {
            let now = chrono::Utc::now().to_rfc3339();

            sqlx::query(
                r#"UPDATE inventory SET
                    total_quantity = ?,
                    available_quantity = ?,
                    updated_at = ?
                WHERE product_id = ?"#
            )
            .bind(after_qty)
            .bind(after_qty)
            .bind(&now)
            .bind(product_id)
            .execute(&mut *tx)
            .await?;

            let movement_code = format!("MV{}", chrono::Utc::now().format("%Y%m%d%H%M%S%3f"));
            let movement_type = if quantity_change > 0 { 1 } else { 2 };

            sqlx::query(
                r#"INSERT INTO stock_movements (
                    movement_code, product_id, movement_type, quantity,
                    before_quantity, after_quantity, note, operator_id, created_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"#
            )
            .bind(&movement_code)
            .bind(product_id)
            .bind(movement_type)
            .bind(quantity_change.abs())
            .bind(before_qty)
            .bind(after_qty)
            .bind(&req.note)
            .bind(operator_id)
            .bind(&now)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        self.get_by_product(product_id).await
    }

    pub async fn lock(&self, product_id: i64, quantity: i64, order_id: Option<i64>) -> Result<bool> {
        let mut tx = self.pool.begin().await?;

        let current: Option<Inventory> = sqlx::query_as(
            "SELECT * FROM inventory WHERE product_id = ?"
        )
        .bind(product_id)
        .fetch_optional(&mut *tx)
        .await?;

        if current.is_none() { return Ok(false); }
        let current = current.unwrap();
        if current.available_quantity < quantity { return Ok(false); }

        let now = chrono::Utc::now().to_rfc3339();
        let new_locked = current.locked_quantity + quantity;
        let new_available = current.available_quantity - quantity;

        sqlx::query(
            "UPDATE inventory SET locked_quantity = ?, available_quantity = ?, updated_at = ? WHERE product_id = ?"
        )
        .bind(new_locked)
        .bind(new_available)
        .bind(&now)
        .bind(product_id)
        .execute(&mut *tx)
        .await?;

        let movement_code = format!("LK{}", chrono::Utc::now().format("%Y%m%d%H%M%S%3f"));
        sqlx::query(
            r#"INSERT INTO stock_movements (
                movement_code, product_id, movement_type, quantity,
                before_quantity, after_quantity, reference_type, reference_id, created_at
            ) VALUES (?, ?, 6, ?, ?, ?, 'order', ?, ?)"#
        )
        .bind(&movement_code)
        .bind(product_id)
        .bind(quantity)
        .bind(current.available_quantity)
        .bind(new_available)
        .bind(order_id)
        .bind(&now)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(true)
    }

    pub async fn unlock(&self, product_id: i64, quantity: i64, order_id: Option<i64>) -> Result<bool> {
        let mut tx = self.pool.begin().await?;

        let current: Option<Inventory> = sqlx::query_as(
            "SELECT * FROM inventory WHERE product_id = ?"
        )
        .bind(product_id)
        .fetch_optional(&mut *tx)
        .await?;

        if current.is_none() { return Ok(false); }
        let current = current.unwrap();
        if current.locked_quantity < quantity { return Ok(false); }

        let now = chrono::Utc::now().to_rfc3339();
        let new_locked = current.locked_quantity - quantity;
        let new_available = current.available_quantity + quantity;

        sqlx::query(
            "UPDATE inventory SET locked_quantity = ?, available_quantity = ?, updated_at = ? WHERE product_id = ?"
        )
        .bind(new_locked)
        .bind(new_available)
        .bind(&now)
        .bind(product_id)
        .execute(&mut *tx)
        .await?;

        let movement_code = format!("UL{}", chrono::Utc::now().format("%Y%m%d%H%M%S%3f"));
        sqlx::query(
            r#"INSERT INTO stock_movements (
                movement_code, product_id, movement_type, quantity,
                before_quantity, after_quantity, reference_type, reference_id, created_at
            ) VALUES (?, ?, 7, ?, ?, ?, 'order', ?, ?)"#
        )
        .bind(&movement_code)
        .bind(product_id)
        .bind(quantity)
        .bind(current.available_quantity)
        .bind(new_available)
        .bind(order_id)
        .bind(&now)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(true)
    }

    pub async fn delete(&self, id: i64) -> Result<bool> {
        let result = sqlx::query("DELETE FROM inventory WHERE id = ?")
            .bind(id)
            .execute(self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn get_alerts(&self) -> Result<Vec<InventoryAlert>> {
        let alerts: Vec<InventoryAlert> = sqlx::query_as(
            r#"SELECT
                i.product_id, p.product_code, p.name as product_name,
                i.available_quantity, i.safety_stock,
                (i.safety_stock - i.available_quantity) as shortage
            FROM inventory i
            JOIN products p ON p.id = i.product_id
            WHERE i.available_quantity < i.safety_stock
            AND p.deleted_at IS NULL
            ORDER BY shortage DESC"#
        )
        .fetch_all(self.pool)
        .await?;
        Ok(alerts)
    }

    pub async fn get_movements(
        &self,
        product_id: Option<i64>,
        page: u32,
        page_size: u32,
    ) -> Result<PagedResponse<StockMovement>> {
        let offset = (page.saturating_sub(1)) * page_size;

        let (count_sql, list_sql) = if let Some(id) = product_id {
            (
                "SELECT COUNT(*) FROM stock_movements WHERE product_id = ?".to_string(),
                format!(
                    "SELECT * FROM stock_movements WHERE product_id = ? ORDER BY created_at DESC LIMIT {} OFFSET {}",
                    page_size, offset
                ),
            )
        } else {
            (
                "SELECT COUNT(*) FROM stock_movements".to_string(),
                format!(
                    "SELECT * FROM stock_movements ORDER BY created_at DESC LIMIT {} OFFSET {}",
                    page_size, offset
                ),
            )
        };

        let total: (i64,) = if product_id.is_some() {
            sqlx::query_as(&count_sql)
                .bind(product_id)
                .fetch_one(self.pool)
                .await?
        } else {
            sqlx::query_as(&count_sql)
                .fetch_one(self.pool)
                .await?
        };

        let items: Vec<StockMovement> = if let Some(id) = product_id {
            sqlx::query_as(&list_sql)
                .bind(id)
                .fetch_all(self.pool)
                .await?
        } else {
            sqlx::query_as(&list_sql)
                .fetch_all(self.pool)
                .await?
        };

        Ok(PagedResponse::new(items, page, page_size, total.0 as u64))
    }

    pub async fn adjust_inventory(
        &self,
        product_id: i64,
        delta_total: i64,
        delta_available: i64,
        delta_locked: i64,
        delta_damaged: i64,
        note: &str,
        operator_id: Option<i64>,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        let current: Option<Inventory> = sqlx::query_as(
            "SELECT * FROM inventory WHERE product_id = ?"
        )
        .bind(product_id)
        .fetch_optional(&mut *tx)
        .await?;

        let now = chrono::Utc::now().to_rfc3339();

        let (before_qty, after_qty) = if let Some(cur) = current {
            let new_total = cur.total_quantity + delta_total;
            let new_available = cur.available_quantity + delta_available;
            let new_locked = cur.locked_quantity + delta_locked;
            let new_damaged = cur.damaged_quantity + delta_damaged;

            sqlx::query(
                r#"UPDATE inventory SET
                    total_quantity = ?,
                    available_quantity = ?,
                    locked_quantity = ?,
                    damaged_quantity = ?,
                    updated_at = ?
                WHERE product_id = ?"#
            )
            .bind(new_total)
            .bind(new_available)
            .bind(new_locked)
            .bind(new_damaged)
            .bind(&now)
            .bind(product_id)
            .execute(&mut *tx)
            .await?;

            (cur.available_quantity, new_available)
        } else {
            sqlx::query(
                r#"INSERT INTO inventory (product_id, total_quantity, available_quantity, locked_quantity, damaged_quantity, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?)"#
            )
            .bind(product_id)
            .bind(delta_total)
            .bind(delta_available)
            .bind(delta_locked)
            .bind(delta_damaged)
            .bind(&now)
            .bind(&now)
            .execute(&mut *tx)
            .await?;

            (0, delta_available)
        };

        let movement_type = if delta_locked > 0 {
            6
        } else if delta_locked < 0 {
            7
        } else if delta_total > 0 && delta_damaged == 0 {
            1
        } else if delta_total < 0 {
            2
        } else if delta_damaged > 0 {
            5
        } else {
            4
        };

        let movement_code = format!("MV{}", chrono::Utc::now().format("%Y%m%d%H%M%S%3f"));
        let quantity = if delta_available != 0 { delta_available.abs() } else { delta_total.abs() };

        sqlx::query(
            r#"INSERT INTO stock_movements (
                movement_code, product_id, movement_type, quantity,
                before_quantity, after_quantity, note, operator_id, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"#
        )
        .bind(&movement_code)
        .bind(product_id)
        .bind(movement_type)
        .bind(quantity)
        .bind(before_qty)
        .bind(after_qty)
        .bind(note)
        .bind(operator_id)
        .bind(&now)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }
}
