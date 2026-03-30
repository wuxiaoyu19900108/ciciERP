//! 订单管理模板数据结构

/// 订单列表项
#[derive(Debug, Clone)]
pub struct OrderItem {
    pub id: i64,
    pub order_code: String,
    pub platform: String,
    pub customer_name: Option<String>,
    pub total_amount: f64,
    pub order_status: i64,
    pub payment_status: i64,
    pub created_at: String,
}

/// 新订单状态定义 - 以 PI/CI 流程为核心
/// 1 = 未成交 (草稿) - 可下载 PI
/// 2 = 价格锁定 - 客户确认价格 - 可下载 PI
/// 3 = 已付款 - 可下载 CI
/// 4 = 已发货 - 可下载 CI + 发货单
/// 5 = 已收货 - 可下载 CI
/// 6 = 已取消

/// 获取订单状态文本
pub fn order_status_text(status: i64) -> &'static str {
    match status {
        1 => "未成交",
        2 => "价格锁定",
        3 => "已付款",
        4 => "已发货",
        5 => "已收货",
        6 => "已取消",
        _ => "未知",
    }
}

/// 获取订单状态样式
pub fn order_status_class(status: i64) -> &'static str {
    match status {
        1 => "bg-gray-100 text-gray-700",
        2 => "bg-blue-100 text-blue-700",
        3 => "bg-green-100 text-green-700",
        4 => "bg-indigo-100 text-indigo-700",
        5 => "bg-emerald-100 text-emerald-700",
        6 => "bg-red-100 text-red-700",
        _ => "bg-gray-100 text-gray-600",
    }
}

/// 判断是否可以下载 PI
pub fn can_download_pi(status: i64) -> bool {
    matches!(status, 1 | 2)
}

/// 判断是否可以下载 CI
pub fn can_download_ci(status: i64) -> bool {
    matches!(status, 3 | 4 | 5)
}

/// 获取支付状态文本
pub fn payment_status_text(status: i64) -> &'static str {
    match status {
        1 => "未支付",
        2 => "部分支付",
        3 => "已支付",
        4 => "已退款",
        5 => "部分退款",
        _ => "未知",
    }
}

/// 获取支付状态样式
pub fn payment_status_class(status: i64) -> &'static str {
    match status {
        1 => "text-gray-500",
        2 => "text-yellow-600",
        3 => "text-green-600",
        4 => "text-red-600",
        5 => "text-orange-600",
        _ => "text-gray-500",
    }
}

/// 获取平台名称
pub fn platform_text(platform: &str) -> &str {
    match platform {
        "ali" => "阿里国际站",
        "ae" => "速卖通",
        "manual" => "手动创建",
        _ => platform,
    }
}
