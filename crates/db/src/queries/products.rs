//! 产品相关数据库查询

use anyhow::Result;
use sqlx::{QueryBuilder, Sqlite, SqlitePool, Transaction};

use cicierp_models::{
    product::{CreateProductRequest, Product, ProductDashboardStats, ProductListItem, UpdateProductRequest},
    common::PagedResponse,
};

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
        supplier_id: Option<i64>,
        price_min: Option<f64>,
        price_max: Option<f64>,
    ) -> Result<PagedResponse<ProductListItem>> {
        let offset = (page.saturating_sub(1)) * page_size;

        // 构建安全的 count 查询
        let mut count_query = QueryBuilder::new(
            "SELECT COUNT(DISTINCT p.id) as count FROM products p \
             LEFT JOIN product_prices pp ON pp.product_id = p.id AND pp.is_reference = 1 AND pp.platform = 'website' \
             WHERE p.deleted_at IS NULL"
        );

        if let Some(cat_id) = category_id {
            count_query.push(" AND p.category_id = ");
            count_query.push_bind(cat_id);
        }
        if let Some(b_id) = brand_id {
            count_query.push(" AND p.brand_id = ");
            count_query.push_bind(b_id);
        }
        if let Some(s_id) = supplier_id {
            count_query.push(" AND p.supplier_id = ");
            count_query.push_bind(s_id);
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
            count_query.push(" OR p.model LIKE ");
            count_query.push_bind(format!("%{}%", kw));
            count_query.push(")");
        }
        if let Some(min) = price_min {
            count_query.push(" AND pp.sale_price_usd >= ");
            count_query.push_bind(min);
        }
        if let Some(max) = price_max {
            count_query.push(" AND pp.sale_price_usd <= ");
            count_query.push_bind(max);
        }

        let total: (i64,) = count_query.build_query_as()
            .fetch_one(self.pool)
            .await?;

        // 构建安全的 list 查询（从 product_prices 获取参考售价，从 product_costs 获取成本）
        // 利润计算：美金售价 - 美金成本 - 平台费率*售价（platform_fee_rate 用于参考利润估算）
        let mut list_query = QueryBuilder::new(
            r#"SELECT
                p.id AS id,
                p.product_code AS product_code,
                p.name AS name,
                p.model AS model,
                p.main_image AS main_image,
                s.name AS supplier_name,
                pc.cost_cny AS cost_cny,
                pc.cost_usd AS cost_usd,
                pc.exchange_rate AS cost_exchange_rate,
                pp.sale_price_cny AS sale_price_cny,
                pp.sale_price_usd AS sale_price_usd,
                pp.exchange_rate AS price_exchange_rate,
                CASE
                    WHEN pp.sale_price_usd IS NOT NULL AND pc.cost_usd IS NOT NULL
                    THEN pp.sale_price_usd - pc.cost_usd - (pp.sale_price_usd * COALESCE(pp.platform_fee_rate, 0))
                    ELSE NULL
                END AS profit_usd,
                CASE
                    WHEN pp.sale_price_usd IS NOT NULL AND pp.sale_price_usd > 0
                    THEN ((pp.sale_price_usd - COALESCE(pc.cost_usd, 0) - (pp.sale_price_usd * COALESCE(pp.platform_fee_rate, 0))) / pp.sale_price_usd) * 100
                    ELSE NULL
                END AS profit_margin,
                (
                    SELECT GROUP_CONCAT(
                        CASE platform
                            WHEN 'alibaba' THEN 'Ali'
                            WHEN 'aliexpress' THEN 'AE'
                            WHEN 'website' THEN 'Web'
                            ELSE platform
                        END || ':' || ROUND(platform_fee_rate * 100, 1) || '%',
                        ' | '
                    )
                    FROM product_prices
                    WHERE product_id = p.id AND is_reference = 1
                    ORDER BY platform
                ) AS platform_fees,
                p.status AS status,
                CAST(0 AS INTEGER) AS stock_quantity,
                c.name AS category_name,
                b.name AS brand_name,
                p.created_at AS created_at
            FROM products p
            LEFT JOIN categories c ON c.id = p.category_id
            LEFT JOIN brands b ON b.id = p.brand_id
            LEFT JOIN suppliers s ON s.id = p.supplier_id
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
        if let Some(s_id) = supplier_id {
            list_query.push(" AND p.supplier_id = ");
            list_query.push_bind(s_id);
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
            list_query.push(" OR p.model LIKE ");
            list_query.push_bind(format!("%{}%", kw));
            list_query.push(")");
        }
        if let Some(min) = price_min {
            list_query.push(" AND pp.sale_price_usd >= ");
            list_query.push_bind(min);
        }
        if let Some(max) = price_max {
            list_query.push(" AND pp.sale_price_usd <= ");
            list_query.push_bind(max);
        }

        list_query.push(" GROUP BY p.id ORDER BY CAST(SUBSTR(p.product_code, 4) AS INTEGER) DESC LIMIT ");
        list_query.push_bind(page_size as i64);
        list_query.push(" OFFSET ");
        list_query.push_bind(offset as i64);

        let items: Vec<ProductListItem> = list_query.build_query_as()
            .fetch_all(self.pool)
            .await?;

        Ok(PagedResponse::new(items, page, page_size, total.0 as u64))
    }

    pub async fn dashboard_stats(
        &self,
        category_id: Option<i64>,
        supplier_id: Option<i64>,
        keyword: Option<&str>,
    ) -> Result<ProductDashboardStats> {
        let mut q = QueryBuilder::new(
            r#"SELECT
                CAST(COUNT(DISTINCT p.id) AS INTEGER) as total_count,
                CAST(0 AS REAL) as total_stock,
                CAST(COALESCE(AVG(pp.sale_price_usd), 0) AS REAL) as avg_price_usd,
                CAST(0 AS REAL) as total_stock_value
            FROM products p
            LEFT JOIN product_prices pp ON pp.product_id = p.id AND pp.is_reference = 1
            WHERE p.deleted_at IS NULL"#
        );
        if let Some(cid) = category_id {
            q.push(" AND p.category_id = "); q.push_bind(cid);
        }
        if let Some(sid) = supplier_id {
            q.push(" AND p.supplier_id = "); q.push_bind(sid);
        }
        if let Some(kw) = keyword {
            q.push(" AND (p.name LIKE "); q.push_bind(format!("%{}%", kw));
            q.push(" OR p.product_code LIKE "); q.push_bind(format!("%{}%", kw));
            q.push(")");
        }
        let stats: ProductDashboardStats = q.build_query_as().fetch_one(self.pool).await?;
        Ok(stats)
    }

    /// 根据 ID 获取产品
    pub async fn get_by_id(&self, id: i64) -> Result<Option<Product>> {
        let product: Option<Product> = sqlx::query_as(
            r#"SELECT
                id,
                product_code,
                name,
                model,
                name_en,
                slug,
                category_id,
                brand_id,
                supplier_id,
                weight,
                volume,
                description,
                description_en,
                specifications,
                main_image,
                images,
                status,
                is_featured,
                is_new,
                view_count,
                sales_count,
                notes,
                unit,
                created_at,
                updated_at,
                deleted_at
            FROM products
            WHERE id = ? AND deleted_at IS NULL"#
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await?;

        Ok(product)
    }

    /// 根据编码获取产品
    pub async fn get_by_code(&self, product_code: &str) -> Result<Option<Product>> {
        let product: Option<Product> = sqlx::query_as(
            r#"SELECT
                id,
                product_code,
                name,
                model,
                name_en,
                slug,
                category_id,
                brand_id,
                supplier_id,
                weight,
                volume,
                description,
                description_en,
                specifications,
                main_image,
                images,
                status,
                is_featured,
                is_new,
                view_count,
                sales_count,
                notes,
                unit,
                created_at,
                updated_at,
                deleted_at
            FROM products
            WHERE product_code = ? AND deleted_at IS NULL"#
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
                product_code, name, model, name_en, slug, category_id, brand_id, supplier_id,
                weight, volume,
                description, description_en, specifications, main_image, images,
                status, is_featured, is_new, notes, unit,
                purchase_price, sale_price, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&product_code)
        .bind(&req.name)
        .bind(&req.model)
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
        .bind(req.unit.as_deref().unwrap_or("pcs"))
        .bind(0.0_f64) // purchase_price: 价格已迁移到 product_costs/product_prices，此处保持默认值
        .bind(0.0_f64) // sale_price: 同上
        .bind(&now)
        .bind(&now)
        .execute(self.pool)
        .await?;

        let id = result.last_insert_rowid();
        self.get_by_id(id).await?.ok_or_else(|| anyhow::anyhow!("Failed to fetch created product"))
    }

    pub async fn create_in_tx(
        &self,
        tx: &mut Transaction<'_, Sqlite>,
        req: &CreateProductRequest,
    ) -> Result<Product> {
        let now = chrono::Utc::now().to_rfc3339();
        let status = req.status.unwrap_or(1);
        let is_featured = req.is_featured.unwrap_or(false) as i64;
        let is_new = req.is_new.unwrap_or(false) as i64;
        let specifications = req.specifications.clone().unwrap_or(serde_json::json!({}));
        let images = req.images.clone().unwrap_or(serde_json::json!([]));
        let product_code = match &req.product_code {
            Some(code) => code.clone(),
            None => self.generate_product_code().await?,
        };

        let result = sqlx::query(
            r#"
            INSERT INTO products (
                product_code, name, model, name_en, slug, category_id, brand_id, supplier_id,
                weight, volume,
                description, description_en, specifications, main_image, images,
                status, is_featured, is_new, notes, unit,
                purchase_price, sale_price, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&product_code)
        .bind(&req.name)
        .bind(&req.model)
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
        .bind(req.unit.as_deref().unwrap_or("pcs"))
        .bind(0.0_f64)
        .bind(0.0_f64)
        .bind(&now)
        .bind(&now)
        .execute(&mut **tx)
        .await?;

        let id = result.last_insert_rowid();
        Self::get_by_id_in_tx(tx, id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to fetch created product"))
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
        if let Some(ref model) = req.model {
            updates.push("model = ?");
            bindings.push(model.clone());
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
        if let Some(ref unit) = req.unit {
            updates.push("unit = ?");
            bindings.push(unit.clone());
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

    pub async fn update_in_tx(
        &self,
        tx: &mut Transaction<'_, Sqlite>,
        id: i64,
        req: &UpdateProductRequest,
    ) -> Result<Option<Product>> {
        if Self::get_by_id_in_tx(tx, id).await?.is_none() {
            return Ok(None);
        }

        let now = chrono::Utc::now().to_rfc3339();
        let mut updates = vec!["updated_at = ?"];
        let mut bindings: Vec<String> = vec![now.clone()];

        if let Some(ref name) = req.name {
            updates.push("name = ?");
            bindings.push(name.clone());
        }
        if let Some(ref model) = req.model {
            updates.push("model = ?");
            bindings.push(model.clone());
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
        if let Some(ref unit) = req.unit {
            updates.push("unit = ?");
            bindings.push(unit.clone());
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
        query.execute(&mut **tx).await?;

        Self::get_by_id_in_tx(tx, id).await
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
                p.id AS id,
                p.product_code AS product_code,
                p.name AS name,
                p.model AS model,
                p.main_image AS main_image,
                NULL AS supplier_name,
                pc.cost_cny AS cost_cny,
                pc.cost_usd AS cost_usd,
                pc.exchange_rate AS cost_exchange_rate,
                pp.sale_price_cny AS sale_price_cny,
                pp.sale_price_usd AS sale_price_usd,
                pp.exchange_rate AS price_exchange_rate,
                CASE
                    WHEN pp.sale_price_usd IS NOT NULL AND pc.cost_usd IS NOT NULL
                    THEN pp.sale_price_usd - pc.cost_usd - (pp.sale_price_usd * COALESCE(pp.platform_fee_rate, 0))
                    ELSE NULL
                END AS profit_usd,
                CASE
                    WHEN pp.sale_price_usd IS NOT NULL AND pp.sale_price_usd > 0
                    THEN ((pp.sale_price_usd - COALESCE(pc.cost_usd, 0) - (pp.sale_price_usd * COALESCE(pp.platform_fee_rate, 0))) / pp.sale_price_usd) * 100
                    ELSE NULL
                END AS profit_margin,
                (
                    SELECT GROUP_CONCAT(
                        CASE platform
                            WHEN 'alibaba' THEN 'Ali'
                            WHEN 'aliexpress' THEN 'AE'
                            WHEN 'website' THEN 'Web'
                            ELSE platform
                        END || ':' || ROUND(platform_fee_rate * 100, 1) || '%',
                        ' | '
                    )
                    FROM product_prices
                    WHERE product_id = p.id AND is_reference = 1
                    ORDER BY platform
                ) AS platform_fees,
                p.status AS status,
                CAST(0 AS INTEGER) AS stock_quantity,
                c.name AS category_name,
                b.name AS brand_name,
                p.created_at AS created_at
            FROM products p
            JOIN products_fts fts ON fts.rowid = p.id
            LEFT JOIN categories c ON c.id = p.category_id
            LEFT JOIN brands b ON b.id = p.brand_id
            LEFT JOIN product_prices pp ON pp.product_id = p.id AND pp.is_reference = 1 AND pp.platform = 'website'
            LEFT JOIN product_costs pc ON pc.product_id = p.id AND pc.is_reference = 1
            WHERE products_fts MATCH ? AND p.deleted_at IS NULL
            GROUP BY p.id
            ORDER BY CAST(SUBSTR(p.product_code, 4) AS INTEGER) DESC
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
        // 查询所有 SP-N 格式编码中的最大序号（包含已删除，保证不重复）
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT product_code FROM products WHERE product_code LIKE 'SP-%' \
             ORDER BY CAST(SUBSTR(product_code, 4) AS INTEGER) DESC LIMIT 1"
        )
        .fetch_optional(self.pool)
        .await?;

        let next_n = if let Some((code,)) = row {
            code.trim_start_matches("SP-")
                .parse::<u64>()
                .unwrap_or(0) + 1
        } else {
            1
        };

        Ok(format!("SP-{}", next_n))
    }

    async fn get_by_id_in_tx(
        tx: &mut Transaction<'_, Sqlite>,
        id: i64,
    ) -> Result<Option<Product>> {
        let product: Option<Product> = sqlx::query_as(
            r#"SELECT
                id,
                product_code,
                name,
                model,
                name_en,
                slug,
                category_id,
                brand_id,
                supplier_id,
                weight,
                volume,
                description,
                description_en,
                specifications,
                main_image,
                images,
                status,
                is_featured,
                is_new,
                view_count,
                sales_count,
                notes,
                unit,
                created_at,
                updated_at,
                deleted_at
            FROM products
            WHERE id = ? AND deleted_at IS NULL"#
        )
        .bind(id)
        .fetch_optional(&mut **tx)
        .await?;

        Ok(product)
    }

    /// 获取指定时间后更新的产品列表（用于增量同步）
    pub async fn list_updated_since(&self, updated_after: &str, limit: u32) -> Result<Vec<Product>> {
        let products: Vec<Product> = sqlx::query_as(
            r#"SELECT
                id,
                product_code,
                name,
                model,
                name_en,
                slug,
                category_id,
                brand_id,
                supplier_id,
                weight,
                volume,
                description,
                description_en,
                specifications,
                main_image,
                images,
                status,
                is_featured,
                is_new,
                view_count,
                sales_count,
                notes,
                unit,
                created_at,
                updated_at,
                deleted_at
            FROM products
            WHERE updated_at > ? AND deleted_at IS NULL
            ORDER BY updated_at ASC
            LIMIT ?"#
        )
        .bind(updated_after)
        .bind(limit as i64)
        .fetch_all(self.pool)
        .await?;

        Ok(products)
    }
}
