//! 库存管理模板数据结构

/// 库存项
#[derive(Debug, Clone)]
pub struct InventoryItem {
    pub id: i64,
    pub sku_code: String,
    pub product_name: String,
    pub spec_values: Option<String>,
    pub total_quantity: i64,
    pub available_quantity: i64,
    pub locked_quantity: i64,
    pub safety_stock: i64,
    pub is_low_stock: bool,
}

/// 获取库存状态样式
pub fn stock_status_class(item: &InventoryItem) -> &'static str {
    if item.available_quantity <= 0 {
        "bg-red-100 text-red-700"
    } else if item.is_low_stock {
        "bg-yellow-100 text-yellow-700"
    } else {
        "bg-green-100 text-green-700"
    }
}

/// 获取库存状态文本
pub fn stock_status_text(item: &InventoryItem) -> &'static str {
    if item.available_quantity <= 0 {
        "缺货"
    } else if item.is_low_stock {
        "低库存"
    } else {
        "正常"
    }
}
