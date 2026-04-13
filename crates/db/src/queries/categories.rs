//! 分类相关数据库查询

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

pub struct CategoryQueries<'a> {
    pool: &'a SqlitePool,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub slug: String,
}

impl<'a> CategoryQueries<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    /// 搜索分类（关键字匹配 name）
    pub async fn search(&self, keyword: Option<&str>, limit: u32) -> Result<Vec<Category>> {
        let items: Vec<Category> = if let Some(kw) = keyword.filter(|s| !s.is_empty()) {
            sqlx::query_as(
                "SELECT id, name, slug FROM categories WHERE deleted_at IS NULL AND name LIKE ? ORDER BY sort_order, name LIMIT ?"
            )
            .bind(format!("%{}%", kw))
            .bind(limit as i64)
            .fetch_all(self.pool)
            .await?
        } else {
            sqlx::query_as(
                "SELECT id, name, slug FROM categories WHERE deleted_at IS NULL ORDER BY sort_order, name LIMIT ?"
            )
            .bind(limit as i64)
            .fetch_all(self.pool)
            .await?
        };
        Ok(items)
    }

    /// 根据名称精确查找分类（忽略大小写）
    pub async fn find_by_name(&self, name: &str) -> Result<Option<Category>> {
        let item: Option<Category> = sqlx::query_as(
            "SELECT id, name, slug FROM categories WHERE deleted_at IS NULL AND LOWER(name) = LOWER(?)"
        )
        .bind(name)
        .fetch_optional(self.pool)
        .await?;
        Ok(item)
    }

    /// 创建分类，若同名已存在则直接返回已有记录
    pub async fn find_or_create(&self, name: &str) -> Result<Category> {
        if let Some(c) = self.find_by_name(name).await? {
            return Ok(c);
        }
        let slug_base: String = name.chars()
            .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
            .collect::<String>()
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-");
        let slug = {
            let existing: Option<(i64,)> = sqlx::query_as(
                "SELECT id FROM categories WHERE slug = ?"
            )
            .bind(&slug_base)
            .fetch_optional(self.pool)
            .await?;
            if existing.is_none() {
                slug_base.clone()
            } else {
                format!("{}-{}", slug_base, chrono::Utc::now().timestamp())
            }
        };
        let now = chrono::Utc::now().to_rfc3339();
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO categories (name, slug, created_at, updated_at) VALUES (?, ?, ?, ?) RETURNING id"
        )
        .bind(name)
        .bind(&slug)
        .bind(&now)
        .bind(&now)
        .fetch_one(self.pool)
        .await?;
        Ok(Category { id, name: name.to_string(), slug })
    }
}
