//! 客户相关数据库查询

use anyhow::Result;
use serde::Deserialize;
use sqlx::{QueryBuilder, SqlitePool};

use cicierp_models::{
    customer::{CreateCustomerRequest, Customer, CustomerAddress, UpdateCustomerRequest},
    common::PagedResponse,
};

pub struct CustomerQueries<'a> {
    pool: &'a SqlitePool,
}

impl<'a> CustomerQueries<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    /// 生成客户编码 CUS-YYYYMMDD-XXXX
    async fn generate_customer_code(&self) -> Result<String> {
        let today = chrono::Utc::now().format("%Y%m%d").to_string();
        let prefix = format!("CUS-{}-", today);

        // 查询今天已有的编码数量
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM customers WHERE customer_code LIKE ?"
        )
        .bind(format!("{}%", prefix))
        .fetch_one(self.pool)
        .await?;

        let seq = count.0 + 1;
        Ok(format!("{}{:04}", prefix, seq))
    }

    /// 获取客户列表（使用安全的参数化查询）
    pub async fn list(
        &self,
        page: u32,
        page_size: u32,
        level_id: Option<i64>,
        status: Option<i64>,
        source: Option<&str>,
        keyword: Option<&str>,
    ) -> Result<PagedResponse<Customer>> {
        let offset = (page.saturating_sub(1)) * page_size;

        // 构建安全的 count 查询
        let mut count_query = QueryBuilder::new("SELECT COUNT(*) FROM customers WHERE deleted_at IS NULL");

        if let Some(lid) = level_id {
            count_query.push(" AND level_id = ");
            count_query.push_bind(lid);
        }
        if let Some(s) = status {
            count_query.push(" AND status = ");
            count_query.push_bind(s);
        }
        if let Some(src) = source {
            count_query.push(" AND source = ");
            count_query.push_bind(src);
        }
        if let Some(kw) = keyword {
            count_query.push(" AND (name LIKE ");
            count_query.push_bind(format!("%{}%", kw));
            count_query.push(" OR mobile LIKE ");
            count_query.push_bind(format!("%{}%", kw));
            count_query.push(" OR customer_code LIKE ");
            count_query.push_bind(format!("%{}%", kw));
            count_query.push(")");
        }

        let total: (i64,) = count_query.build_query_as()
            .fetch_one(self.pool)
            .await?;

        // 构建安全的 list 查询
        let mut list_query = QueryBuilder::new("SELECT * FROM customers WHERE deleted_at IS NULL");

        if let Some(lid) = level_id {
            list_query.push(" AND level_id = ");
            list_query.push_bind(lid);
        }
        if let Some(s) = status {
            list_query.push(" AND status = ");
            list_query.push_bind(s);
        }
        if let Some(src) = source {
            list_query.push(" AND source = ");
            list_query.push_bind(src);
        }
        if let Some(kw) = keyword {
            list_query.push(" AND (name LIKE ");
            list_query.push_bind(format!("%{}%", kw));
            list_query.push(" OR mobile LIKE ");
            list_query.push_bind(format!("%{}%", kw));
            list_query.push(" OR customer_code LIKE ");
            list_query.push_bind(format!("%{}%", kw));
            list_query.push(")");
        }

        list_query.push(" ORDER BY created_at DESC LIMIT ");
        list_query.push_bind(page_size as i64);
        list_query.push(" OFFSET ");
        list_query.push_bind(offset as i64);

        let items: Vec<Customer> = list_query.build_query_as()
            .fetch_all(self.pool)
            .await?;

        Ok(PagedResponse::new(items, page, page_size, total.0 as u64))
    }

    /// 根据 ID 获取客户
    pub async fn get_by_id(&self, id: i64) -> Result<Option<Customer>> {
        let customer: Option<Customer> = sqlx::query_as(
            "SELECT * FROM customers WHERE id = ? AND deleted_at IS NULL"
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await?;

        Ok(customer)
    }

    /// 根据手机号获取客户
    pub async fn get_by_mobile(&self, mobile: &str) -> Result<Option<Customer>> {
        let customer: Option<Customer> = sqlx::query_as(
            "SELECT * FROM customers WHERE mobile = ? AND deleted_at IS NULL"
        )
        .bind(mobile)
        .fetch_optional(self.pool)
        .await?;

        Ok(customer)
    }

    /// 创建客户（简化版）
    pub async fn create(&self, req: &CreateCustomerRequest) -> Result<Customer> {
        let now = chrono::Utc::now().to_rfc3339();
        let customer_code = self.generate_customer_code().await?;
        let source = req.source.clone().unwrap_or_else(|| "manual".to_string());
        let status = req.status.unwrap_or(1);

        let result = sqlx::query(
            r#"
            INSERT INTO customers (
                customer_code, name, mobile, email, status, notes, source,
                created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&customer_code)
        .bind(&req.name)
        .bind(&req.mobile)
        .bind(&req.email)
        .bind(status)
        .bind(&req.notes)
        .bind(&source)
        .bind(&now)
        .bind(&now)
        .execute(self.pool)
        .await?;

        let id = result.last_insert_rowid();
        self.get_by_id(id).await?.ok_or_else(|| anyhow::anyhow!("Failed to fetch created customer"))
    }

    /// 更新客户（简化版）
    pub async fn update(&self, id: i64, req: &UpdateCustomerRequest) -> Result<Option<Customer>> {
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
        if let Some(ref mobile) = req.mobile {
            updates.push("mobile = ?");
            bindings.push(mobile.clone());
        }
        if let Some(ref email) = req.email {
            updates.push("email = ?");
            bindings.push(email.clone());
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
            "UPDATE customers SET {} WHERE id = ? AND deleted_at IS NULL",
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

    /// 软删除客户
    pub async fn delete(&self, id: i64) -> Result<bool> {
        let now = chrono::Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE customers SET deleted_at = ? WHERE id = ? AND deleted_at IS NULL"
        )
        .bind(&now)
        .bind(id)
        .execute(self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// 获取客户地址列表
    pub async fn get_addresses(&self, customer_id: i64) -> Result<Vec<CustomerAddress>> {
        let addresses: Vec<CustomerAddress> = sqlx::query_as(
            "SELECT * FROM customer_addresses WHERE customer_id = ? ORDER BY is_default DESC, created_at DESC"
        )
        .bind(customer_id)
        .fetch_all(self.pool)
        .await?;

        Ok(addresses)
    }

    /// 创建客户地址
    pub async fn create_address(&self, customer_id: i64, req: &CreateAddressRequest) -> Result<CustomerAddress> {
        let now = chrono::Utc::now().to_rfc3339();

        // 如果设置为默认地址，先清除其他默认地址
        if req.is_default {
            sqlx::query("UPDATE customer_addresses SET is_default = 0 WHERE customer_id = ?")
                .bind(customer_id)
                .execute(self.pool)
                .await?;
        }

        let result = sqlx::query(
            r#"
            INSERT INTO customer_addresses (
                customer_id, receiver_name, receiver_phone, country, country_code,
                province, city, district, address, postal_code, address_type, is_default,
                created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(customer_id)
        .bind(&req.receiver_name)
        .bind(&req.receiver_phone)
        .bind(&req.country)
        .bind(&req.country_code)
        .bind(&req.province)
        .bind(&req.city)
        .bind(&req.district)
        .bind(&req.address)
        .bind(&req.postal_code)
        .bind(req.address_type.unwrap_or(1))
        .bind(req.is_default)
        .bind(&now)
        .bind(&now)
        .execute(self.pool)
        .await?;

        let id = result.last_insert_rowid();
        self.get_address_by_id(id).await?.ok_or_else(|| anyhow::anyhow!("Failed to fetch created address"))
    }

    /// 根据ID获取地址
    pub async fn get_address_by_id(&self, id: i64) -> Result<Option<CustomerAddress>> {
        let address: Option<CustomerAddress> = sqlx::query_as(
            "SELECT * FROM customer_addresses WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await?;

        Ok(address)
    }

    /// 删除客户地址
    pub async fn delete_address(&self, customer_id: i64, address_id: i64) -> Result<bool> {
        let result = sqlx::query(
            "DELETE FROM customer_addresses WHERE id = ? AND customer_id = ?"
        )
        .bind(address_id)
        .bind(customer_id)
        .execute(self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// 设置默认地址
    pub async fn set_default_address(&self, customer_id: i64, address_id: i64) -> Result<bool> {
        // 先清除其他默认地址
        sqlx::query("UPDATE customer_addresses SET is_default = 0 WHERE customer_id = ?")
            .bind(customer_id)
            .execute(self.pool)
            .await?;

        // 设置新的默认地址
        let result = sqlx::query(
            "UPDATE customer_addresses SET is_default = 1 WHERE id = ? AND customer_id = ?"
        )
        .bind(address_id)
        .bind(customer_id)
        .execute(self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }
}

/// 创建地址请求
#[derive(Debug, Deserialize)]
pub struct CreateAddressRequest {
    pub receiver_name: String,
    pub receiver_phone: String,
    pub country: String,
    pub country_code: Option<String>,
    pub province: Option<String>,
    pub city: Option<String>,
    pub district: Option<String>,
    pub address: String,
    pub postal_code: Option<String>,
    pub address_type: Option<i64>,
    pub is_default: bool,
}
