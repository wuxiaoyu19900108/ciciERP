//! 产品内容相关数据库查询

use anyhow::Result;
use sqlx::SqlitePool;

use cicierp_models::product::{
    CreateProductContentRequest, ProductContent, UpdateProductContentRequest,
};

/// 产品内容查询结构体
pub struct ProductContentQueries<'a> {
    pool: &'a SqlitePool,
}

impl<'a> ProductContentQueries<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    /// 根据产品ID获取内容
    pub async fn get_by_product_id(&self, product_id: i64) -> Result<Option<ProductContent>> {
        let content: Option<ProductContent> = sqlx::query_as(
            "SELECT * FROM product_content WHERE product_id = ?"
        )
        .bind(product_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(content)
    }

    /// 根据ID获取内容
    pub async fn get_by_id(&self, id: i64) -> Result<Option<ProductContent>> {
        let content: Option<ProductContent> = sqlx::query_as(
            "SELECT * FROM product_content WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await?;

        Ok(content)
    }

    /// 创建产品内容
    pub async fn create(&self, req: &CreateProductContentRequest) -> Result<ProductContent> {
        let now = chrono::Utc::now().to_rfc3339();

        let result = sqlx::query(
            r#"
            INSERT INTO product_content (
                product_id, title_en, description, description_en,
                main_image, images, specifications,
                meta_title, meta_description, meta_keywords,
                content_html, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(req.product_id)
        .bind(&req.title_en)
        .bind(&req.description)
        .bind(&req.description_en)
        .bind(&req.main_image)
        .bind(&req.images)
        .bind(&req.specifications)
        .bind(&req.meta_title)
        .bind(&req.meta_description)
        .bind(&req.meta_keywords)
        .bind(&req.content_html)
        .bind(&now)
        .bind(&now)
        .execute(self.pool)
        .await?;

        let id = result.last_insert_rowid();
        self.get_by_id(id).await?.ok_or_else(|| anyhow::anyhow!("Failed to fetch created product content"))
    }

    /// 更新产品内容
    pub async fn update(&self, product_id: i64, req: &UpdateProductContentRequest) -> Result<Option<ProductContent>> {
        if self.get_by_product_id(product_id).await?.is_none() {
            return Ok(None);
        }

        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            UPDATE product_content SET
                title_en = COALESCE(?, title_en),
                description = COALESCE(?, description),
                description_en = COALESCE(?, description_en),
                main_image = COALESCE(?, main_image),
                images = COALESCE(?, images),
                specifications = COALESCE(?, specifications),
                meta_title = COALESCE(?, meta_title),
                meta_description = COALESCE(?, meta_description),
                meta_keywords = COALESCE(?, meta_keywords),
                content_html = COALESCE(?, content_html),
                updated_at = ?
            WHERE product_id = ?
            "#
        )
        .bind(&req.title_en)
        .bind(&req.description)
        .bind(&req.description_en)
        .bind(&req.main_image)
        .bind(&req.images)
        .bind(&req.specifications)
        .bind(&req.meta_title)
        .bind(&req.meta_description)
        .bind(&req.meta_keywords)
        .bind(&req.content_html)
        .bind(&now)
        .bind(product_id)
        .execute(self.pool)
        .await?;

        self.get_by_product_id(product_id).await
    }

    /// 创建或更新产品内容（upsert）
    pub async fn upsert(&self, product_id: i64, req: &UpdateProductContentRequest) -> Result<ProductContent> {
        if let Some(content) = self.get_by_product_id(product_id).await? {
            self.update(product_id, req).await?.ok_or_else(|| anyhow::anyhow!("Failed to update product content"))
        } else {
            let create_req = CreateProductContentRequest {
                product_id,
                title_en: req.title_en.clone(),
                description: req.description.clone(),
                description_en: req.description_en.clone(),
                main_image: req.main_image.clone(),
                images: req.images.clone(),
                specifications: req.specifications.clone(),
                meta_title: req.meta_title.clone(),
                meta_description: req.meta_description.clone(),
                meta_keywords: req.meta_keywords.clone(),
                content_html: req.content_html.clone(),
            };
            self.create(&create_req).await
        }
    }

    /// 删除产品内容
    pub async fn delete(&self, product_id: i64) -> Result<bool> {
        let result = sqlx::query("DELETE FROM product_content WHERE product_id = ?")
            .bind(product_id)
            .execute(self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }
}
