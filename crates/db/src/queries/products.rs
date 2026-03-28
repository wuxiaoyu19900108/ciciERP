//! 产品相关数据库查询

use anyhow::Result;
use sqlx::{QueryBuilder, SqlitePool};

use cicierp_models::{
    product::{CreateProductRequest, Product, ProductListItem, ProductSku, UpdateProductRequest},
    common::PagedResponse,
};

use super::product_prices::ProductPriceQueries;

/// 产品查询结构体
pub struct ProductQueries<'a> {
    pool: &'a SqlitePool,
}

impl<'a> ProductQueries<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    /// 获取产品列表（使用安全的参数化查询）
    pub async fn list(
        &self,
        page: u32,
        page_size: u32,
        category_id: Option<i64>,
        brand_id: Option<i64>,
        status: Option<i64>,
        keyword: Option<&str>,
    ) -> Result<PagedResponse<ProductListItem>> {
        let offset = (page.saturating_sub(1)) * page_size;

        // 构建安全的 count 查询
        let mut count_query = QueryBuilder::new("SELECT COUNT(*) as count FROM products p WHERE p.deleted_at IS NULL");

        if let Some(cat_id) = category_id {
            count_query.push(" AND p.category_id = ");
            count_query.push_bind(cat_id);
        }
        if let Some(b_id) = brand_id {
            count_query.push(" AND p.brand_id = ");
            count_query.push_bind(b_id);
        }
        if let Some(s) = status {
            count_query.push(" AND p.status = ");
            count_query.push_bind(s);
        }
        if let Some(kw) = keyword {
            count_query.push(" AND (p.name LIKE ");
            count_query.push_bind(format!("%{}%", kw));
            count_query.push(" OR p.product_code LIKE ");
            count_query.push_bind(format!("%{}%", kw));
            count_query.push(")");
        }

        let total: (i64,) = count_query.build_query_as()
            .fetch_one(self.pool)
            .await?;

        // 构建安全的 list 查询（从 product_prices 获取参考售价，从 product_costs 获取成本）
        let mut list_query = QueryBuilder::new(
            r#"SELECT
                p.id, p.product_code, p.name, p.main_image, p.status,
                p.created_at,
                pc.cost_cny,
                pp.sale_price_cny,
                COALESCE(SUM(ps.stock_quantity), 0) as stock_quantity,
                c.name as category_name,
                b.name as brand_name
            FROM products p
            LEFT JOIN product_skus ps ON ps.product_id = p.id
            LEFT JOIN categories c ON c.id = p.category_id
            LEFT JOIN brands b ON b.id = p.brand_id
            LEFT JOIN product_prices pp ON pp.product_id = p.id AND pp.is_reference = 1 AND pp.platform = 'website'
            LEFT JOIN product_costs pc ON pc.product_id = p.id AND pc.is_reference = 1
            WHERE p.deleted_at IS NULL"#
        );

        if let Some(cat_id) = category_id {
            list_query.push(" AND p.category_id = ");
            list_query.push_bind(cat_id);
        }
        if let Some(b_id) = brand_id {
            list_query.push(" AND p.brand_id = ");
            list_query.push_bind(b_id);
        }
        if let Some(s) = status {
            list_query.push(" AND p.status = ");
            list_query.push_bind(s);
        }
        if let Some(kw) = keyword {
            list_query.push(" AND (p.name LIKE ");
            list_query.push_bind(format!("%{}%", kw));
            list_query.push(" OR p.product_code LIKE ");
            list_query.push_bind(format!("%{}%", kw));
            list_query.push(")");
        }

        list_query.push(" GROUP BY p.id ORDER BY p.created_at DESC LIMIT ");
        list_query.push_bind(page_size as i64);
        list_query.push(" OFFSET ");
        list_query.push_bind(offset as i64);

        let items: Vec<ProductListItem> = list_query.build_query_as()
            .fetch_all(self.pool)
            .await?;

        Ok(PagedResponse::new(items, page, page_size, total.0 as u64))
    }

    /// 根据 ID 获取产品
    pub async fn get_by_id(&self, id: i64) -> Result<Option<Product>> {
        let product: Option<Product> = sqlx::query_as(
            "SELECT * FROM products WHERE id = ? AND deleted_at IS NULL"
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await?;

        Ok(product)
    }

    /// 根据编码获取产品
    pub async fn get_by_code(&self, product_code: &str) -> Result<Option<Product>> {
        let product: Option<Product> = sqlx::query_as(
            "SELECT * FROM products WHERE product_code = ? AND deleted_at IS NULL"
        )
        .bind(product_code)
        .fetch_optional(self.pool)
        .await?;

        Ok(product)
    }

    /// 创建产品
    pub async fn create(&self, req: &CreateProductRequest) -> Result<Product> {
        let now = chrono::Utc::now().to_rfc3339();
        let status = req.status.unwrap_or(1);
        let is_featured = req.is_featured.unwrap_or(false) as i64;
        let is_new = req.is_new.unwrap_or(false) as i64;
        let specifications = req.specifications.clone().unwrap_or(serde_json::json!({}));
        let images = req.images.clone().unwrap_or(serde_json::json!([]));

        // 如果没有提供 product_code，自动生成
        let product_code = match &req.product_code {
            Some(code) => code.clone(),
            None => self.generate_product_code().await?,
        };

        let result = sqlx::query(
            r#"
            INSERT INTO products (
                product_code, name, name_en, slug, category_id, brand_id, supplier_id,
                weight, volume,
                description, description_en, specifications, main_image, images,
                status, is_featured, is_new, notes, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&product_code)
        .bind(&req.name)
        .bind(&req.name_en)
        .bind(&req.slug)
        .bind(req.category_id)
        .bind(req.brand_id)
        .bind(req.supplier_id)
        .bind(req.weight)
        .bind(req.volume)
        .bind(&req.description)
        .bind(&req.description_en)
        .bind(&specifications)
        .bind(&req.main_image)
        .bind(&images)
        .bind(status)
        .bind(is_featured)
        .bind(is_new)
        .bind(&req.notes)
        .bind(&now)
        .bind(&now)
        .execute(self.pool)
        .await?;

        let id = result.last_insert_rowid();
        self.get_by_id(id).await?.ok_or_else(|| anyhow::anyhow!("Failed to fetch created product"))
    }

    /// 更新产品
    pub async fn update(&self, id: i64, req: &UpdateProductRequest) -> Result<Option<Product>> {
        // 先检查产品是否存在
        if self.get_by_id(id).await?.is_none() {
            return Ok(None);
        }

        let now = chrono::Utc::now().to_rfc3339();

        // 动态构建更新语句
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
        if let Some(ref slug) = req.slug {
            updates.push("slug = ?");
            bindings.push(slug.clone());
        }
        if req.category_id.is_some() {
            updates.push("category_id = ?");
        }
        if req.brand_id.is_some() {
            updates.push("brand_id = ?");
        }
        if req.supplier_id.is_some() {
            updates.push("supplier_id = ?");
        }
        if req.weight.is_some() {
            updates.push("weight = ?");
        }
        if req.volume.is_some() {
            updates.push("volume = ?");
        }
        if let Some(ref desc) = req.description {
            updates.push("description = ?");
            bindings.push(desc.clone());
        }
        if let Some(ref desc_en) = req.description_en {
            updates.push("description_en = ?");
            bindings.push(desc_en.clone());
        }
        if let Some(ref specs) = req.specifications {
            updates.push("specifications = ?");
            bindings.push(specs.to_string());
        }
        if let Some(ref img) = req.main_image {
            updates.push("main_image = ?");
            bindings.push(img.clone());
        }
        if let Some(ref imgs) = req.images {
            updates.push("images = ?");
            bindings.push(imgs.to_string());
        }
        if let Some(s) = req.status {
            updates.push("status = ?");
            bindings.push(s.to_string());
        }
        if let Some(f) = req.is_featured {
            updates.push("is_featured = ?");
            bindings.push((f as i64).to_string());
        }
        if let Some(n) = req.is_new {
            updates.push("is_new = ?");
            bindings.push((n as i64).to_string());
        }
        if let Some(ref notes) = req.notes {
            updates.push("notes = ?");
            bindings.push(notes.clone());
        }

        let sql = format!(
            "UPDATE products SET {} WHERE id = ? AND deleted_at IS NULL",
            updates.join(", ")
        );

        let mut query = sqlx::query(&sql);
        for bind in &bindings {
            query = query.bind(bind);
        }
        if let Some(cat_id) = req.category_id {
            query = query.bind(cat_id);
        }
        if let Some(b_id) = req.brand_id {
            query = query.bind(b_id);
        }
        if let Some(s_id) = req.supplier_id {
            query = query.bind(s_id);
        }
        if let Some(w) = req.weight {
            query = query.bind(w);
        }
        if let Some(v) = req.volume {
            query = query.bind(v);
        }
        query = query.bind(id);

        query.execute(self.pool).await?;

        self.get_by_id(id).await
    }

    /// 软删除产品
    pub async fn delete(&self, id: i64) -> Result<bool> {
        let now = chrono::Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE products SET deleted_at = ? WHERE id = ? AND deleted_at IS NULL"
        )
        .bind(&now)
        .bind(id)
        .execute(self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// 获取产品的 SKU 列表
    pub async fn get_skus(&self, product_id: i64) -> Result<Vec<ProductSku>> {
        let skus: Vec<ProductSku> = sqlx::query_as(
            "SELECT * FROM product_skus WHERE product_id = ? ORDER BY id"
        )
        .bind(product_id)
        .fetch_all(self.pool)
        .await?;

        Ok(skus)
    }

    /// 全文搜索产品
    pub async fn search(&self, keyword: &str, page: u32, page_size: u32) -> Result<PagedResponse<ProductListItem>> {
        let offset = (page.saturating_sub(1)) * page_size;

        // 查询总数
        let count_sql = r#"
            SELECT COUNT(*) FROM products p
            JOIN products_fts fts ON fts.rowid = p.id
            WHERE products_fts MATCH ? AND p.deleted_at IS NULL
        "#;
        let total: (i64,) = sqlx::query_as(count_sql)
            .bind(keyword)
            .fetch_one(self.pool)
            .await?;

        // 搜索列表（从 product_prices 获取参考售价）
        let list_sql = r#"
            SELECT
                p.id, p.product_code, p.name, p.main_image, p.status,
                p.created_at,
                pp.sale_price_cny,
                COALESCE(SUM(ps.stock_quantity), 0) as stock_quantity,
                c.name as category_name,
                b.name as brand_name
            FROM products p
            JOIN products_fts fts ON fts.rowid = p.id
            LEFT JOIN product_skus ps ON ps.product_id = p.id
            LEFT JOIN categories c ON c.id = p.category_id
            LEFT JOIN brands b ON b.id = p.brand_id
            LEFT JOIN product_prices pp ON pp.product_id = p.id AND pp.is_reference = 1 AND pp.platform = 'website'
            WHERE products_fts MATCH ? AND p.deleted_at IS NULL
            GROUP BY p.id
            ORDER BY p.created_at DESC
            LIMIT ? OFFSET ?
        "#;

        let items: Vec<ProductListItem> = sqlx::query_as(list_sql)
            .bind(keyword)
            .bind(page_size as i64)
            .bind(offset as i64)
            .fetch_all(self.pool)
            .await?;

        Ok(PagedResponse::new(items, page, page_size, total.0 as u64))
    }

    /// 获取产品总数
    pub async fn count(&self) -> sqlx::Result<i64> {
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM products WHERE deleted_at IS NULL"
        )
        .fetch_one(self.pool)
        .await?;
        Ok(count.0)
    }

    /// 生成产品编号
    /// 格式：SKU-YYYYMMDD-XXXX
    /// SKU- 固定前缀
    /// YYYYMMDD 当天日期
    /// XXXX 当日4位序号（从0001开始）
    pub async fn generate_product_code(&self) -> Result<String> {
        let today = chrono::Utc::now().format("%Y%m%d").to_string();
        let prefix = format!("SKU-{}-", today);

        // 查询当天已有的产品数量（包含已删除的，保证编号唯一性）
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM products WHERE product_code LIKE ?"
        )
        .bind(format!("{}%", prefix))
        .fetch_one(self.pool)
        .await?;

        let sequence = count.0 + 1;
        let product_code = format!("{}{:04}", prefix, sequence);

        Ok(product_code)
    }

    /// 获取指定时间后更新的产品列表（用于增量同步）
    pub async fn list_updated_since(&self, updated_after: &str, limit: u32) -> Result<Vec<Product>> {
        let products: Vec<Product> = sqlx::query_as(
            "SELECT * FROM products WHERE updated_at > ? AND deleted_at IS NULL ORDER BY updated_at ASC LIMIT ?"
        )
        .bind(updated_after)
        .bind(limit as i64)
        .fetch_all(self.pool)
        .await?;

        Ok(products)
    }
}
