//! 品牌相关数据库查询

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

pub struct BrandQueries<'a> {
    pool: &'a SqlitePool,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Brand {
    pub id: i64,
    pub name: String,
    pub name_en: Option<String>,
    pub slug: String,
}

impl<'a> BrandQueries<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    /// 搜索品牌（关键字匹配 name）
    pub async fn search(&self, keyword: Option<&str>, limit: u32) -> Result<Vec<Brand>> {
        let items: Vec<Brand> = if let Some(kw) = keyword.filter(|s| !s.is_empty()) {
            sqlx::query_as(
                "SELECT id, name, name_en, slug FROM brands WHERE name LIKE ? ORDER BY name LIMIT ?"
            )
            .bind(format!("%{}%", kw))
            .bind(limit as i64)
            .fetch_all(self.pool)
            .await?
        } else {
            sqlx::query_as(
                "SELECT id, name, name_en, slug FROM brands ORDER BY name LIMIT ?"
            )
            .bind(limit as i64)
            .fetch_all(self.pool)
            .await?
        };
        Ok(items)
    }

    /// 根据名称精确查找品牌（忽略大小写）
    pub async fn find_by_name(&self, name: &str) -> Result<Option<Brand>> {
        let item: Option<Brand> = sqlx::query_as(
            "SELECT id, name, name_en, slug FROM brands WHERE LOWER(name) = LOWER(?)"
        )
        .bind(name)
        .fetch_optional(self.pool)
        .await?;
        Ok(item)
    }

    /// 创建品牌，若同名已存在则直接返回已有记录
    pub async fn find_or_create(&self, name: &str) -> Result<Brand> {
        // 先精确查找
        if let Some(b) = self.find_by_name(name).await? {
            return Ok(b);
        }
        // slug：小写 + 非字母数字替换为 -
        let slug_base: String = name.chars()
            .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
            .collect::<String>()
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-");
        // 确保 slug 唯一
        let slug = {
            let existing: Option<(i64,)> = sqlx::query_as(
                "SELECT id FROM brands WHERE slug = ?"
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
            "INSERT INTO brands (name, slug, status, created_at, updated_at) VALUES (?, ?, 1, ?, ?) RETURNING id"
        )
        .bind(name)
        .bind(&slug)
        .bind(&now)
        .bind(&now)
        .fetch_one(self.pool)
        .await?;
        Ok(Brand { id, name: name.to_string(), name_en: None, slug })
    }
}
