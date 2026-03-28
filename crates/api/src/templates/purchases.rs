//! 采购管理模板数据结构

/// 采购列表项
#[derive(Debug, Clone)]
pub struct PurchaseItem {
    pub id: i64,
    pub order_code: String,
    pub total_amount: f64,
    pub status: i64,
    pub payment_status: i64,
    pub delivery_status: i64,
    pub item_count: i64,
    pub created_at: String,
}

/// 获取采购单状态文本
pub fn purchase_status_text(status: i64) -> &'static str {
    match status {
        1 => "草稿",
        2 => "待审核",
        3 => "已审核",
        4 => "部分入库",
        5 => "已完成",
        6 => "已取消",
        _ => "未知",
    }
}

/// 获取采购单状态样式
pub fn purchase_status_class(status: i64) -> &'static str {
    match status {
        1 => "bg-gray-100 text-gray-700",
        2 => "bg-yellow-100 text-yellow-700",
        3 => "bg-blue-100 text-blue-700",
        4 => "bg-purple-100 text-purple-700",
        5 => "bg-green-100 text-green-700",
        6 => "bg-red-100 text-red-600",
        _ => "bg-gray-100 text-gray-600",
    }
}

/// 获取付款状态文本
pub fn payment_status_text(status: i64) -> &'static str {
    match status {
        1 => "未付款",
        2 => "部分付款",
        3 => "已付款",
        _ => "未知",
    }
}

/// 获取付款状态样式
pub fn payment_status_class(status: i64) -> &'static str {
    match status {
        1 => "text-gray-500",
        2 => "text-yellow-600",
        3 => "text-green-600",
        _ => "text-gray-500",
    }
}

/// 获取交货状态文本
pub fn delivery_status_text(status: i64) -> &'static str {
    match status {
        1 => "未收货",
        2 => "部分收货",
        3 => "已收货",
        _ => "未知",
    }
}

/// 获取交货状态样式
pub fn delivery_status_class(status: i64) -> &'static str {
    match status {
        1 => "text-gray-500",
        2 => "text-yellow-600",
        3 => "text-green-600",
        _ => "text-gray-500",
    }
}
