//! 客户管理模板数据结构

/// 客户列表项
#[derive(Debug, Clone)]
pub struct CustomerItem {
    pub id: i64,
    pub customer_code: String,
    pub name: String,
    pub mobile: Option<String>,
    pub level_name: Option<String>,
    pub total_orders: i64,
    pub total_amount: f64,
    pub status: i64,
    pub source: String,
}
