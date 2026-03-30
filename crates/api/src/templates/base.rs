//! 基础布局模板数据结构

/// 用户信息（用于导航栏显示）
#[derive(Debug, Clone)]
pub struct UserInfo {
    pub id: i64,
    pub username: String,
    pub real_name: Option<String>,
    pub avatar: Option<String>,
}

impl UserInfo {
    pub fn display_name(&self) -> &str {
        self.real_name.as_deref().unwrap_or(&self.username)
    }
}

/// 菜单项
#[derive(Debug, Clone)]
pub struct MenuItem {
    pub code: &'static str,
    pub icon: &'static str,
    pub label: &'static str,
    pub href: &'static str,
}

/// 获取侧边栏菜单
pub fn get_menus() -> Vec<MenuItem> {
    vec![
        MenuItem {
            code: "dashboard",
            icon: "📊",
            label: "仪表板",
            href: "/",
        },
        MenuItem {
            code: "products",
            icon: "📦",
            label: "产品管理",
            href: "/products",
        },
        MenuItem {
            code: "orders",
            icon: "📋",
            label: "订单管理",
            href: "/orders",
        },
        MenuItem {
            code: "analytics",
            icon: "📈",
            label: "数据分析",
            href: "/analytics",
        },
        MenuItem {
            code: "inventory",
            icon: "📊",
            label: "库存管理",
            href: "/inventory",
        },
        MenuItem {
            code: "customers",
            icon: "👥",
            label: "客户管理",
            href: "/customers",
        },
        MenuItem {
            code: "suppliers",
            icon: "🏭",
            label: "供应商管理",
            href: "/suppliers",
        },
        MenuItem {
            code: "purchase",
            icon: "🛒",
            label: "采购管理",
            href: "/purchase",
        },
        MenuItem {
            code: "logistics",
            icon: "🚚",
            label: "物流管理",
            href: "/logistics",
        },
    ]
}
