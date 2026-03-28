//! 通用类型和响应结构

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 分页信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pagination {
    pub page: u32,
    pub page_size: u32,
    pub total: u64,
    pub total_pages: u32,
}

impl Pagination {
    pub fn new(page: u32, page_size: u32, total: u64) -> Self {
        let total_pages = if page_size > 0 {
            ((total as f64) / (page_size as f64)).ceil() as u32
        } else {
            0
        };
        Self {
            page,
            page_size,
            total,
            total_pages,
        }
    }

    /// 计算数据库查询的 OFFSET
    pub fn offset(&self) -> u32 {
        self.page.saturating_sub(1) * self.page_size
    }
}

/// 分页响应
#[derive(Debug, Serialize, Deserialize)]
pub struct PagedResponse<T> {
    pub items: Vec<T>,
    pub pagination: Pagination,
}

impl<T> PagedResponse<T> {
    pub fn new(items: Vec<T>, page: u32, page_size: u32, total: u64) -> Self {
        Self {
            items,
            pagination: Pagination::new(page, page_size, total),
        }
    }
}

/// 列表查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct ListQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

impl Default for ListQuery {
    fn default() -> Self {
        Self {
            page: Some(1),
            page_size: Some(20),
        }
    }
}

impl ListQuery {
    pub fn page(&self) -> u32 {
        self.page.unwrap_or(1).max(1)
    }

    pub fn page_size(&self) -> u32 {
        self.page_size.unwrap_or(20).min(100).max(1)
    }
}

/// 基础状态枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(rename_all = "lowercase")]
pub enum Status {
    #[serde(rename = "1")]
    Active = 1,
    #[serde(rename = "2")]
    Inactive = 2,
}

impl Default for Status {
    fn default() -> Self {
        Status::Active
    }
}

/// 性别枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
pub enum Gender {
    #[serde(rename = "1")]
    Male = 1,
    #[serde(rename = "2")]
    Female = 2,
}

/// ID 路径参数
#[derive(Debug, Deserialize)]
pub struct PathId {
    pub id: i64,
}
