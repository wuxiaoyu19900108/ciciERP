//! 产品管理模板数据结构

/// 产品列表项
#[derive(Debug, Clone)]
pub struct ProductItem {
    pub id: i64,
    pub product_code: String,
    pub name: String,
    pub category_name: Option<String>,
    pub reference_cost_cny: Option<f64>,  // 参考成本
    pub reference_price_cny: Option<f64>,  // 参考售价
    pub stock_quantity: i64,
    pub status: i64,
}
