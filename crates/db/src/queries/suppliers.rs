//! 供应商相关数据库查询

use anyhow::Result;
use sqlx::{QueryBuilder, SqlitePool};

use cicierp_models::{
    supplier::{CreateSupplierRequest, ProductSupplierInfo, Supplier, UpdateSupplierRequest},
    common::PagedResponse,
};

pub struct SupplierQueries<'a> {
    pool: &'a SqlitePool,
}

impl<'a> SupplierQueries<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    /// 生成供应商编码 SUP-YYYYMMDD-XXXX
    async fn generate_supplier_code(&self) -> Result<String> {
        let today = chrono::Utc::now().format("%Y%m%d").to_string();
        let prefix = format!("SUP-{}-", today);

        // 查询今天已有的编码数量
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM suppliers WHERE supplier_code LIKE ?"
        )
        .bind(format!("{}%", prefix))
        .fetch_one(self.pool)
        .await?;

        let seq = count.0 + 1;
        Ok(format!("{}{:04}", prefix, seq))
    }

    /// 获取供应商列表（使用安全的参数化查询）
    pub async fn list(
        &self,
        page: u32,
        page_size: u32,
        status: Option<i64>,
        rating_level: Option<&str>,
        keyword: Option<&str>,
    ) -> Result<PagedResponse<Supplier>> {
        let offset = (page.saturating_sub(1)) * page_size;

        // 构建安全的 count 查询
        let mut count_query = QueryBuilder::new("SELECT COUNT(*) FROM suppliers WHERE deleted_at IS NULL");

        if let Some(s) = status {
            count_query.push(" AND status = ");
            count_query.push_bind(s);
        }
        if let Some(r) = rating_level {
            count_query.push(" AND rating_level = ");
            count_query.push_bind(r);
        }
        if let Some(kw) = keyword {
            count_query.push(" AND (name LIKE ");
            count_query.push_bind(format!("%{}%", kw));
            count_query.push(" OR supplier_code LIKE ");
            count_query.push_bind(format!("%{}%", kw));
            count_query.push(" OR contact_person LIKE ");
            count_query.push_bind(format!("%{}%", kw));
            count_query.push(")");
        }

        let total: (i64,) = count_query.build_query_as()
            .fetch_one(self.pool)
            .await?;

        // 构建安全的 list 查询
        let mut list_query = QueryBuilder::new("SELECT * FROM suppliers WHERE deleted_at IS NULL");

        if let Some(s) = status {
            list_query.push(" AND status = ");
            list_query.push_bind(s);
        }
        if let Some(r) = rating_level {
            list_query.push(" AND rating_level = ");
            list_query.push_bind(r);
        }
        if let Some(kw) = keyword {
            list_query.push(" AND (name LIKE ");
            list_query.push_bind(format!("%{}%", kw));
            list_query.push(" OR supplier_code LIKE ");
            list_query.push_bind(format!("%{}%", kw));
            list_query.push(" OR contact_person LIKE ");
            list_query.push_bind(format!("%{}%", kw));
            list_query.push(")");
        }

        list_query.push(" ORDER BY created_at DESC LIMIT ");
        list_query.push_bind(page_size as i64);
        list_query.push(" OFFSET ");
        list_query.push_bind(offset as i64);

        let items: Vec<Supplier> = list_query.build_query_as()
            .fetch_all(self.pool)
            .await?;

        Ok(PagedResponse::new(items, page, page_size, total.0 as u64))
    }

    /// 根据 ID 获取供应商
    pub async fn get_by_id(&self, id: i64) -> Result<Option<Supplier>> {
        let supplier: Option<Supplier> = sqlx::query_as(
            "SELECT * FROM suppliers WHERE id = ? AND deleted_at IS NULL"
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await?;

        Ok(supplier)
    }

    /// 创建供应商（自动生成编码）
    pub async fn create(&self, req: &CreateSupplierRequest) -> Result<Supplier> {
        let now = chrono::Utc::now().to_rfc3339();
        let supplier_code = self.generate_supplier_code().await?;
        let rating_level = req.rating_level.clone().unwrap_or_else(|| "C".to_string());
        let rating_score = req.rating_score.unwrap_or(3.5);
        let payment_terms = req.payment_terms.unwrap_or(30);

        let result = sqlx::query(
            r#"
            INSERT INTO suppliers (
                supplier_code, name, name_en, contact_person, contact_phone, contact_email,
                address, credit_code, tax_id, bank_name, bank_account,
                rating_level, rating_score, payment_terms, payment_method,
                notes, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&supplier_code)
        .bind(&req.name)
        .bind(&req.name_en)
        .bind(&req.contact_person)
        .bind(&req.contact_phone)
        .bind(&req.contact_email)
        .bind(&req.address)
        .bind(&req.credit_code)
        .bind(&req.tax_id)
        .bind(&req.bank_name)
        .bind(&req.bank_account)
        .bind(&rating_level)
        .bind(rating_score)
        .bind(payment_terms)
        .bind(&req.payment_method)
        .bind(&req.notes)
        .bind(&now)
        .bind(&now)
        .execute(self.pool)
        .await?;

        let id = result.last_insert_rowid();
        self.get_by_id(id).await?.ok_or_else(|| anyhow::anyhow!("Failed to fetch created supplier"))
    }

    /// 更新供应商
    pub async fn update(&self, id: i64, req: &UpdateSupplierRequest) -> Result<Option<Supplier>> {
        if self.get_by_id(id).await?.is_none() {
            return Ok(None);
        }

        let now = chrono::Utc::now().to_rfc3339();
        let mut updates = vec!["updated_at = ?"];
        let mut bindings: Vec<String> = vec![now.clone()];

        if let Some(ref name) = req.name {
            updates.push("name = ?");
            bindings.push(name.clone());
        }
        if let Some(ref name_en) = req.name_en {
            updates.push("name_en = ?");
            bindings.push(name_en.clone());
        }
        if let Some(ref cp) = req.contact_person {
            updates.push("contact_person = ?");
            bindings.push(cp.clone());
        }
        if let Some(ref phone) = req.contact_phone {
            updates.push("contact_phone = ?");
            bindings.push(phone.clone());
        }
        if let Some(ref email) = req.contact_email {
            updates.push("contact_email = ?");
            bindings.push(email.clone());
        }
        if let Some(ref addr) = req.address {
            updates.push("address = ?");
            bindings.push(addr.clone());
        }
        if let Some(ref rl) = req.rating_level {
            updates.push("rating_level = ?");
            bindings.push(rl.clone());
        }
        if let Some(rs) = req.rating_score {
            updates.push("rating_score = ?");
            bindings.push(rs.to_string());
        }
        if let Some(pt) = req.payment_terms {
            updates.push("payment_terms = ?");
            bindings.push(pt.to_string());
        }
        if let Some(ref pm) = req.payment_method {
            updates.push("payment_method = ?");
            bindings.push(pm.clone());
        }
        if let Some(s) = req.status {
            updates.push("status = ?");
            bindings.push(s.to_string());
        }
        if let Some(ref notes) = req.notes {
            updates.push("notes = ?");
            bindings.push(notes.clone());
        }

        let sql = format!(
            "UPDATE suppliers SET {} WHERE id = ? AND deleted_at IS NULL",
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

    /// 软删除供应商
    pub async fn delete(&self, id: i64) -> Result<bool> {
        let now = chrono::Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE suppliers SET deleted_at = ? WHERE id = ? AND deleted_at IS NULL"
        )
        .bind(&now)
        .bind(id)
        .execute(self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// 获取供应商的产品列表
    pub async fn get_products(&self, supplier_id: i64) -> Result<Vec<ProductSupplierInfo>> {
        let products: Vec<ProductSupplierInfo> = sqlx::query_as(
            r#"
            SELECT
                p.id as product_id, p.product_code, p.name as product_name,
                ps.supplier_sku, ps.purchase_price, ps.min_order_qty,
                ps.lead_time, ps.is_primary
            FROM product_suppliers ps
            JOIN products p ON p.id = ps.product_id
            WHERE ps.supplier_id = ? AND p.deleted_at IS NULL
            ORDER BY ps.is_primary DESC, p.name
            "#
        )
        .bind(supplier_id)
        .fetch_all(self.pool)
        .await?;

        Ok(products)
    }
}
