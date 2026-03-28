//! 仪表板模板数据结构

/// 仪表板统计数据
#[derive(Debug, Clone, Default)]
pub struct DashboardStats {
    pub total_orders: i64,
    pub pending_orders: i64,
    pub today_sales: f64,
    pub low_stock_count: i64,
    pub total_customers: i64,
    pub total_products: i64,
}
