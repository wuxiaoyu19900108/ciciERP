//! 供应商管理模板数据结构

/// 供应商列表项
#[derive(Debug, Clone)]
pub struct SupplierItem {
    pub id: i64,
    pub supplier_code: String,
    pub name: String,
    pub contact_person: Option<String>,
    pub contact_phone: Option<String>,
    pub rating_level: String,
    pub total_orders: i64,
    pub total_amount: f64,
    pub status: i64,
}

/// 获取评级样式
pub fn rating_class(level: &str) -> &'static str {
    match level {
        "A" | "A+" => "text-green-600 bg-green-100",
        "B" | "B+" => "text-blue-600 bg-blue-100",
        "C" => "text-yellow-600 bg-yellow-100",
        _ => "text-gray-600 bg-gray-100",
    }
}
