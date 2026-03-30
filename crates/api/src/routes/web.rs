//! Web 页面路由
//!
//! 提供 HTML 页面渲染

use axum::{
    extract::{Form, Path, Query, State},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Extension, Router,
};
use chrono::{Datelike, Local};
use serde::Deserialize;
use tracing::info;

use crate::middleware::auth::AuthUser;
use crate::state::AppState;
use crate::templates::base::{get_menus, UserInfo};
use crate::templates::dashboard::DashboardStats;
use cicierp_db::queries::{
    customers::{CustomerQueries, CreateAddressRequest},
    inventory::InventoryQueries,
    logistics::{LogisticsCompanyQueries, ShipmentQueries},
    orders::OrderQueries,
    product_content::ProductContentQueries,
    product_costs::ProductCostQueries,
    product_prices::ProductPriceQueries,
    products::ProductQueries,
    purchases::PurchaseQueries,
    suppliers::SupplierQueries,
    users::UserQueries,
};
use cicierp_models::auth::JwtConfig;
use cicierp_models::common::PagedResponse;
use cicierp_models::customer::{CreateCustomerRequest, CustomerAddress, UpdateCustomerRequest};
use cicierp_models::inventory::UpdateInventoryRequest;
use cicierp_models::order::{CreateOrderRequest, OrderItemRequest};
use cicierp_models::product::{CreateProductCostRequest, CreateProductPriceRequest, CreateProductRequest, UpdateProductRequest};
use cicierp_models::supplier::{CreateSupplierRequest, UpdateSupplierRequest};

/// 创建公开路由（无需认证）
pub fn public_router() -> Router<AppState> {
    Router::new()
        .route("/login", get(login_page).post(login_handler))
        .route("/logout", post(logout_handler))
}

/// 创建受保护路由（需要认证）
pub fn protected_router() -> Router<AppState> {
    Router::new()
        .route("/", get(dashboard_page))
        // 产品
        .route("/products", get(products_page))
        .route("/products/new", get(product_new_page).post(product_create_handler))
        .route("/products/:id", get(product_detail_page))
        .route("/products/:id/edit", get(product_edit_page).post(product_update_handler))
        // 订单
        .route("/orders", get(orders_page))
        .route("/orders/new", get(order_new_page).post(order_create_handler))
        .route("/orders/:id", get(order_detail_page))
        .route("/orders/:id/edit", get(order_edit_page).post(order_update_handler))
        // 库存
        .route("/inventory", get(inventory_page))
        .route("/inventory/new", get(inventory_new_page).post(inventory_create_handler))
        .route("/inventory/:id", get(inventory_detail_page))
        .route("/inventory/:id/adjust", get(inventory_adjust_page).post(inventory_adjust_handler))
        .route("/inventory/:id/movements", get(inventory_movements_page))
        // 客户
        .route("/customers", get(customers_page))
        .route("/customers/new", get(customer_new_page).post(customer_create_handler))
        .route("/customers/:id", get(customer_detail_page))
        .route("/customers/:id/edit", get(customer_edit_page).post(customer_update_handler))
        .route("/customers/:id/addresses", post(customer_address_add_handler))
        .route("/customers/:id/addresses/:address_id", post(customer_address_delete_handler))
        .route("/customers/:id/addresses/:address_id/edit", get(customer_address_edit_page))
        .route("/customers/:id/addresses/:address_id/update", post(customer_address_update_handler))
        .route("/customers/:id/addresses/:address_id/set-default", post(customer_address_set_default_handler))
        // 供应商
        .route("/suppliers", get(suppliers_page))
        .route("/suppliers/new", get(supplier_new_page).post(supplier_create_handler))
        .route("/suppliers/:id", get(supplier_detail_page))
        .route("/suppliers/:id/edit", get(supplier_edit_page).post(supplier_update_handler))
        // 采购
        .route("/purchase", get(purchase_page))
        .route("/purchase/new", get(purchase_new_page).post(purchase_create_handler))
        .route("/purchase/:id", get(purchase_detail_page))
        // 物流
        .route("/logistics", get(logistics_page))
        // 分析报告
        .route("/analytics", get(analytics_page))
        // PI/CI
        .merge(super::web_invoice::router())
}

/// 从请求扩展中获取用户信息
fn get_user_from_extension(auth_user: &AuthUser) -> UserInfo {
    UserInfo {
        id: auth_user.user_id,
        username: auth_user.username.clone(),
        real_name: None,
        avatar: None,
    }
}

/// 渲染带布局的页面
fn render_layout(title: &str, active_menu: &str, user: Option<UserInfo>, content: &str) -> Html<String> {
    // 使用简单的字符串拼接来生成 HTML
    let menus = get_menus();

    let menu_html: String = menus
        .iter()
        .map(|menu| {
            let class = if menu.code == active_menu {
                "bg-slate-700 text-white"
            } else {
                "text-slate-300 hover:bg-slate-700 hover:text-white"
            };
            format!(
                r#"<a href="{}" onclick="closeSidebar()" class="flex items-center gap-3 px-4 py-3 rounded-lg transition-colors {}">
                    <span>{}</span>
                    <span>{}</span>
                </a>"#,
                menu.href, class, menu.icon, menu.label
            )
        })
        .collect();

    let user_section = if let Some(ref u) = user {
        format!(
            r#"<div class="flex items-center gap-2 sm:gap-4">
                <span class="text-gray-600 text-sm sm:text-base hidden sm:inline">{}</span>
                <form action="/logout" method="POST">
                    <button type="submit" class="px-3 sm:px-4 py-2 text-sm text-gray-600 hover:text-gray-800 hover:bg-gray-100 rounded-lg transition-colors">
                        退出
                    </button>
                </form>
            </div>"#,
            u.display_name()
        )
    } else {
        String::new()
    };

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{} - ciciERP</title>
    <script src="https://cdn.tailwindcss.com"></script>
    <script src="https://unpkg.com/htmx.org@1.9.10"></script>
    <style>
        .htmx-request .htmx-indicator {{ display: inline-block; }}
        .htmx-request .htmx-hidden {{ display: none; }}
        .htmx-indicator {{ display: none; }}
        /* 侧边栏过渡动画 */
        #sidebar {{
            transition: transform 0.3s ease-in-out;
        }}
        #sidebar-overlay {{
            transition: opacity 0.3s ease-in-out;
        }}
    </style>
</head>
<body class="bg-gray-100 min-h-screen">
    <div class="flex h-screen">
        <!-- 移动端遮罩层 -->
        <div id="sidebar-overlay" class="fixed inset-0 bg-black bg-opacity-50 z-40 hidden lg:hidden" onclick="closeSidebar()"></div>

        <!-- 侧边栏 -->
        <aside id="sidebar" class="fixed lg:relative z-50 lg:z-auto w-64 bg-slate-800 text-white flex flex-col h-full -translate-x-full lg:translate-x-0">
            <div class="p-4 border-b border-slate-700">
                <h1 class="text-xl font-bold">📦 ciciERP</h1>
                <p class="text-slate-400 text-sm">企业资源管理系统</p>
            </div>
            <nav class="flex-1 p-4 overflow-y-auto">
                <ul class="space-y-2">{}</ul>
            </nav>
            <div class="p-4 border-t border-slate-700 text-slate-400 text-sm">
                <p>ciciERP v0.1.0</p>
            </div>
        </aside>

        <!-- 主内容区 -->
        <div class="flex-1 flex flex-col overflow-hidden w-full">
            <header class="bg-white shadow-sm border-b border-gray-200">
                <div class="flex items-center justify-between px-4 sm:px-6 py-4">
                    <div class="flex items-center gap-3">
                        <!-- 汉堡菜单按钮（仅移动端显示） -->
                        <button id="menu-toggle" onclick="toggleSidebar()" class="lg:hidden p-2 rounded-lg hover:bg-gray-100 transition-colors">
                            <svg class="w-6 h-6 text-gray-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h16M4 18h16"></path>
                            </svg>
                        </button>
                        <h2 class="text-base sm:text-lg font-semibold text-gray-800">{}</h2>
                    </div>
                    {}
                </div>
            </header>
            <main class="flex-1 overflow-auto p-4 sm:p-6">{}</main>
        </div>
    </div>
    <div id="toast" class="fixed top-4 right-4 z-50 hidden">
        <div class="bg-green-500 text-white px-6 py-3 rounded-lg shadow-lg">
            <span id="toast-message"></span>
        </div>
    </div>
    <script>
        function toggleSidebar() {{
            const sidebar = document.getElementById('sidebar');
            const overlay = document.getElementById('sidebar-overlay');
            sidebar.classList.toggle('-translate-x-full');
            sidebar.classList.toggle('translate-x-0');
            overlay.classList.toggle('hidden');
        }}

        function closeSidebar() {{
            const sidebar = document.getElementById('sidebar');
            const overlay = document.getElementById('sidebar-overlay');
            sidebar.classList.add('-translate-x-full');
            sidebar.classList.remove('translate-x-0');
            overlay.classList.add('hidden');
        }}

        function showToast(message, type = 'success') {{
            const toast = document.getElementById('toast');
            const messageEl = document.getElementById('toast-message');
            const container = toast.querySelector('div');
            messageEl.textContent = message;
            container.className = 'px-6 py-3 rounded-lg shadow-lg text-white ' +
                (type === 'success' ? 'bg-green-500' : type === 'error' ? 'bg-red-500' : 'bg-blue-500');
            toast.classList.remove('hidden');
            setTimeout(() => toast.classList.add('hidden'), 3000);
        }}
    </script>
</body>
</html>"#,
        title, menu_html, title, user_section, content
    );

    Html(html)
}

/// 渲染空布局页面（用于登录）
fn render_empty_layout(title: &str, content: &str) -> Html<String> {
    let html = format!(
        r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{} - ciciERP</title>
    <script src="https://cdn.tailwindcss.com"></script>
</head>
<body class="bg-gradient-to-br from-slate-800 to-slate-900 min-h-screen flex items-center justify-center">
    {}
</body>
</html>"#,
        title, content
    );
    Html(html)
}

// ============================================================================
// 登录相关
// ============================================================================

/// 登录页面
pub async fn login_page() -> Html<String> {
    let content = r#"<div class="w-full max-w-md">
    <div class="bg-white rounded-2xl shadow-xl p-8">
        <div class="text-center mb-8">
            <h1 class="text-3xl font-bold text-slate-800">📦 ciciERP</h1>
            <p class="text-slate-500 mt-2">企业资源管理系统</p>
        </div>
        <form action="/login" method="POST" class="space-y-6">
            <div>
                <label for="username" class="block text-sm font-medium text-gray-700 mb-2">用户名</label>
                <input type="text" id="username" name="username" required autofocus
                       class="w-full px-4 py-3 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent transition-colors"
                       placeholder="请输入用户名">
            </div>
            <div>
                <label for="password" class="block text-sm font-medium text-gray-700 mb-2">密码</label>
                <input type="password" id="password" name="password" required
                       class="w-full px-4 py-3 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent transition-colors"
                       placeholder="请输入密码">
            </div>
            <button type="submit"
                    class="w-full bg-blue-600 text-white py-3 px-4 rounded-lg font-medium hover:bg-blue-700 focus:ring-4 focus:ring-blue-200 transition-colors">
                登 录
            </button>
        </form>
        <div class="mt-6 text-center text-sm text-slate-500">
            <p>默认管理员: admin / admin123</p>
        </div>
    </div>
</div>"#;

    render_empty_layout("登录", content)
}

/// 登录表单
#[derive(Debug, Deserialize)]
pub struct LoginForm {
    username: String,
    password: String,
}

/// 登录处理
pub async fn login_handler(
    State(state): State<AppState>,
    Form(form): Form<LoginForm>,
) -> Result<impl IntoResponse, Html<String>> {
    info!("Login attempt: {}", form.username);

    let queries = UserQueries::new(state.db.pool());

    // 查找用户
    let user = match queries.get_by_username(&form.username).await {
        Ok(Some(u)) if u.status == 1 => u,
        Ok(Some(_)) => {
            return Err(render_login_error("账户已被禁用"));
        }
        _ => {
            return Err(render_login_error("用户名或密码错误"));
        }
    };

    // 验证密码（简化处理，实际应使用 argon2）
    // 这里需要与 auth.rs 中的密码验证逻辑一致
    use argon2::{
        password_hash::{PasswordHash, PasswordVerifier},
        Argon2,
    };

    let parsed_hash = match PasswordHash::new(&user.password_hash) {
        Ok(h) => h,
        Err(_) => return Err(render_login_error("系统错误")),
    };

    if Argon2::default()
        .verify_password(form.password.as_bytes(), &parsed_hash)
        .is_err()
    {
        return Err(render_login_error("用户名或密码错误"));
    }

    // 生成 token 并设置 cookie
    let roles = queries.get_user_roles(user.id).await.unwrap_or_default();
    let permissions = queries.get_user_permissions(user.id).await.unwrap_or_default();

    let role_codes: Vec<String> = roles.iter().map(|r| r.code.clone()).collect();

    let config = JwtConfig::from_env();
    let token = match crate::middleware::auth::generate_token(
        user.id,
        &user.username,
        role_codes,
        permissions,
        &config,
    ) {
        Ok(t) => t,
        Err(_) => return Err(render_login_error("系统错误")),
    };

    // 更新最后登录时间
    let _ = queries.update_last_login(user.id, None).await;

    info!("User logged in: {}", user.username);

    // 设置 cookie 并重定向
    Ok((
        [("Set-Cookie", format!("auth_token={}; Path=/; HttpOnly; Max-Age={}", token, config.expires_in))],
        Redirect::to("/"),
    ))
}

fn render_login_error(error: &str) -> Html<String> {
    let content = format!(
        r#"<div class="w-full max-w-md">
    <div class="bg-white rounded-2xl shadow-xl p-8">
        <div class="text-center mb-8">
            <h1 class="text-3xl font-bold text-slate-800">📦 ciciERP</h1>
            <p class="text-slate-500 mt-2">企业资源管理系统</p>
        </div>
        <div class="mb-6 p-4 bg-red-50 border border-red-200 rounded-lg">
            <p class="text-red-600 text-sm">{}</p>
        </div>
        <form action="/login" method="POST" class="space-y-6">
            <div>
                <label for="username" class="block text-sm font-medium text-gray-700 mb-2">用户名</label>
                <input type="text" id="username" name="username" required autofocus
                       class="w-full px-4 py-3 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent transition-colors"
                       placeholder="请输入用户名">
            </div>
            <div>
                <label for="password" class="block text-sm font-medium text-gray-700 mb-2">密码</label>
                <input type="password" id="password" name="password" required
                       class="w-full px-4 py-3 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent transition-colors"
                       placeholder="请输入密码">
            </div>
            <button type="submit"
                    class="w-full bg-blue-600 text-white py-3 px-4 rounded-lg font-medium hover:bg-blue-700 focus:ring-4 focus:ring-blue-200 transition-colors">
                登 录
            </button>
        </form>
        <div class="mt-6 text-center text-sm text-slate-500">
            <p>默认管理员: admin / admin123</p>
        </div>
    </div>
</div>"#,
        error
    );
    render_empty_layout("登录", &content)
}

/// 登出处理
pub async fn logout_handler() -> impl IntoResponse {
    (
        [("Set-Cookie", "auth_token=; Path=/; HttpOnly; Max-Age=0")],
        Redirect::to("/login"),
    )
}

// ============================================================================
// 仪表板
// ============================================================================

/// 仪表板页面
pub async fn dashboard_page(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
) -> Html<String> {
    let user = get_user_from_extension(&auth_user);

    // 获取统计数据
    let product_queries = ProductQueries::new(state.db.pool());
    let order_queries = OrderQueries::new(state.db.pool());

    let total_products = product_queries.count().await.unwrap_or(0);
    let total_orders = order_queries.count().await.unwrap_or(0);

    let stats = DashboardStats {
        total_orders,
        pending_orders: 0,
        today_sales: 0.0,
        low_stock_count: 0,
        total_customers: 0,
        total_products,
    };

    let content = format!(
        r#"<!-- 欢迎区域 -->
<div class="mb-6 sm:mb-8">
    <h1 class="text-xl sm:text-2xl font-bold text-gray-800">欢迎回来!</h1>
    <p class="text-gray-600 mt-1 text-sm sm:text-base">这是您的业务概览</p>
</div>

<!-- 统计卡片 -->
<div class="grid grid-cols-2 lg:grid-cols-4 gap-3 sm:gap-6 mb-6 sm:mb-8">
    <div class="bg-white rounded-xl shadow-sm p-4 sm:p-6 border border-gray-100">
        <div class="flex items-center justify-between">
            <div>
                <p class="text-gray-500 text-xs sm:text-sm">订单总数</p>
                <p class="text-2xl sm:text-3xl font-bold text-gray-800 mt-1">{}</p>
            </div>
            <div class="w-10 h-10 sm:w-12 sm:h-12 bg-blue-100 rounded-lg flex items-center justify-center">
                <span class="text-xl sm:text-2xl">📋</span>
            </div>
        </div>
    </div>

    <div class="bg-white rounded-xl shadow-sm p-4 sm:p-6 border border-gray-100">
        <div class="flex items-center justify-between">
            <div>
                <p class="text-gray-500 text-xs sm:text-sm">今日销售额</p>
                <p class="text-2xl sm:text-3xl font-bold text-gray-800 mt-1">¥{:.2}</p>
            </div>
            <div class="w-10 h-10 sm:w-12 sm:h-12 bg-green-100 rounded-lg flex items-center justify-center">
                <span class="text-xl sm:text-2xl">💰</span>
            </div>
        </div>
    </div>

    <div class="bg-white rounded-xl shadow-sm p-4 sm:p-6 border border-gray-100">
        <div class="flex items-center justify-between">
            <div>
                <p class="text-gray-500 text-xs sm:text-sm">库存预警</p>
                <p class="text-2xl sm:text-3xl font-bold text-gray-800 mt-1">{}</p>
            </div>
            <div class="w-10 h-10 sm:w-12 sm:h-12 bg-yellow-100 rounded-lg flex items-center justify-center">
                <span class="text-xl sm:text-2xl">⚠️</span>
            </div>
        </div>
    </div>

    <div class="bg-white rounded-xl shadow-sm p-4 sm:p-6 border border-gray-100">
        <div class="flex items-center justify-between">
            <div>
                <p class="text-gray-500 text-xs sm:text-sm">产品总数</p>
                <p class="text-2xl sm:text-3xl font-bold text-gray-800 mt-1">{}</p>
            </div>
            <div class="w-10 h-10 sm:w-12 sm:h-12 bg-purple-100 rounded-lg flex items-center justify-center">
                <span class="text-xl sm:text-2xl">📦</span>
            </div>
        </div>
    </div>
</div>

<!-- 快捷入口 -->
<div class="bg-white rounded-xl shadow-sm border border-gray-100">
    <div class="p-4 sm:p-6 border-b border-gray-100">
        <h3 class="text-base sm:text-lg font-semibold text-gray-800">快捷操作</h3>
    </div>
    <div class="p-4 sm:p-6">
        <div class="grid grid-cols-1 sm:grid-cols-2 gap-3 sm:gap-4">
            <a href="/products" class="flex items-center gap-3 p-3 sm:p-4 bg-blue-50 rounded-lg hover:bg-blue-100 transition-colors">
                <span class="text-xl sm:text-2xl">📦</span>
                <div>
                    <p class="font-medium text-gray-800 text-sm sm:text-base">产品管理</p>
                    <p class="text-xs sm:text-sm text-gray-500">管理产品信息</p>
                </div>
            </a>
            <a href="/orders" class="flex items-center gap-3 p-3 sm:p-4 bg-green-50 rounded-lg hover:bg-green-100 transition-colors">
                <span class="text-xl sm:text-2xl">📋</span>
                <div>
                    <p class="font-medium text-gray-800 text-sm sm:text-base">订单管理</p>
                    <p class="text-xs sm:text-sm text-gray-500">处理订单</p>
                </div>
            </a>
            <a href="/inventory" class="flex items-center gap-3 p-3 sm:p-4 bg-orange-50 rounded-lg hover:bg-orange-100 transition-colors">
                <span class="text-xl sm:text-2xl">📊</span>
                <div>
                    <p class="font-medium text-gray-800 text-sm sm:text-base">库存管理</p>
                    <p class="text-xs sm:text-sm text-gray-500">查看库存</p>
                </div>
            </a>
            <a href="/customers" class="flex items-center gap-3 p-3 sm:p-4 bg-purple-50 rounded-lg hover:bg-purple-100 transition-colors">
                <span class="text-xl sm:text-2xl">👥</span>
                <div>
                    <p class="font-medium text-gray-800 text-sm sm:text-base">客户管理</p>
                    <p class="text-xs sm:text-sm text-gray-500">管理客户</p>
                </div>
            </a>
        </div>
    </div>
</div>"#,
        stats.total_orders,
        stats.today_sales,
        stats.low_stock_count,
        stats.total_products
    );

    render_layout("仪表板", "dashboard", Some(user), &content)
}

// ============================================================================
// 产品管理
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct ProductsQuery {
    page: Option<u32>,
    keyword: Option<String>,
}

/// 产品列表页面
pub async fn products_page(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Query(query): Query<ProductsQuery>,
) -> Html<String> {
    let user = get_user_from_extension(&auth_user);
    let page = query.page.unwrap_or(1).max(1);
    let page_size = 20;

    let queries = ProductQueries::new(state.db.pool());
    let result = queries
        .list(page, page_size, None, None, None, query.keyword.as_deref())
        .await
        .unwrap_or_else(|_| PagedResponse::new(vec![], page, page_size, 0));

    let products = result.items;
    let total = result.pagination.total;
    let total_pages = ((total as f64) / (page_size as f64)).ceil() as u32;

    let mut rows = String::new();
    for p in &products {
        let status_badge = match p.status {
            1 => r#"<span class="px-2 py-1 text-xs font-medium bg-green-100 text-green-700 rounded-full">上架</span>"#,
            2 => r#"<span class="px-2 py-1 text-xs font-medium bg-gray-100 text-gray-600 rounded-full">下架</span>"#,
            _ => r#"<span class="px-2 py-1 text-xs font-medium bg-yellow-100 text-yellow-700 rounded-full">草稿</span>"#,
        };

        let stock_class = if p.stock_quantity.unwrap_or(0) <= 0 { "text-red-600 font-semibold" } else { "text-gray-800" };

        // 成本显示
        let cost_cny_display = p.cost_cny.map(|c| format!("¥{:.2}", c)).unwrap_or_else(|| "-".to_string());
        let cost_usd_display = p.cost_usd.map(|c| format!("${:.2}", c)).unwrap_or_else(|| "-".to_string());

        // 售价显示
        let price_usd_display = p.sale_price_usd.map(|v| format!("${:.2}", v)).unwrap_or_else(|| "-".to_string());

        // 平台费用
        let platform_fee_display = p.platform_fee.map(|f| format!("${:.2}", f)).unwrap_or_else(|| "-".to_string());

        // 利润显示
        let profit_usd_display = p.profit_usd.map(|v| {
            let color = if v >= 0.0 { "text-green-600" } else { "text-red-600" };
            format!(r#"<span class="{}">${:.2}</span>"#, color, v)
        }).unwrap_or_else(|| "-".to_string());

        let profit_margin_display = p.profit_margin.map(|v| {
            let color = if v >= 0.0 { "text-green-600" } else { "text-red-600" };
            format!(r#"<span class="{}">{:.1}%</span>"#, color, v)
        }).unwrap_or_else(|| "-".to_string());

        rows.push_str(&format!(
            r#"<tr class="hover:bg-gray-50 transition-colors">
                <td class="px-2 py-3"><span class="font-mono text-xs text-gray-600">{}</span></td>
                <td class="px-2 py-3"><span class="font-medium text-gray-800 text-sm">{}</span></td>
                <td class="px-2 py-3 text-right"><span class="text-xs text-gray-600">{}</span></td>
                <td class="px-2 py-3 text-right"><span class="text-xs text-gray-600">{}</span></td>
                <td class="px-2 py-3 text-right"><span class="text-xs font-medium text-gray-800">{}</span></td>
                <td class="px-2 py-3 text-right"><span class="text-xs text-gray-600">{}</span></td>
                <td class="px-2 py-3 text-right"><span class="text-xs">{}</span></td>
                <td class="px-2 py-3 text-right"><span class="text-xs">{}</span></td>
                <td class="px-2 py-3 text-right"><span class="{} text-xs">{}</span></td>
                <td class="px-2 py-3 text-center">{}</td>
                <td class="px-2 py-3 text-center">
                    <div class="flex items-center justify-center gap-1">
                        <a href="/products/{}" class="px-2 py-1 text-xs text-blue-600 hover:text-blue-800 hover:bg-blue-50 rounded">查看</a>
                        <a href="/products/{}/edit" class="px-2 py-1 text-xs text-green-600 hover:text-green-800 hover:bg-green-50 rounded">编辑</a>
                    </div>
                </td>
            </tr>"#,
            p.product_code,
            p.name,
            cost_cny_display,
            cost_usd_display,
            price_usd_display,
            platform_fee_display,
            profit_usd_display,
            profit_margin_display,
            stock_class,
            p.stock_quantity.unwrap_or(0),
            status_badge,
            p.id,
            p.id
        ));
    }

    if rows.is_empty() {
        rows = r#"<tr><td colspan="11" class="px-6 py-12 text-center"><div class="text-gray-500"><p class="text-4xl mb-2">📦</p><p>暂无产品数据</p></div></td></tr>"#.to_string();
    }

    let pagination = if total_pages > 1 {
        // 上一页按钮
        let prev_btn = if page > 1 {
            let url = match &query.keyword {
                Some(k) => format!("/products?page={}&keyword={}", page - 1, k),
                None => format!("/products?page={}", page - 1),
            };
            format!(r#"<a href="{}" class="px-4 py-2 text-sm text-gray-600 hover:bg-gray-100 rounded-lg border border-gray-300">上一页</a>"#, url)
        } else {
            r#"<span class="px-4 py-2 text-sm text-gray-400 rounded-lg border border-gray-200">上一页</span>"#.to_string()
        };
        // 下一页按钮
        let next_btn = if page < total_pages {
            let url = match &query.keyword {
                Some(k) => format!("/products?page={}&keyword={}", page + 1, k),
                None => format!("/products?page={}", page + 1),
            };
            format!(r#"<a href="{}" class="px-4 py-2 text-sm text-gray-600 hover:bg-gray-100 rounded-lg border border-gray-300">下一页</a>"#, url)
        } else {
            r#"<span class="px-4 py-2 text-sm text-gray-400 rounded-lg border border-gray-200">下一页</span>"#.to_string()
        };
        format!(
            r#"<div class="flex items-center justify-between mt-4 sm:mt-6 px-2">
                <p class="text-xs sm:text-sm text-gray-600">共 {} 条，第 {}/{} 页</p>
                <div class="flex items-center gap-2">{}{}</div>
            </div>"#,
            total, page, total_pages, prev_btn, next_btn
        )
    } else {
        String::new()
    };

    let content = format!(
        r#"<!-- 页面标题 -->
<div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4 mb-6">
    <div>
        <h1 class="text-xl sm:text-2xl font-bold text-gray-800">产品管理</h1>
        <p class="text-gray-600 mt-1 text-sm sm:text-base">管理所有产品信息</p>
    </div>
    <a href="/products/new" class="inline-flex items-center justify-center gap-2 px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors w-full sm:w-auto">
        <span>+</span><span>新增产品</span>
    </a>
</div>

<!-- 搜索栏 -->
<div class="bg-white rounded-xl shadow-sm border border-gray-100 p-4 mb-6">
    <form action="/products" method="GET" class="flex flex-col sm:flex-row gap-3 sm:gap-4">
        <div class="flex-1">
            <input type="text" name="keyword" value="{}" placeholder="搜索产品编码、名称..."
                   class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent">
        </div>
        <button type="submit" class="px-6 py-2 bg-gray-100 text-gray-700 rounded-lg hover:bg-gray-200 transition-colors w-full sm:w-auto">搜索</button>
    </form>
</div>

<!-- 产品表格 -->
<div class="bg-white rounded-xl shadow-sm border border-gray-100 overflow-hidden">
    <div class="overflow-x-auto">
        <table class="w-full min-w-[1100px]">
            <thead class="bg-gray-50 border-b border-gray-200">
                <tr>
                    <th class="px-2 py-3 text-left text-xs font-semibold text-gray-700">产品编码</th>
                    <th class="px-2 py-3 text-left text-xs font-semibold text-gray-700">产品名称</th>
                    <th class="px-2 py-3 text-right text-xs font-semibold text-gray-700">RMB成本</th>
                    <th class="px-2 py-3 text-right text-xs font-semibold text-gray-700">美金成本</th>
                    <th class="px-2 py-3 text-right text-xs font-semibold text-gray-700">美金卖价</th>
                    <th class="px-2 py-3 text-right text-xs font-semibold text-gray-700">平台费用</th>
                    <th class="px-2 py-3 text-right text-xs font-semibold text-gray-700">利润($)</th>
                    <th class="px-2 py-3 text-right text-xs font-semibold text-gray-700">利润率</th>
                    <th class="px-2 py-3 text-right text-xs font-semibold text-gray-700">库存</th>
                    <th class="px-2 py-3 text-center text-xs font-semibold text-gray-700">状态</th>
                    <th class="px-2 py-3 text-center text-xs font-semibold text-gray-700">操作</th>
                </tr>
            </thead>
            <tbody class="divide-y divide-gray-100">{}</tbody>
        </table>
    </div>
</div>
{}"#,
        query.keyword.unwrap_or_default(),
        rows,
        pagination
    );

    render_layout("产品管理", "products", Some(user), &content)
}

/// 新增产品页面
pub async fn product_new_page(
    Extension(auth_user): Extension<AuthUser>,
) -> Html<String> {
    let user = get_user_from_extension(&auth_user);

    let content = r#"<!-- 页面标题 -->
<div class="mb-6">
    <div class="flex items-center gap-2 text-sm text-gray-500 mb-2">
        <a href="/products" class="hover:text-blue-600">产品列表</a>
        <span>/</span>
        <span class="text-gray-800">新增产品</span>
    </div>
    <h1 class="text-xl sm:text-2xl font-bold text-gray-800">新增产品</h1>
</div>

<!-- 产品表单 -->
<div class="bg-white rounded-xl shadow-sm border border-gray-100">
    <form action="/products/new" method="POST" class="p-4 sm:p-6">
        <!-- 第一部分：产品基本信息 -->
        <div class="mb-8">
            <h3 class="text-lg font-semibold text-gray-800 mb-4 pb-2 border-b border-gray-200">
                📦 产品基本信息
            </h3>
            <div class="grid grid-cols-1 md:grid-cols-2 gap-4 sm:gap-6">
                <!-- 产品名称 -->
                <div>
                    <label for="name" class="block text-sm font-medium text-gray-700 mb-2">
                        产品名称 <span class="text-red-500">*</span>
                    </label>
                    <input type="text" id="name" name="name" required
                           class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                           placeholder="请输入产品名称">
                </div>

                <!-- 英文名称 -->
                <div>
                    <label for="name_en" class="block text-sm font-medium text-gray-700 mb-2">
                        英文名称
                    </label>
                    <input type="text" id="name_en" name="name_en"
                           class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                           placeholder="English Name">
                </div>

                <!-- 分类 -->
                <div>
                    <label for="category_id" class="block text-sm font-medium text-gray-700 mb-2">
                        分类
                    </label>
                    <select id="category_id" name="category_id"
                            class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent">
                        <option value="">请选择分类</option>
                    </select>
                </div>

                <!-- 品牌 -->
                <div>
                    <label for="brand_id" class="block text-sm font-medium text-gray-700 mb-2">
                        品牌
                    </label>
                    <select id="brand_id" name="brand_id"
                            class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent">
                        <option value="">请选择品牌</option>
                    </select>
                </div>

                <!-- 供应商 -->
                <div>
                    <label for="supplier_id" class="block text-sm font-medium text-gray-700 mb-2">
                        供应商
                    </label>
                    <select id="supplier_id" name="supplier_id"
                            class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent">
                        <option value="">请选择供应商</option>
                    </select>
                </div>

                <!-- 重量 -->
                <div>
                    <label for="weight" class="block text-sm font-medium text-gray-700 mb-2">
                        重量 (kg)
                    </label>
                    <input type="number" id="weight" name="weight" step="0.001" min="0"
                           class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                           placeholder="0.000">
                </div>

                <!-- 体积 -->
                <div>
                    <label for="volume" class="block text-sm font-medium text-gray-700 mb-2">
                        体积 (m³)
                    </label>
                    <input type="number" id="volume" name="volume" step="0.0001" min="0"
                           class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                           placeholder="0.0000">
                </div>

                <!-- 状态 -->
                <div>
                    <label for="status" class="block text-sm font-medium text-gray-700 mb-2">
                        状态
                    </label>
                    <select id="status" name="status"
                            class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent">
                        <option value="3">草稿</option>
                        <option value="1" selected>上架</option>
                        <option value="2">下架</option>
                    </select>
                </div>

                <!-- 标记 -->
                <div class="flex items-center gap-4 pt-6">
                    <label class="flex items-center gap-2 cursor-pointer">
                        <input type="checkbox" name="is_featured" value="true" class="w-4 h-4 text-blue-600 rounded focus:ring-blue-500">
                        <span class="text-sm text-gray-700">推荐产品</span>
                    </label>
                    <label class="flex items-center gap-2 cursor-pointer">
                        <input type="checkbox" name="is_new" value="true" class="w-4 h-4 text-blue-600 rounded focus:ring-blue-500">
                        <span class="text-sm text-gray-700">新品</span>
                    </label>
                </div>
            </div>

            <!-- 描述 -->
            <div class="mt-4">
                <label for="description" class="block text-sm font-medium text-gray-700 mb-2">
                    产品描述
                </label>
                <textarea id="description" name="description" rows="3"
                          class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                          placeholder="请输入产品描述..."></textarea>
            </div>

            <!-- 备注 -->
            <div class="mt-4">
                <label for="notes" class="block text-sm font-medium text-gray-700 mb-2">
                    备注
                </label>
                <textarea id="notes" name="notes" rows="2"
                          class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                          placeholder="内部备注信息..."></textarea>
            </div>
        </div>

        <!-- 第二部分：参考成本（可选） -->
        <div class="mb-8">
            <h3 class="text-lg font-semibold text-gray-800 mb-4 pb-2 border-b border-gray-200">
                💰 参考成本 <span class="text-sm font-normal text-gray-500">(可选)</span>
            </h3>
            <div class="grid grid-cols-1 md:grid-cols-3 gap-4 sm:gap-6">
                <!-- 成本 CNY -->
                <div>
                    <label for="cost_cny" class="block text-sm font-medium text-gray-700 mb-2">
                        成本 (CNY)
                    </label>
                    <input type="number" id="cost_cny" name="cost_cny" step="0.01" min="0"
                           class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                           placeholder="0.00">
                </div>

                <!-- 成本 USD -->
                <div>
                    <label for="cost_usd" class="block text-sm font-medium text-gray-700 mb-2">
                        成本 (USD)
                    </label>
                    <input type="number" id="cost_usd" name="cost_usd" step="0.01" min="0"
                           class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                           placeholder="0.00">
                </div>

                <!-- 汇率 -->
                <div>
                    <label for="cost_exchange_rate" class="block text-sm font-medium text-gray-700 mb-2">
                        汇率
                    </label>
                    <input type="number" id="cost_exchange_rate" name="cost_exchange_rate" step="0.01" min="0" value="7.2"
                           class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                           placeholder="7.20">
                </div>

                <!-- 成本备注 -->
                <div class="md:col-span-3">
                    <label for="cost_notes" class="block text-sm font-medium text-gray-700 mb-2">
                        成本备注
                    </label>
                    <input type="text" id="cost_notes" name="cost_notes"
                           class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                           placeholder="成本相关备注信息">
                </div>
            </div>
        </div>

        <!-- 第三部分：参考售价（可选） -->
        <div class="mb-8">
            <h3 class="text-lg font-semibold text-gray-800 mb-4 pb-2 border-b border-gray-200">
                🏷️ 参考售价 <span class="text-sm font-normal text-gray-500">(可选)</span>
            </h3>
            <div class="grid grid-cols-1 md:grid-cols-3 gap-4 sm:gap-6">
                <!-- 销售平台 -->
                <div>
                    <label for="price_platform" class="block text-sm font-medium text-gray-700 mb-2">
                        销售平台
                    </label>
                    <select id="price_platform" name="price_platform"
                            class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent">
                        <option value="website">独立站 (Website)</option>
                        <option value="alibaba">阿里巴巴 (Alibaba)</option>
                        <option value="amazon">亚马逊 (Amazon)</option>
                    </select>
                </div>

                <!-- 售价 CNY -->
                <div>
                    <label for="sale_price_cny" class="block text-sm font-medium text-gray-700 mb-2">
                        售价 (CNY)
                    </label>
                    <input type="number" id="sale_price_cny" name="sale_price_cny" step="0.01" min="0"
                           class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                           placeholder="0.00">
                </div>

                <!-- 售价 USD -->
                <div>
                    <label for="sale_price_usd" class="block text-sm font-medium text-gray-700 mb-2">
                        售价 (USD)
                    </label>
                    <input type="number" id="sale_price_usd" name="sale_price_usd" step="0.01" min="0"
                           class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                           placeholder="0.00">
                </div>

                <!-- 汇率 -->
                <div>
                    <label for="price_exchange_rate" class="block text-sm font-medium text-gray-700 mb-2">
                        汇率
                    </label>
                    <input type="number" id="price_exchange_rate" name="price_exchange_rate" step="0.01" min="0" value="7.2"
                           class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                           placeholder="7.20">
                </div>

                <!-- 利润率 -->
                <div>
                    <label for="profit_margin" class="block text-sm font-medium text-gray-700 mb-2">
                        目标利润率 (%)
                    </label>
                    <input type="number" id="profit_margin" name="profit_margin" step="0.01" min="0" value="15"
                           class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                           placeholder="15">
                </div>

                <!-- 平台费率 -->
                <div>
                    <label for="platform_fee_rate" class="block text-sm font-medium text-gray-700 mb-2">
                        平台费率 (%)
                    </label>
                    <input type="number" id="platform_fee_rate" name="platform_fee_rate" step="0.01" min="0" value="2.5"
                           class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                           placeholder="2.5">
                </div>

                <!-- 售价备注 -->
                <div class="md:col-span-3">
                    <label for="price_notes" class="block text-sm font-medium text-gray-700 mb-2">
                        售价备注
                    </label>
                    <input type="text" id="price_notes" name="price_notes"
                           class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                           placeholder="售价相关备注信息">
                </div>
            </div>
        </div>

        <!-- 提交按钮 -->
        <div class="flex items-center gap-4 pt-4 border-t border-gray-200">
            <button type="submit"
                    class="px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors">
                保存产品
            </button>
            <a href="/products" class="px-6 py-2 bg-gray-100 text-gray-700 rounded-lg hover:bg-gray-200 transition-colors">
                取消
            </a>
        </div>
    </form>
</div>"#;

    render_layout("新增产品", "products", Some(user), content)
}

/// 创建产品表单数据
#[derive(Debug, Deserialize)]
pub struct ProductForm {
    // 产品基本信息（product_code 由系统自动生成）
    name: String,
    name_en: Option<String>,
    category_id: Option<i64>,
    brand_id: Option<i64>,
    supplier_id: Option<i64>,
    weight: Option<f64>,
    volume: Option<f64>,
    description: Option<String>,
    status: Option<i64>,
    is_featured: Option<String>,
    is_new: Option<String>,
    notes: Option<String>,
    // 参考成本（可选）
    cost_cny: Option<f64>,
    cost_usd: Option<f64>,
    cost_exchange_rate: Option<f64>,
    cost_notes: Option<String>,
    // 参考售价（可选）
    price_platform: Option<String>,
    sale_price_cny: Option<f64>,
    sale_price_usd: Option<f64>,
    price_exchange_rate: Option<f64>,
    profit_margin: Option<f64>,
    platform_fee_rate: Option<f64>,
    price_notes: Option<String>,
}

/// 创建产品处理
pub async fn product_create_handler(
    State(state): State<AppState>,
    Form(form): Form<ProductForm>,
) -> Result<impl IntoResponse, Html<String>> {
    let queries = ProductQueries::new(state.db.pool());

    // 产品编码由系统自动生成，不再需要检查

    let req = CreateProductRequest {
        product_code: None,  // 自动生成
        name: form.name.clone(),
        name_en: form.name_en.clone(),
        slug: None,
        category_id: form.category_id,
        brand_id: form.brand_id,
        supplier_id: form.supplier_id,
        weight: form.weight,
        volume: form.volume,
        description: form.description.clone(),
        description_en: None,
        specifications: None,
        main_image: None,
        images: None,
        status: form.status,
        is_featured: Some(form.is_featured.is_some()),
        is_new: Some(form.is_new.is_some()),
        notes: form.notes.clone(),
    };

    match queries.create(&req).await {
        Ok(product) => {
            info!("Product created: id={}, code={}", product.id, product.product_code);

            // 如果有参考成本，创建成本记录
            if let Some(cost_cny) = form.cost_cny {
                if cost_cny > 0.0 {
                    let cost_queries = ProductCostQueries::new(state.db.pool());
                    let cost_req = CreateProductCostRequest {
                        product_id: product.id,
                        supplier_id: form.supplier_id,
                        cost_cny,
                        cost_usd: form.cost_usd,
                        currency: Some("CNY".to_string()),
                        exchange_rate: form.cost_exchange_rate.or(Some(7.2)),
                        profit_margin: Some(0.15),
                        platform_fee_rate: Some(0.025),
                        platform_fee: None,
                        sale_price_usd: None,
                        quantity: Some(1),
                        purchase_order_id: None,
                        is_reference: Some(true),
                        effective_date: None,
                        notes: form.cost_notes.clone(),
                    };
                    if let Err(e) = cost_queries.create(&cost_req).await {
                        info!("Failed to create reference cost: {}", e);
                    }
                }
            }

            // 如果有参考售价，创建价格记录
            if let Some(sale_price_cny) = form.sale_price_cny {
                if sale_price_cny > 0.0 {
                    let price_queries = ProductPriceQueries::new(state.db.pool());
                    let price_req = CreateProductPriceRequest {
                        product_id: product.id,
                        platform: form.price_platform.clone().or(Some("website".to_string())),
                        sale_price_cny,
                        sale_price_usd: form.sale_price_usd,
                        exchange_rate: form.price_exchange_rate.or(Some(7.2)),
                        profit_margin: form.profit_margin.map(|v| v / 100.0).or(Some(0.15)),
                        platform_fee_rate: form.platform_fee_rate.map(|v| v / 100.0).or(Some(0.025)),
                        platform_fee: None,
                        is_reference: Some(true),
                        effective_date: None,
                        notes: form.price_notes.clone(),
                    };
                    if let Err(e) = price_queries.create(&price_req).await {
                        info!("Failed to create reference price: {}", e);
                    }
                }
            }

            Ok(Redirect::to(&format!("/products/{}", product.id)))
        }
        Err(e) => {
            info!("Failed to create product: {}", e);
            Err(render_product_form_error("创建产品失败，请检查输入信息", &form))
        }
    }
}

/// 渲染带错误信息的表单
fn render_product_form_error(error: &str, form: &ProductForm) -> Html<String> {
    let content = format!(
        r#"<!-- 页面标题 -->
<div class="mb-6">
    <div class="flex items-center gap-2 text-sm text-gray-500 mb-2">
        <a href="/products" class="hover:text-blue-600">产品列表</a>
        <span>/</span>
        <span class="text-gray-800">新增产品</span>
    </div>
    <h1 class="text-xl sm:text-2xl font-bold text-gray-800">新增产品</h1>
</div>

<!-- 错误提示 -->
<div class="mb-6 p-4 bg-red-50 border border-red-200 rounded-lg">
    <p class="text-red-600 text-sm">{}</p>
</div>

<!-- 产品表单（简化版，用于错误回显） -->
<div class="bg-white rounded-xl shadow-sm border border-gray-100">
    <form action="/products/new" method="POST" class="p-4 sm:p-6">
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4 sm:gap-6">
            <!-- 产品名称 -->
            <div>
                <label for="name" class="block text-sm font-medium text-gray-700 mb-2">
                    产品名称 <span class="text-red-500">*</span>
                </label>
                <input type="text" id="name" name="name" value="{}" required
                       class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                       placeholder="请输入产品名称">
            </div>

            <!-- 英文名称 -->
            <div>
                <label for="name_en" class="block text-sm font-medium text-gray-700 mb-2">
                    英文名称
                </label>
                <input type="text" id="name_en" name="name_en" value="{}"
                       class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                       placeholder="English Name">
            </div>

            <!-- 参考成本 -->
            <div>
                <label for="cost_cny" class="block text-sm font-medium text-gray-700 mb-2">
                    参考成本 (CNY)
                </label>
                <input type="number" id="cost_cny" name="cost_cny" value="{}" step="0.01" min="0"
                       class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                       placeholder="0.00">
            </div>

            <!-- 参考售价 -->
            <div>
                <label for="sale_price_cny" class="block text-sm font-medium text-gray-700 mb-2">
                    参考售价 (CNY)
                </label>
                <input type="number" id="sale_price_cny" name="sale_price_cny" value="{}" step="0.01" min="0"
                       class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                       placeholder="0.00">
            </div>

            <!-- 状态 -->
            <div>
                <label for="status" class="block text-sm font-medium text-gray-700 mb-2">
                    状态
                </label>
                <select id="status" name="status"
                        class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent">
                    <option value="3" {}>草稿</option>
                    <option value="1" {}>上架</option>
                    <option value="2" {}>下架</option>
                </select>
            </div>
        </div>

        <!-- 描述 -->
        <div class="mt-6">
            <label for="description" class="block text-sm font-medium text-gray-700 mb-2">
                产品描述
            </label>
            <textarea id="description" name="description" rows="4"
                      class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                      placeholder="请输入产品描述...">{}</textarea>
        </div>

        <!-- 提交按钮 -->
        <div class="mt-6 flex items-center gap-4">
            <button type="submit"
                    class="px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors">
                保存产品
            </button>
            <a href="/products" class="px-6 py-2 bg-gray-100 text-gray-700 rounded-lg hover:bg-gray-200 transition-colors">
                取消
            </a>
        </div>
    </form>
</div>"#,
        error,
        form.name,
        form.name_en.as_deref().unwrap_or(""),
        form.cost_cny.unwrap_or(0.0),
        form.sale_price_cny.unwrap_or(0.0),
        if form.status == Some(3) { "selected" } else { "" },
        if form.status == Some(1) || form.status.is_none() { "selected" } else { "" },
        if form.status == Some(2) { "selected" } else { "" },
        form.description.as_deref().unwrap_or("")
    );

    render_layout("新增产品", "products", None, &content)
}

/// 产品详情页面
pub async fn product_detail_page(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<i64>,
) -> Html<String> {
    let user = get_user_from_extension(&auth_user);

    let queries = ProductQueries::new(state.db.pool());

    // 获取产品信息
    let product = match queries.get_by_id(id).await {
        Ok(Some(p)) => p,
        _ => {
            let content = r#"<div class="text-center py-12">
                <p class="text-4xl mb-4">📦</p>
                <p class="text-gray-600 mb-4">产品不存在</p>
                <a href="/products" class="text-blue-600 hover:text-blue-800">返回产品列表</a>
            </div>"#;
            return render_layout("产品详情", "products", Some(user), content);
        }
    };

    // 获取 SKU 列表
    let skus = queries.get_skus(id).await.unwrap_or_default();

    // 获取成本信息
    let cost_queries = ProductCostQueries::new(state.db.pool());
    let product_cost = cost_queries.get_reference_cost(id).await.unwrap_or(None);

    // 获取价格信息
    let price_queries = ProductPriceQueries::new(state.db.pool());
    let product_price = price_queries.get_reference_price(id, "website").await.unwrap_or(None);

    // 获取内容信息
    let content_queries = ProductContentQueries::new(state.db.pool());
    let product_content = content_queries.get_by_product_id(id).await.unwrap_or(None);

    let status_badge = match product.status {
        1 => r#"<span class="px-3 py-1 text-sm font-medium bg-green-100 text-green-700 rounded-full">上架</span>"#,
        2 => r#"<span class="px-3 py-1 text-sm font-medium bg-gray-100 text-gray-600 rounded-full">下架</span>"#,
        _ => r#"<span class="px-3 py-1 text-sm font-medium bg-yellow-100 text-yellow-700 rounded-full">草稿</span>"#,
    };

    // SKU 列表 HTML
    let sku_rows: String = if skus.is_empty() {
        r#"<tr><td colspan="6" class="px-4 py-8 text-center text-gray-500">暂无 SKU 数据</td></tr>"#.to_string()
    } else {
        skus.iter().map(|sku| {
            let sku_status = if sku.status == 1 {
                r#"<span class="px-2 py-1 text-xs bg-green-100 text-green-700 rounded">正常</span>"#
            } else {
                r#"<span class="px-2 py-1 text-xs bg-gray-100 text-gray-600 rounded">禁用</span>"#
            };
            format!(
                r#"<tr class="hover:bg-gray-50">
                    <td class="px-4 py-3"><span class="font-mono text-sm">{}</span></td>
                    <td class="px-4 py-3 text-right">¥{:.2}</td>
                    <td class="px-4 py-3 text-right">¥{:.2}</td>
                    <td class="px-4 py-3 text-right">{}</td>
                    <td class="px-4 py-3 text-center">{}</td>
                </tr>"#,
                sku.sku_code,
                sku.cost_price,
                sku.sale_price,
                sku.stock_quantity,
                sku_status
            )
        }).collect()
    };

    // 成本信息 HTML
    let cost_section = if let Some(ref cost) = product_cost {
        format!(
            r#"<div class="bg-white rounded-xl shadow-sm border border-gray-100 p-4 sm:p-6">
    <h3 class="text-base font-semibold text-gray-800 mb-4">💰 成本信息</h3>
    <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
        <div>
            <p class="text-xs text-gray-500">成本 (CNY)</p>
            <p class="text-lg font-semibold text-gray-800">¥{:.2}</p>
        </div>
        <div>
            <p class="text-xs text-gray-500">成本 (USD)</p>
            <p class="text-lg font-semibold text-gray-800">{}</p>
        </div>
        <div>
            <p class="text-xs text-gray-500">利润率</p>
            <p class="text-lg font-semibold text-gray-800">{:.1}%</p>
        </div>
        <div>
            <p class="text-xs text-gray-500">平台费率</p>
            <p class="text-lg font-semibold text-gray-800">{:.1}%</p>
        </div>
    </div>
    <div class="mt-4 pt-4 border-t border-gray-100">
        <p class="text-xs text-gray-500">备注: {}</p>
    </div>
</div>"#,
            cost.cost_cny,
            cost.cost_usd.map(|v| format!("${:.2}", v)).unwrap_or("-".to_string()),
            cost.profit_margin * 100.0,
            cost.platform_fee_rate * 100.0,
            cost.notes.as_deref().unwrap_or("无")
        )
    } else {
        r#"<div class="bg-white rounded-xl shadow-sm border border-gray-100 p-4 sm:p-6">
    <h3 class="text-base font-semibold text-gray-800 mb-4">💰 成本信息</h3>
    <p class="text-gray-500 text-sm">暂无成本数据</p>
</div>"#.to_string()
    };

    // 内容信息 HTML
    let content_section = if let Some(ref content) = product_content {
        format!(
            r#"<div class="bg-white rounded-xl shadow-sm border border-gray-100 p-4 sm:p-6">
    <h3 class="text-base font-semibold text-gray-800 mb-4">📝 内容信息</h3>
    <div class="space-y-4">
        <div>
            <p class="text-xs text-gray-500 mb-1">英文标题</p>
            <p class="text-sm text-gray-800">{}</p>
        </div>
        <div>
            <p class="text-xs text-gray-500 mb-1">描述</p>
            <p class="text-sm text-gray-800">{}</p>
        </div>
        <div>
            <p class="text-xs text-gray-500 mb-1">SEO 标题</p>
            <p class="text-sm text-gray-800">{}</p>
        </div>
    </div>
</div>"#,
            content.title_en.as_deref().unwrap_or("-"),
            content.description.as_deref().map(|d| if d.len() > 200 { &d[..200] } else { d }).unwrap_or("-"),
            content.meta_title.as_deref().unwrap_or("-")
        )
    } else {
        r#"<div class="bg-white rounded-xl shadow-sm border border-gray-100 p-4 sm:p-6">
    <h3 class="text-base font-semibold text-gray-800 mb-4">📝 内容信息</h3>
    <p class="text-gray-500 text-sm">暂无内容数据</p>
</div>"#.to_string()
    };

    let content = format!(
        r#"<!-- 页面标题 -->
<div class="mb-6">
    <div class="flex items-center gap-2 text-sm text-gray-500 mb-2">
        <a href="/products" class="hover:text-blue-600">产品列表</a>
        <span>/</span>
        <span class="text-gray-800">{}</span>
    </div>
    <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div class="flex items-center gap-3">
            <h1 class="text-xl sm:text-2xl font-bold text-gray-800">{}</h1>
            {}
        </div>
        <div class="flex items-center gap-2">
            <a href="/products/{}/edit" class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors text-sm">
                编辑
            </a>
            <a href="/products" class="px-4 py-2 bg-gray-100 text-gray-700 rounded-lg hover:bg-gray-200 transition-colors text-sm">
                返回列表
            </a>
        </div>
    </div>
</div>

<!-- 基本信息 -->
<div class="bg-white rounded-xl shadow-sm border border-gray-100 p-4 sm:p-6 mb-6">
    <h3 class="text-base font-semibold text-gray-800 mb-4">📦 基本信息</h3>
    <div class="grid grid-cols-2 md:grid-cols-4 gap-4 sm:gap-6">
        <div>
            <p class="text-xs text-gray-500 mb-1">产品编码</p>
            <p class="font-mono text-sm text-gray-800">{}</p>
        </div>
        <div>
            <p class="text-xs text-gray-500 mb-1">产品名称</p>
            <p class="text-sm text-gray-800 font-medium">{}</p>
        </div>
        <div>
            <p class="text-xs text-gray-500 mb-1">英文名称</p>
            <p class="text-sm text-gray-800">{}</p>
        </div>
        <div>
            <p class="text-xs text-gray-500 mb-1">创建时间</p>
            <p class="text-sm text-gray-800">{}</p>
        </div>
        <div>
            <p class="text-xs text-gray-500 mb-1">参考成本 (CNY)</p>
            <p class="text-lg font-semibold text-gray-800">¥{:.2}</p>
        </div>
        <div>
            <p class="text-xs text-gray-500 mb-1">参考售价 (USD)</p>
            <p class="text-lg font-semibold text-green-600">${:.2}</p>
        </div>
        <div>
            <p class="text-xs text-gray-500 mb-1">重量</p>
            <p class="text-sm text-gray-800">{} kg</p>
        </div>
        <div>
            <p class="text-xs text-gray-500 mb-1">体积</p>
            <p class="text-sm text-gray-800">{} m³</p>
        </div>
    </div>
    <div class="mt-4 pt-4 border-t border-gray-100">
        <p class="text-xs text-gray-500 mb-1">产品描述</p>
        <p class="text-sm text-gray-700">{}</p>
    </div>
    <div class="mt-4 pt-4 border-t border-gray-100">
        <p class="text-xs text-gray-500 mb-1">备注</p>
        <p class="text-sm text-gray-700">{}</p>
    </div>
</div>

<!-- 成本和内容信息 -->
<div class="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-6">
    {}
    {}
</div>

<!-- SKU 列表 -->
<div class="bg-white rounded-xl shadow-sm border border-gray-100">
    <div class="p-4 sm:p-6 border-b border-gray-100">
        <h3 class="text-base font-semibold text-gray-800">SKU 列表</h3>
    </div>
    <div class="overflow-x-auto">
        <table class="w-full min-w-[500px]">
            <thead class="bg-gray-50">
                <tr>
                    <th class="px-4 py-3 text-left text-sm font-semibold text-gray-700">SKU编码</th>
                    <th class="px-4 py-3 text-right text-sm font-semibold text-gray-700">成本价</th>
                    <th class="px-4 py-3 text-right text-sm font-semibold text-gray-700">售价</th>
                    <th class="px-4 py-3 text-right text-sm font-semibold text-gray-700">库存</th>
                    <th class="px-4 py-3 text-center text-sm font-semibold text-gray-700">状态</th>
                </tr>
            </thead>
            <tbody class="divide-y divide-gray-100">
                {}
            </tbody>
        </table>
    </div>
</div>"#,
        product.name,
        product.name,
        status_badge,
        product.id,
        product.product_code,
        product.name,
        product.name_en.as_deref().unwrap_or("-"),
        product.created_at.format("%Y-%m-%d %H:%M"),
        product_cost.as_ref().map(|c| c.cost_cny).unwrap_or(0.0),
        product_price.as_ref().and_then(|p| p.sale_price_usd).unwrap_or(0.0),
        product.weight.map(|w| format!("{:.3}", w)).unwrap_or("-".to_string()),
        product.volume.map(|v| format!("{:.4}", v)).unwrap_or("-".to_string()),
        product.description.as_deref().unwrap_or("无描述"),
        product.notes.as_deref().unwrap_or("无"),
        cost_section,
        content_section,
        sku_rows
    );

    render_layout("产品详情", "products", Some(user), &content)
}

/// 产品编辑页面
pub async fn product_edit_page(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<i64>,
) -> Html<String> {
    let user = get_user_from_extension(&auth_user);

    let queries = ProductQueries::new(state.db.pool());

    // 获取产品信息
    let product = match queries.get_by_id(id).await {
        Ok(Some(p)) => p,
        _ => {
            let content = r#"<div class="text-center py-12">
                <p class="text-4xl mb-4">📦</p>
                <p class="text-gray-600 mb-4">产品不存在</p>
                <a href="/products" class="text-blue-600 hover:text-blue-800">返回产品列表</a>
            </div>"#;
            return render_layout("编辑产品", "products", Some(user), content);
        }
    };

    // 获取成本信息
    let cost_queries = ProductCostQueries::new(state.db.pool());
    let product_cost = cost_queries.get_reference_cost(id).await.unwrap_or(None);

    // 获取价格信息
    let price_queries = ProductPriceQueries::new(state.db.pool());
    let product_price = price_queries.get_reference_price(id, "website").await.unwrap_or(None);

    // 状态选中
    let status_options = format!(
        r#"<option value="3" {}>草稿</option>
            <option value="1" {}>上架</option>
            <option value="2" {}>下架</option>"#,
        if product.status == 3 { "selected" } else { "" },
        if product.status == 1 { "selected" } else { "" },
        if product.status == 2 { "selected" } else { "" }
    );

    let content = format!(
        r#"<!-- 页面标题 -->
<div class="mb-6">
    <div class="flex items-center gap-2 text-sm text-gray-500 mb-2">
        <a href="/products" class="hover:text-blue-600">产品列表</a>
        <span>/</span>
        <a href="/products/{}" class="hover:text-blue-600">{}</a>
        <span>/</span>
        <span class="text-gray-800">编辑</span>
    </div>
    <h1 class="text-xl sm:text-2xl font-bold text-gray-800">编辑产品</h1>
</div>

<!-- 编辑表单 -->
<div class="bg-white rounded-xl shadow-sm border border-gray-100">
    <form action="/products/{}/edit" method="POST" class="p-4 sm:p-6">
        <!-- 第一部分：基本信息 -->
        <div class="mb-8">
            <h3 class="text-lg font-semibold text-gray-800 mb-4 pb-2 border-b border-gray-200">
                📦 基本信息
            </h3>
            <div class="grid grid-cols-1 md:grid-cols-3 gap-4 sm:gap-6">
                <!-- 产品编码（不可编辑） -->
                <div>
                    <label class="block text-sm font-medium text-gray-700 mb-2">
                        产品编码 <span class="text-gray-400">(不可修改)</span>
                    </label>
                    <input type="text" value="{}" disabled
                           class="w-full px-4 py-2 border border-gray-200 rounded-lg bg-gray-50 text-gray-500">
                </div>

                <!-- 产品名称 -->
                <div>
                    <label for="name" class="block text-sm font-medium text-gray-700 mb-2">
                        产品名称 <span class="text-red-500">*</span>
                    </label>
                    <input type="text" id="name" name="name" value="{}" required
                           class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                           placeholder="请输入产品名称">
                </div>

                <!-- 英文名称 -->
                <div>
                    <label for="name_en" class="block text-sm font-medium text-gray-700 mb-2">
                        英文名称
                    </label>
                    <input type="text" id="name_en" name="name_en" value="{}"
                           class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                           placeholder="English Name">
                </div>

                <!-- 重量 -->
                <div>
                    <label for="weight" class="block text-sm font-medium text-gray-700 mb-2">
                        重量 (kg)
                    </label>
                    <input type="number" id="weight" name="weight" step="0.001" min="0" value="{}"
                           class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                           placeholder="0.000">
                </div>

                <!-- 体积 -->
                <div>
                    <label for="volume" class="block text-sm font-medium text-gray-700 mb-2">
                        体积 (m³)
                    </label>
                    <input type="number" id="volume" name="volume" step="0.0001" min="0" value="{}"
                           class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                           placeholder="0.0000">
                </div>

                <!-- 状态 -->
                <div>
                    <label for="status" class="block text-sm font-medium text-gray-700 mb-2">
                        状态
                    </label>
                    <select id="status" name="status"
                            class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent">
                        {}
                    </select>
                </div>

                <!-- 描述 -->
                <div class="md:col-span-3">
                    <label for="description" class="block text-sm font-medium text-gray-700 mb-2">
                        产品描述
                    </label>
                    <textarea id="description" name="description" rows="3"
                              class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                              placeholder="请输入产品描述...">{}</textarea>
                </div>

                <!-- 备注 -->
                <div class="md:col-span-3">
                    <label for="notes" class="block text-sm font-medium text-gray-700 mb-2">
                        备注
                    </label>
                    <input type="text" id="notes" name="notes" value="{}"
                           class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                           placeholder="内部备注信息">
                </div>
            </div>
        </div>

        <!-- 第二部分：参考成本 -->
        <div class="mb-8">
            <h3 class="text-lg font-semibold text-gray-800 mb-4 pb-2 border-b border-gray-200">
                💰 参考成本 <span class="text-sm font-normal text-gray-500">(可选，不影响历史订单)</span>
            </h3>
            <div class="grid grid-cols-1 md:grid-cols-3 gap-4 sm:gap-6">
                <!-- 成本 CNY -->
                <div>
                    <label for="cost_cny" class="block text-sm font-medium text-gray-700 mb-2">
                        成本 (CNY)
                    </label>
                    <input type="number" id="cost_cny" name="cost_cny" step="0.01" min="0" value="{}"
                           class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                           placeholder="0.00">
                </div>

                <!-- 成本 USD -->
                <div>
                    <label for="cost_usd" class="block text-sm font-medium text-gray-700 mb-2">
                        成本 (USD)
                    </label>
                    <input type="number" id="cost_usd" name="cost_usd" step="0.01" min="0" value="{}"
                           class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                           placeholder="0.00">
                </div>

                <!-- 汇率 -->
                <div>
                    <label for="cost_exchange_rate" class="block text-sm font-medium text-gray-700 mb-2">
                        汇率
                    </label>
                    <input type="number" id="cost_exchange_rate" name="cost_exchange_rate" step="0.01" min="0" value="{}"
                           class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                           placeholder="7.20">
                </div>

                <!-- 成本备注 -->
                <div class="md:col-span-3">
                    <label for="cost_notes" class="block text-sm font-medium text-gray-700 mb-2">
                        成本备注
                    </label>
                    <input type="text" id="cost_notes" name="cost_notes" value="{}"
                           class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                           placeholder="成本相关备注信息">
                </div>
            </div>
        </div>

        <!-- 第三部分：参考售价 -->
        <div class="mb-8">
            <h3 class="text-lg font-semibold text-gray-800 mb-4 pb-2 border-b border-gray-200">
                🏷️ 参考售价 <span class="text-sm font-normal text-gray-500">(可选，不影响历史订单)</span>
            </h3>
            <div class="grid grid-cols-1 md:grid-cols-3 gap-4 sm:gap-6">
                <!-- 销售平台 -->
                <div>
                    <label for="price_platform" class="block text-sm font-medium text-gray-700 mb-2">
                        销售平台
                    </label>
                    <select id="price_platform" name="price_platform"
                            class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent">
                        <option value="website" {}>独立站 (Website)</option>
                        <option value="alibaba" {}>阿里巴巴 (Alibaba)</option>
                        <option value="amazon" {}>亚马逊 (Amazon)</option>
                    </select>
                </div>

                <!-- 售价 CNY -->
                <div>
                    <label for="sale_price_cny" class="block text-sm font-medium text-gray-700 mb-2">
                        售价 (CNY)
                    </label>
                    <input type="number" id="sale_price_cny" name="sale_price_cny" step="0.01" min="0" value="{}"
                           class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                           placeholder="0.00">
                </div>

                <!-- 售价 USD -->
                <div>
                    <label for="sale_price_usd" class="block text-sm font-medium text-gray-700 mb-2">
                        售价 (USD)
                    </label>
                    <input type="number" id="sale_price_usd" name="sale_price_usd" step="0.01" min="0" value="{}"
                           class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                           placeholder="0.00">
                </div>

                <!-- 汇率 -->
                <div>
                    <label for="price_exchange_rate" class="block text-sm font-medium text-gray-700 mb-2">
                        汇率
                    </label>
                    <input type="number" id="price_exchange_rate" name="price_exchange_rate" step="0.01" min="0" value="{}"
                           class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                           placeholder="7.20">
                </div>

                <!-- 利润率 -->
                <div>
                    <label for="profit_margin" class="block text-sm font-medium text-gray-700 mb-2">
                        目标利润率 (%)
                    </label>
                    <input type="number" id="profit_margin" name="profit_margin" step="0.01" min="0" value="{}"
                           class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                           placeholder="15">
                </div>

                <!-- 平台费率 -->
                <div>
                    <label for="platform_fee_rate" class="block text-sm font-medium text-gray-700 mb-2">
                        平台费率 (%)
                    </label>
                    <input type="number" id="platform_fee_rate" name="platform_fee_rate" step="0.01" min="0" value="{}"
                           class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                           placeholder="2.5">
                </div>

                <!-- 售价备注 -->
                <div class="md:col-span-3">
                    <label for="price_notes" class="block text-sm font-medium text-gray-700 mb-2">
                        售价备注
                    </label>
                    <input type="text" id="price_notes" name="price_notes" value="{}"
                           class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                           placeholder="售价相关备注信息">
                </div>
            </div>
        </div>

        <!-- 提交按钮 -->
        <div class="flex items-center gap-4 pt-4 border-t border-gray-200">
            <button type="submit"
                    class="px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors">
                保存修改
            </button>
            <a href="/products/{}" class="px-6 py-2 bg-gray-100 text-gray-700 rounded-lg hover:bg-gray-200 transition-colors">
                取消
            </a>
        </div>
    </form>
</div>"#,
        product.id,
        product.name,
        product.id,
        product.product_code,
        product.name,
        product.name_en.as_deref().unwrap_or(""),
        product.weight.map(|w| format!("{:.3}", w)).unwrap_or_default(),
        product.volume.map(|v| format!("{:.4}", v)).unwrap_or_default(),
        status_options,
        product.description.as_deref().unwrap_or(""),
        product.notes.as_deref().unwrap_or(""),
        product_cost.as_ref().map(|c| format!("{:.2}", c.cost_cny)).unwrap_or_default(),
        product_cost.as_ref().and_then(|c| c.cost_usd.map(|v| format!("{:.2}", v))).unwrap_or_default(),
        product_cost.as_ref().map(|c| format!("{:.2}", c.exchange_rate)).unwrap_or_else(|| "7.2".to_string()),
        product_cost.as_ref().and_then(|c| c.notes.clone()).unwrap_or_default(),
        if product_price.as_ref().map(|p| p.platform.as_str()) == Some("website") { "selected" } else { "" },
        if product_price.as_ref().map(|p| p.platform.as_str()) == Some("alibaba") { "selected" } else { "" },
        if product_price.as_ref().map(|p| p.platform.as_str()) == Some("amazon") { "selected" } else { "" },
        product_price.as_ref().map(|p| format!("{:.2}", p.sale_price_cny)).unwrap_or_default(),
        product_price.as_ref().and_then(|p| p.sale_price_usd.map(|v| format!("{:.2}", v))).unwrap_or_default(),
        product_price.as_ref().map(|p| format!("{:.2}", p.exchange_rate)).unwrap_or_else(|| "7.2".to_string()),
        product_price.as_ref().map(|p| format!("{:.1}", p.profit_margin * 100.0)).unwrap_or_else(|| "15".to_string()),
        product_price.as_ref().map(|p| format!("{:.1}", p.platform_fee_rate * 100.0)).unwrap_or_else(|| "2.5".to_string()),
        product_price.as_ref().and_then(|p| p.notes.clone()).unwrap_or_default(),
        product.id
    );

    render_layout("编辑产品", "products", Some(user), &content)
}

/// 编辑产品表单数据
#[derive(Debug, Deserialize)]
pub struct ProductEditForm {
    // 产品基本信息（不含 product_code，不可编辑）
    name: String,
    name_en: Option<String>,
    weight: Option<f64>,
    volume: Option<f64>,
    description: Option<String>,
    status: Option<i64>,
    notes: Option<String>,
    // 参考成本（可选）
    cost_cny: Option<f64>,
    cost_usd: Option<f64>,
    cost_exchange_rate: Option<f64>,
    cost_notes: Option<String>,
    // 参考售价（可选）
    price_platform: Option<String>,
    sale_price_cny: Option<f64>,
    sale_price_usd: Option<f64>,
    price_exchange_rate: Option<f64>,
    profit_margin: Option<f64>,
    platform_fee_rate: Option<f64>,
    price_notes: Option<String>,
}

/// 更新产品处理
pub async fn product_update_handler(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Form(form): Form<ProductEditForm>,
) -> Result<impl IntoResponse, Html<String>> {
    let queries = ProductQueries::new(state.db.pool());

    // 检查产品是否存在
    if queries.get_by_id(id).await.unwrap_or(None).is_none() {
        let error_content = r#"<div class="text-center py-12">
            <p class="text-4xl mb-4">📦</p>
            <p class="text-gray-600 mb-4">产品不存在</p>
            <a href="/products" class="text-blue-600 hover:text-blue-800">返回产品列表</a>
        </div>"#;
        return Err(Html(error_content.to_string()));
    }

    // 更新产品基本信息
    let update_req = UpdateProductRequest {
        name: Some(form.name.clone()),
        name_en: form.name_en.clone(),
        slug: None,
        category_id: None,
        brand_id: None,
        supplier_id: None,
        weight: form.weight,
        volume: form.volume,
        description: form.description.clone(),
        description_en: None,
        specifications: None,
        main_image: None,
        images: None,
        status: form.status,
        is_featured: None,
        is_new: None,
        notes: form.notes.clone(),
    };

    if let Err(e) = queries.update(id, &update_req).await {
        info!("Failed to update product: {}", e);
        let error_content = format!(r#"<div class="text-center py-12">
            <p class="text-4xl mb-4">❌</p>
            <p class="text-gray-600 mb-4">更新产品失败：{}</p>
            <a href="/products/{}" class="text-blue-600 hover:text-blue-800">返回产品详情</a>
        </div>"#, e, id);
        return Err(Html(error_content));
    }

    info!("Product updated: id={}", id);

    // 更新参考成本（如果有）
    if let Some(cost_cny) = form.cost_cny {
        if cost_cny > 0.0 {
            let cost_queries = ProductCostQueries::new(state.db.pool());
            // 尝试更新现有参考成本，如果不存在则创建
            if let Err(_) = cost_queries.update_reference_cost(
                id,
                cost_cny,
                form.cost_usd,
                form.cost_exchange_rate.unwrap_or(7.2),
                form.cost_notes.clone(),
            ).await {
                // 更新失败，尝试创建新的
                let cost_req = CreateProductCostRequest {
                    product_id: id,
                    supplier_id: None,
                    cost_cny,
                    cost_usd: form.cost_usd,
                    currency: Some("CNY".to_string()),
                    exchange_rate: form.cost_exchange_rate.or(Some(7.2)),
                    profit_margin: Some(0.15),
                    platform_fee_rate: Some(0.025),
                    platform_fee: None,
                    sale_price_usd: None,
                    quantity: Some(1),
                    purchase_order_id: None,
                    is_reference: Some(true),
                    effective_date: None,
                    notes: form.cost_notes.clone(),
                };
                let _ = cost_queries.create(&cost_req).await;
            }
        }
    }

    // 更新参考售价（如果有）
    if let Some(sale_price_cny) = form.sale_price_cny {
        if sale_price_cny > 0.0 {
            let price_queries = ProductPriceQueries::new(state.db.pool());
            let platform = form.price_platform.clone().unwrap_or_else(|| "website".to_string());
            // 尝试更新现有参考售价，如果不存在则创建
            if let Err(_) = price_queries.update_reference_price(
                id,
                &platform,
                sale_price_cny,
                form.sale_price_usd,
                form.price_exchange_rate.unwrap_or(7.2),
                form.profit_margin.map(|v| v / 100.0),
                form.platform_fee_rate.map(|v| v / 100.0),
                form.price_notes.clone(),
            ).await {
                // 更新失败，尝试创建新的
                let price_req = CreateProductPriceRequest {
                    product_id: id,
                    platform: form.price_platform.clone(),
                    sale_price_cny,
                    sale_price_usd: form.sale_price_usd,
                    exchange_rate: form.price_exchange_rate.or(Some(7.2)),
                    profit_margin: form.profit_margin.map(|v| v / 100.0).or(Some(0.15)),
                    platform_fee_rate: form.platform_fee_rate.map(|v| v / 100.0).or(Some(0.025)),
                    platform_fee: None,
                    is_reference: Some(true),
                    effective_date: None,
                    notes: form.price_notes.clone(),
                };
                let _ = price_queries.create(&price_req).await;
            }
        }
    }

    Ok(Redirect::to(&format!("/products/{}", id)))
}

// ============================================================================
// 订单管理
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct OrdersQuery {
    pub page: Option<u32>,
    pub status: Option<i64>,
    pub currency: Option<String>,
}

/// 订单列表页面
pub async fn orders_page(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Query(query): Query<OrdersQuery>,
) -> Html<String> {
    let user = get_user_from_extension(&auth_user);
    let page = query.page.unwrap_or(1).max(1);
    let page_size = 20;

    // 获取订单数据
    let queries = OrderQueries::new(state.db.pool());
    let result = queries.list(
        page,
        page_size,
        query.status,
        None, // payment_status
        None, // customer_id
        None, // platform
        None, // date_from
        None, // date_to
        None, // keyword
        query.currency.as_deref(), // currency
    ).await.unwrap_or_else(|_| PagedResponse::new(vec![], page, page_size, 0));

    // 当前币种
    let current_currency = query.currency.as_deref();

    // 生成币种筛选链接
    let currency_filter = |current: Option<&str>, target: Option<&str>, text: &str| -> String {
        let is_active = current == target;
        let mut params = Vec::new();
        if let Some(t) = target {
            params.push(format!("currency={}", t));
        }
        if let Some(s) = query.status {
            params.push(format!("status={}", s));
        }
        let url = if params.is_empty() {
            "/orders".to_string()
        } else {
            format!("/orders?{}", params.join("&"))
        };
        if is_active {
            format!(r#"<a href="{}" class="px-3 py-1 text-sm rounded-full transition-colors bg-green-100 text-green-700">{}</a>"#, url, text)
        } else {
            format!(r#"<a href="{}" class="px-3 py-1 text-sm rounded-full transition-colors text-gray-600 hover:bg-gray-100">{}</a>"#, url, text)
        }
    };

    // 生成状态筛选链接（保留币种参数）
    let status_filter = |current: Option<i64>, target: Option<i64>, text: &str| -> String {
        let is_active = current == target;
        let mut params = Vec::new();
        if let Some(t) = target {
            params.push(format!("status={}", t));
        }
        if let Some(c) = current_currency {
            params.push(format!("currency={}", c));
        }
        let url = if params.is_empty() {
            "/orders".to_string()
        } else {
            format!("/orders?{}", params.join("&"))
        };
        if is_active {
            format!(r#"<a href="{}" class="px-3 py-1 text-sm rounded-full transition-colors bg-blue-100 text-blue-700">{}</a>"#, url, text)
        } else {
            format!(r#"<a href="{}" class="px-3 py-1 text-sm rounded-full transition-colors text-gray-600 hover:bg-gray-100">{}</a>"#, url, text)
        }
    };

    // 生成订单行
    let rows: String = result.items.iter().map(|order| {
        let status_text = crate::templates::orders::order_status_text(order.order_status);
        let status_class = crate::templates::orders::order_status_class(order.order_status);

        // 根据状态生成操作按钮
        let action_buttons = match order.order_status {
            1 => {
                // 未成交 - 可编辑、可下载 PI
                format!(r#"
                    <a href="/orders/{}" class="text-blue-600 hover:text-blue-800 text-sm">详情</a>
                    <a href="/orders/{}/edit" class="text-orange-600 hover:text-orange-800 text-sm ml-2">编辑</a>
                    <a href="/api/v1/orders/{}/download-pi" class="text-green-600 hover:text-green-800 text-sm ml-2" target="_blank">下载PI</a>
                "#, order.id, order.id, order.id)
            }
            2 => {
                // 价格锁定 - 可下载 PI
                format!(r#"
                    <a href="/orders/{}" class="text-blue-600 hover:text-blue-800 text-sm">详情</a>
                    <a href="/api/v1/orders/{}/download-pi" class="text-green-600 hover:text-green-800 text-sm ml-2" target="_blank">下载PI</a>
                "#, order.id, order.id)
            }
            3 | 4 | 5 => {
                // 已付款/已发货/已收货 - 可下载 CI
                format!(r#"
                    <a href="/orders/{}" class="text-blue-600 hover:text-blue-800 text-sm">详情</a>
                    <a href="/api/v1/orders/{}/download-ci" class="text-green-600 hover:text-green-800 text-sm ml-2" target="_blank">下载CI</a>
                "#, order.id, order.id)
            }
            _ => {
                format!(r#"<a href="/orders/{}" class="text-blue-600 hover:text-blue-800 text-sm">详情</a>"#, order.id)
            }
        };

        format!(
            r#"<tr class="hover:bg-gray-50 transition-colors">
                <td class="px-4 sm:px-6 py-4">
                    <span class="font-mono text-sm text-blue-600">{}</span>
                </td>
                <td class="px-4 sm:px-6 py-4">
                    <span class="text-gray-600">{}</span>
                </td>
                <td class="px-4 sm:px-6 py-4">
                    <span class="text-gray-800">{}</span>
                </td>
                <td class="px-4 sm:px-6 py-4 text-right">
                    <span class="font-medium text-gray-800">${:.2}</span>
                </td>
                <td class="px-4 sm:px-6 py-4 text-center">
                    <span class="px-2 py-1 text-xs font-medium rounded-full {}">{}</span>
                </td>
                <td class="px-4 sm:px-6 py-4 text-center">
                    <div class="flex items-center justify-center gap-2">{}</div>
                </td>
            </tr>"#,
            order.order_code,
            order.fulfillment_status,  // 使用 fulfillment_status 代替 platform
            order.customer_name.as_deref().unwrap_or("-"),
            order.total_amount,
            status_class,
            status_text,
            action_buttons
        )
    }).collect();

    let rows = if rows.is_empty() {
        r#"<tr><td colspan="6" class="px-6 py-12 text-center"><div class="text-gray-500"><p class="text-4xl mb-2">📋</p><p>暂无订单数据</p></div></td></tr>"#.to_string()
    } else {
        rows
    };

    // 分页
    let total_pages = ((result.pagination.total as f64) / (page_size as f64)).ceil() as u32;
    let pagination = if total_pages > 1 {
        let build_page_url = |p: u32| -> String {
            let mut params = vec![format!("page={}", p)];
            if let Some(s) = query.status {
                params.push(format!("status={}", s));
            }
            if let Some(c) = current_currency {
                params.push(format!("currency={}", c));
            }
            format!("/orders?{}", params.join("&"))
        };
        let prev_btn = if page > 1 {
            let url = build_page_url(page - 1);
            format!(r#"<a href="{}" class="px-4 py-2 text-sm text-gray-600 hover:bg-gray-100 rounded-lg">上一页</a>"#, url)
        } else {
            String::new()
        };
        let next_btn = if page < total_pages {
            let url = build_page_url(page + 1);
            format!(r#"<a href="{}" class="px-4 py-2 text-sm text-gray-600 hover:bg-gray-100 rounded-lg">下一页</a>"#, url)
        } else {
            String::new()
        };
        format!(
            r#"<div class="flex items-center justify-between mt-4">
                <p class="text-sm text-gray-600">共 {} 条记录，第 {}/{} 页</p>
                <div class="flex items-center gap-2">{}{}</div>
            </div>"#,
            result.pagination.total, page, total_pages, prev_btn, next_btn
        )
    } else {
        String::new()
    };

    let content = format!(
        r#"<!-- 页面标题 -->
<div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4 mb-6">
    <div>
        <h1 class="text-xl sm:text-2xl font-bold text-gray-800">订单管理</h1>
        <p class="text-gray-600 mt-1 text-sm sm:text-base">以订单为核心，PI/CI 为可下载文件</p>
    </div>
    <div class="flex flex-wrap gap-2">
        <a href="/orders/new" class="inline-flex items-center justify-center gap-2 px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors">
            <span>+</span><span>新建订单</span>
        </a>
    </div>
</div>

<!-- 状态说明 -->
<div class="bg-blue-50 rounded-xl border border-blue-100 p-4 mb-6">
    <div class="flex flex-wrap items-center gap-4 text-sm">
        <span class="text-gray-600">状态说明:</span>
        <span class="text-gray-700"><strong>未成交/价格锁定</strong> → 可下载 PI（形式发票）</span>
        <span class="text-gray-700"><strong>已付款/已发货/已收货</strong> → 可下载 CI（商业发票）</span>
    </div>
</div>

<!-- 币种筛选 -->
<div class="bg-white rounded-xl shadow-sm border border-gray-100 p-4 mb-4 overflow-x-auto">
    <div class="flex items-center gap-3 sm:gap-4 min-w-max">
        <span class="text-sm text-gray-500 whitespace-nowrap">币种:</span>
        <div class="flex items-center gap-2">
            {}
            {}
            {}
        </div>
    </div>
</div>

<!-- 状态筛选 -->
<div class="bg-white rounded-xl shadow-sm border border-gray-100 p-4 mb-6 overflow-x-auto">
    <div class="flex items-center gap-3 sm:gap-4 min-w-max">
        <span class="text-sm text-gray-500 whitespace-nowrap">状态:</span>
        <div class="flex items-center gap-2">
            {}
            {}
            {}
            {}
            {}
            {}
        </div>
    </div>
</div>

<!-- 订单表格 -->
<div class="bg-white rounded-xl shadow-sm border border-gray-100 overflow-hidden">
    <div class="overflow-x-auto">
        <table class="w-full min-w-[800px]">
            <thead class="bg-gray-50 border-b border-gray-200">
                <tr>
                    <th class="px-4 sm:px-6 py-4 text-left text-sm font-semibold text-gray-700">订单号</th>
                    <th class="px-4 sm:px-6 py-4 text-left text-sm font-semibold text-gray-700">来源</th>
                    <th class="px-4 sm:px-6 py-4 text-left text-sm font-semibold text-gray-700">客户</th>
                    <th class="px-4 sm:px-6 py-4 text-right text-sm font-semibold text-gray-700">金额</th>
                    <th class="px-4 sm:px-6 py-4 text-center text-sm font-semibold text-gray-700">状态</th>
                    <th class="px-4 sm:px-6 py-4 text-center text-sm font-semibold text-gray-700">操作</th>
                </tr>
            </thead>
            <tbody class="divide-y divide-gray-100">{}</tbody>
        </table>
    </div>
    {}
</div>"#,
        currency_filter(current_currency, None, "全部"),
        currency_filter(current_currency, Some("USD"), "USD"),
        currency_filter(current_currency, Some("CNY"), "CNY"),
        status_filter(query.status, None, "全部"),
        status_filter(query.status, Some(1), "未成交"),
        status_filter(query.status, Some(2), "价格锁定"),
        status_filter(query.status, Some(3), "已付款"),
        status_filter(query.status, Some(4), "已发货"),
        status_filter(query.status, Some(5), "已收货"),
        rows,
        pagination
    );

    render_layout("订单管理", "orders", Some(user), &content)
}

// ============================================================================
// 库存管理
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct InventoryQuery {
    page: Option<u32>,
    low_stock: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct PageQuery {
    page: Option<u32>,
}

/// 库存页面
pub async fn inventory_page(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Query(query): Query<InventoryQuery>,
) -> Html<String> {
    let user = get_user_from_extension(&auth_user);
    let page = query.page.unwrap_or(1).max(1);
    let page_size = 20;

    let queries = InventoryQueries::new(state.db.pool());
    let result = queries.list(
        page,
        page_size,
        query.low_stock,
        None,
        None,
    ).await.unwrap_or_else(|_| PagedResponse::new(vec![], page, page_size, 0));

    // 计算统计数据
    let total_count = result.pagination.total;
    let normal_count = result.items.iter().filter(|i| !i.is_low_stock && i.available_quantity > 0).count();
    let low_stock_count = result.items.iter().filter(|i| i.is_low_stock && i.available_quantity > 0).count();
    let out_of_stock_count = result.items.iter().filter(|i| i.available_quantity <= 0).count();

    let rows: String = result.items.iter().map(|item| {
        let status_badge = if item.available_quantity <= 0 {
            r#"<span class="px-2 py-1 text-xs bg-red-100 text-red-700 rounded-full">缺货</span>"#
        } else if item.is_low_stock {
            r#"<span class="px-2 py-1 text-xs bg-yellow-100 text-yellow-700 rounded-full">低库存</span>"#
        } else {
            r#"<span class="px-2 py-1 text-xs bg-green-100 text-green-700 rounded-full">正常</span>"#
        };
        let row_class = if item.available_quantity <= 0 {
            "bg-red-50"
        } else if item.is_low_stock {
            "bg-yellow-50"
        } else {
            ""
        };
        format!(
            r#"<tr class="hover:bg-gray-50 {}">
                <td class="px-4 sm:px-6 py-4"><span class="font-mono text-sm">{}</span></td>
                <td class="px-4 sm:px-6 py-4 font-medium">{}</td>
                <td class="px-4 sm:px-6 py-4 text-sm text-gray-500">{}</td>
                <td class="px-4 sm:px-6 py-4 text-right">{}</td>
                <td class="px-4 sm:px-6 py-4 text-right font-medium">{}</td>
                <td class="px-4 sm:px-6 py-4 text-right text-gray-500">{}</td>
                <td class="px-4 sm:px-6 py-4 text-right text-gray-500">{}</td>
                <td class="px-4 sm:px-6 py-4 text-center">{}</td>
                <td class="px-4 sm:px-6 py-4 text-center">
                    <a href="/inventory/{}/adjust" class="text-blue-600 hover:text-blue-800 text-sm mr-2">调整</a>
                    <a href="/inventory/{}" class="text-gray-600 hover:text-gray-800 text-sm mr-2">详情</a>
                    <a href="/inventory/{}/movements" class="text-green-600 hover:text-green-800 text-sm">流水</a>
                </td>
            </tr>"#,
            row_class,
            item.sku_code,
            item.product_name,
            item.spec_values,
            item.total_quantity,
            if item.available_quantity <= 0 {
                format!(r#"<span class="text-red-600">{}</span>"#, item.available_quantity)
            } else if item.is_low_stock {
                format!(r#"<span class="text-yellow-600">{}</span>"#, item.available_quantity)
            } else {
                item.available_quantity.to_string()
            },
            item.locked_quantity,
            item.safety_stock,
            status_badge,
            item.sku_id,
            item.sku_id,
            item.sku_id
        )
    }).collect();

    let rows = if rows.is_empty() {
        r#"<tr><td colspan="9" class="px-6 py-12 text-center"><div class="text-gray-500"><p class="text-4xl mb-2">📊</p><p>暂无库存数据</p></div></td></tr>"#.to_string()
    } else {
        rows
    };

    let low_stock_filter = if query.low_stock.unwrap_or(false) {
        r#"<a href="/inventory" class="px-4 py-2 text-sm bg-red-100 text-red-700 rounded-lg">显示全部</a>"#
    } else {
        r#"<a href="/inventory?low_stock=1" class="px-4 py-2 text-sm bg-gray-100 text-gray-600 hover:bg-gray-200 rounded-lg">仅显示低库存</a>"#
    };

    let total_pages = ((result.pagination.total as f64) / (page_size as f64)).ceil() as u32;
    let pagination = if total_pages > 1 {
        let prev_btn = if page > 1 {
            format!(r#"<a href="/inventory?page={}{}" class="px-4 py-2 text-sm text-gray-600 hover:bg-gray-100 rounded-lg">上一页</a>"#, page - 1, if query.low_stock.unwrap_or(false) { "&low_stock=1" } else { "" })
        } else {
            String::new()
        };
        let next_btn = if page < total_pages {
            format!(r#"<a href="/inventory?page={}{}" class="px-4 py-2 text-sm text-gray-600 hover:bg-gray-100 rounded-lg">下一页</a>"#, page + 1, if query.low_stock.unwrap_or(false) { "&low_stock=1" } else { "" })
        } else {
            String::new()
        };
        format!(
            r#"<div class="flex items-center justify-between mt-4">
                <p class="text-sm text-gray-600">共 {} 条记录，第 {}/{} 页</p>
                <div class="flex items-center gap-2">{}{}</div>
            </div>"#,
            result.pagination.total, page, total_pages, prev_btn, next_btn
        )
    } else {
        String::new()
    };

    let content = format!(
        r#"<!-- 页面标题 -->
<div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4 mb-6">
    <div>
        <h1 class="text-xl sm:text-2xl font-bold text-gray-800">库存管理</h1>
        <p class="text-gray-600 mt-1 text-sm sm:text-base">查看和管理产品库存</p>
    </div>
    <div class="flex items-center gap-2">
        {}
    </div>
</div>

<!-- 库存统计 -->
<div class="grid grid-cols-2 md:grid-cols-4 gap-3 sm:gap-4 mb-6">
    <div class="bg-white rounded-lg shadow-sm border border-gray-100 p-3 sm:p-4">
        <div class="flex items-center gap-2 sm:gap-3">
            <div class="w-8 h-8 sm:w-10 sm:h-10 bg-blue-100 rounded-lg flex items-center justify-center"><span class="text-lg sm:text-xl">📦</span></div>
            <div><p class="text-xs sm:text-sm text-gray-500">总SKU数</p><p class="text-lg sm:text-xl font-bold text-gray-800">{}</p></div>
        </div>
    </div>
    <div class="bg-white rounded-lg shadow-sm border border-gray-100 p-3 sm:p-4">
        <div class="flex items-center gap-2 sm:gap-3">
            <div class="w-8 h-8 sm:w-10 sm:h-10 bg-green-100 rounded-lg flex items-center justify-center"><span class="text-lg sm:text-xl">✅</span></div>
            <div><p class="text-xs sm:text-sm text-gray-500">库存正常</p><p class="text-lg sm:text-xl font-bold text-green-600">{}</p></div>
        </div>
    </div>
    <div class="bg-white rounded-lg shadow-sm border border-gray-100 p-3 sm:p-4">
        <div class="flex items-center gap-2 sm:gap-3">
            <div class="w-8 h-8 sm:w-10 sm:h-10 bg-yellow-100 rounded-lg flex items-center justify-center"><span class="text-lg sm:text-xl">⚠️</span></div>
            <div><p class="text-xs sm:text-sm text-gray-500">低库存</p><p class="text-lg sm:text-xl font-bold text-yellow-600">{}</p></div>
        </div>
    </div>
    <div class="bg-white rounded-lg shadow-sm border border-gray-100 p-3 sm:p-4">
        <div class="flex items-center gap-2 sm:gap-3">
            <div class="w-8 h-8 sm:w-10 sm:h-10 bg-red-100 rounded-lg flex items-center justify-center"><span class="text-lg sm:text-xl">🚫</span></div>
            <div><p class="text-xs sm:text-sm text-gray-500">缺货</p><p class="text-lg sm:text-xl font-bold text-red-600">{}</p></div>
        </div>
    </div>
</div>

<!-- 库存表格 -->
<div class="bg-white rounded-xl shadow-sm border border-gray-100 overflow-hidden">
    <div class="overflow-x-auto">
        <table class="w-full min-w-[900px]">
            <thead class="bg-gray-50 border-b border-gray-200">
                <tr>
                    <th class="px-4 sm:px-6 py-4 text-left text-sm font-semibold text-gray-700">SKU编码</th>
                    <th class="px-4 sm:px-6 py-4 text-left text-sm font-semibold text-gray-700">产品名称</th>
                    <th class="px-4 sm:px-6 py-4 text-left text-sm font-semibold text-gray-700">规格</th>
                    <th class="px-4 sm:px-6 py-4 text-right text-sm font-semibold text-gray-700">总库存</th>
                    <th class="px-4 sm:px-6 py-4 text-right text-sm font-semibold text-gray-700">可用</th>
                    <th class="px-4 sm:px-6 py-4 text-right text-sm font-semibold text-gray-700">锁定</th>
                    <th class="px-4 sm:px-6 py-4 text-right text-sm font-semibold text-gray-700">安全库存</th>
                    <th class="px-4 sm:px-6 py-4 text-center text-sm font-semibold text-gray-700">状态</th>
                    <th class="px-4 sm:px-6 py-4 text-center text-sm font-semibold text-gray-700">操作</th>
                </tr>
            </thead>
            <tbody class="divide-y divide-gray-100">{}</tbody>
        </table>
    </div>
    {}
</div>"#,
        low_stock_filter,
        total_count,
        normal_count,
        low_stock_count,
        out_of_stock_count,
        rows,
        pagination
    );

    render_layout("库存管理", "inventory", Some(user), &content)
}

// ============================================================================
// 客户管理
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct CustomersQuery {
    page: Option<u32>,
    keyword: Option<String>,
    status: Option<i64>,
}

/// 客户页面
pub async fn customers_page(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Query(query): Query<CustomersQuery>,
) -> Html<String> {
    let user = get_user_from_extension(&auth_user);
    let page = query.page.unwrap_or(1).max(1);
    let page_size = 20;

    let queries = CustomerQueries::new(state.db.pool());
    let result = queries.list(
        page,
        page_size,
        None,
        query.status,
        None,
        query.keyword.as_deref(),
    ).await.unwrap_or_else(|_| PagedResponse::new(vec![], page, page_size, 0));

    let rows: String = result.items.iter().map(|c| {
        let status_badge = match c.status {
            1 => r#"<span class="px-2 py-1 text-xs bg-green-100 text-green-700 rounded-full">正常</span>"#,
            2 => r#"<span class="px-2 py-1 text-xs bg-yellow-100 text-yellow-700 rounded-full">冻结</span>"#,
            _ => r#"<span class="px-2 py-1 text-xs bg-red-100 text-red-700 rounded-full">黑名单</span>"#,
        };
        format!(
            r#"<tr class="hover:bg-gray-50">
                <td class="px-4 sm:px-6 py-4"><span class="font-mono text-sm">{}</span></td>
                <td class="px-4 sm:px-6 py-4 font-medium">{}</td>
                <td class="px-4 sm:px-6 py-4">{}</td>
                <td class="px-4 sm:px-6 py-4">{}</td>
                <td class="px-4 sm:px-6 py-4 text-center">{}</td>
                <td class="px-4 sm:px-6 py-4 text-center">
                    <a href="/customers/{}" class="text-blue-600 hover:text-blue-800 text-sm mr-2">查看</a>
                    <a href="/customers/{}/edit" class="text-green-600 hover:text-green-800 text-sm">编辑</a>
                </td>
            </tr>"#,
            c.customer_code,
            c.name,
            c.mobile.as_deref().unwrap_or("-"),
            c.email.as_deref().unwrap_or("-"),
            status_badge,
            c.id,
            c.id
        )
    }).collect();

    let rows = if rows.is_empty() {
        r#"<tr><td colspan="6" class="px-6 py-12 text-center"><div class="text-gray-500"><p class="text-4xl mb-2">👥</p><p>暂无客户数据</p><a href="/customers/new" class="text-blue-500 hover:text-blue-600 mt-2 inline-block">添加第一个客户</a></div></td></tr>"#.to_string()
    } else {
        rows
    };

    let total_pages = ((result.pagination.total as f64) / (page_size as f64)).ceil() as u32;
    let pagination = if total_pages > 1 {
        format!(r#"<div class="mt-4 text-sm text-gray-600">共 {} 条，第 {}/{} 页</div>"#, result.pagination.total, page, total_pages)
    } else {
        String::new()
    };

    let content = format!(
        r#"<!-- 页面标题 -->
<div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4 mb-6">
    <div>
        <h1 class="text-xl sm:text-2xl font-bold text-gray-800">客户管理</h1>
        <p class="text-gray-600 mt-1 text-sm sm:text-base">管理所有客户信息</p>
    </div>
    <a href="/customers/new" class="inline-flex items-center justify-center gap-2 px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors w-full sm:w-auto">
        <span>+</span><span>新增客户</span>
    </a>
</div>

<!-- 搜索栏 -->
<div class="bg-white rounded-xl shadow-sm border border-gray-100 p-4 mb-6">
    <form action="/customers" method="GET" class="flex flex-col sm:flex-row gap-3 sm:gap-4">
        <div class="flex-1">
            <input type="text" name="keyword" value="{}" placeholder="搜索客户名称、手机号..."
                   class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent">
        </div>
        <button type="submit" class="px-6 py-2 bg-gray-100 text-gray-700 rounded-lg hover:bg-gray-200 transition-colors w-full sm:w-auto">搜索</button>
    </form>
</div>

<!-- 客户表格 -->
<div class="bg-white rounded-xl shadow-sm border border-gray-100 overflow-hidden">
    <div class="overflow-x-auto">
        <table class="w-full min-w-[700px]">
            <thead class="bg-gray-50 border-b border-gray-200">
                <tr>
                    <th class="px-4 sm:px-6 py-4 text-left text-sm font-semibold text-gray-700">客户编码</th>
                    <th class="px-4 sm:px-6 py-4 text-left text-sm font-semibold text-gray-700">姓名</th>
                    <th class="px-4 sm:px-6 py-4 text-left text-sm font-semibold text-gray-700">手机号</th>
                    <th class="px-4 sm:px-6 py-4 text-left text-sm font-semibold text-gray-700">邮箱</th>
                    <th class="px-4 sm:px-6 py-4 text-center text-sm font-semibold text-gray-700">状态</th>
                    <th class="px-4 sm:px-6 py-4 text-center text-sm font-semibold text-gray-700">操作</th>
                </tr>
            </thead>
            <tbody class="divide-y divide-gray-100">{}</tbody>
        </table>
    </div>
    {}
</div>"#,
        query.keyword.as_deref().unwrap_or(""),
        rows,
        pagination
    );

    render_layout("客户管理", "customers", Some(user), &content)
}

// ============================================================================
// 供应商管理
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct SuppliersQuery {
    page: Option<u32>,
    keyword: Option<String>,
}

/// 供应商页面
pub async fn suppliers_page(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Query(query): Query<SuppliersQuery>,
) -> Html<String> {
    let user = get_user_from_extension(&auth_user);
    let page = query.page.unwrap_or(1).max(1);
    let page_size = 20;

    let queries = SupplierQueries::new(state.db.pool());
    let result = queries.list(
        page,
        page_size,
        None,
        None,
        query.keyword.as_deref(),
    ).await.unwrap_or_else(|_| PagedResponse::new(vec![], page, page_size, 0));

    let rows: String = result.items.iter().map(|s| {
        let status_badge = match s.status {
            1 => r#"<span class="px-2 py-1 text-xs bg-green-100 text-green-700 rounded-full">合作中</span>"#,
            2 => r#"<span class="px-2 py-1 text-xs bg-yellow-100 text-yellow-700 rounded-full">暂停</span>"#,
            _ => r#"<span class="px-2 py-1 text-xs bg-red-100 text-red-700 rounded-full">终止</span>"#,
        };
        format!(
            r#"<tr class="hover:bg-gray-50">
                <td class="px-4 sm:px-6 py-4"><span class="font-mono text-sm">{}</span></td>
                <td class="px-4 sm:px-6 py-4 font-medium">{}</td>
                <td class="px-4 sm:px-6 py-4">{}</td>
                <td class="px-4 sm:px-6 py-4">{}</td>
                <td class="px-4 sm:px-6 py-4 text-center">{}</td>
                <td class="px-4 sm:px-6 py-4 text-center">
                    <a href="/suppliers/{}" class="text-blue-600 hover:text-blue-800 text-sm mr-2">查看</a>
                    <a href="/suppliers/{}/edit" class="text-green-600 hover:text-green-800 text-sm">编辑</a>
                </td>
            </tr>"#,
            s.supplier_code,
            s.name,
            s.contact_person.as_deref().unwrap_or("-"),
            s.contact_phone.as_deref().unwrap_or("-"),
            status_badge,
            s.id,
            s.id
        )
    }).collect();

    let rows = if rows.is_empty() {
        r#"<tr><td colspan="6" class="px-6 py-12 text-center"><div class="text-gray-500"><p class="text-4xl mb-2">🏭</p><p>暂无供应商数据</p><a href="/suppliers/new" class="text-blue-500 hover:text-blue-600 mt-2 inline-block">添加第一个供应商</a></div></td></tr>"#.to_string()
    } else {
        rows
    };

    let total_pages = ((result.pagination.total as f64) / (page_size as f64)).ceil() as u32;
    let pagination = if total_pages > 1 {
        format!(r#"<div class="mt-4 text-sm text-gray-600">共 {} 条，第 {}/{} 页</div>"#, result.pagination.total, page, total_pages)
    } else {
        String::new()
    };

    let content = format!(
        r#"<!-- 页面标题 -->
<div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4 mb-6">
    <div>
        <h1 class="text-xl sm:text-2xl font-bold text-gray-800">供应商管理</h1>
        <p class="text-gray-600 mt-1 text-sm sm:text-base">管理所有供应商信息</p>
    </div>
    <a href="/suppliers/new" class="inline-flex items-center justify-center gap-2 px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors w-full sm:w-auto">
        <span>+</span><span>新增供应商</span>
    </a>
</div>

<!-- 搜索栏 -->
<div class="bg-white rounded-xl shadow-sm border border-gray-100 p-4 mb-6">
    <form action="/suppliers" method="GET" class="flex flex-col sm:flex-row gap-3 sm:gap-4">
        <div class="flex-1">
            <input type="text" name="keyword" value="{}" placeholder="搜索供应商名称..."
                   class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent">
        </div>
        <button type="submit" class="px-6 py-2 bg-gray-100 text-gray-700 rounded-lg hover:bg-gray-200 transition-colors w-full sm:w-auto">搜索</button>
    </form>
</div>

<!-- 供应商表格 -->
<div class="bg-white rounded-xl shadow-sm border border-gray-100 overflow-hidden">
    <div class="overflow-x-auto">
        <table class="w-full min-w-[700px]">
            <thead class="bg-gray-50 border-b border-gray-200">
                <tr>
                    <th class="px-4 sm:px-6 py-4 text-left text-sm font-semibold text-gray-700">供应商编码</th>
                    <th class="px-4 sm:px-6 py-4 text-left text-sm font-semibold text-gray-700">名称</th>
                    <th class="px-4 sm:px-6 py-4 text-left text-sm font-semibold text-gray-700">联系人</th>
                    <th class="px-4 sm:px-6 py-4 text-left text-sm font-semibold text-gray-700">联系电话</th>
                    <th class="px-4 sm:px-6 py-4 text-center text-sm font-semibold text-gray-700">状态</th>
                    <th class="px-4 sm:px-6 py-4 text-center text-sm font-semibold text-gray-700">操作</th>
                </tr>
            </thead>
            <tbody class="divide-y divide-gray-100">{}</tbody>
        </table>
    </div>
    {}
</div>"#,
        query.keyword.as_deref().unwrap_or(""),
        rows,
        pagination
    );

    render_layout("供应商管理", "suppliers", Some(user), &content)
}

// ============================================================================
// 订单新增/详情页面
// ============================================================================

/// 新增订单页面
pub async fn order_new_page(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
) -> Html<String> {
    let user = get_user_from_extension(&auth_user);

    // 获取客户列表用于选择
    let customer_queries = CustomerQueries::new(state.db.pool());
    let customers_result = customer_queries.list(1, 100, None, None, None, None).await
        .unwrap_or_else(|_| PagedResponse::new(vec![], 1, 100, 0));
    let customers = customers_result.items;

    // 获取产品列表用于选择
    let product_queries = ProductQueries::new(state.db.pool());
    let products = product_queries.list(1, 100, None, None, None, None).await
        .map(|r| r.items).unwrap_or_default();

    // 生成客户选项
    let customer_options: String = customers.iter().map(|c| {
        format!(r#"<option value="{}" data-name="{}" data-email="{}" data-mobile="{}">{}</option>"#,
            c.id, c.name, c.email.as_deref().unwrap_or(""), c.mobile.as_deref().unwrap_or(""), c.name)
    }).collect();

    // 生成产品选项（使用 sale_price_cny 字段）
    let product_options: String = products.iter().map(|p| {
        let price = p.sale_price_cny.unwrap_or(0.0);
        format!(r#"<option value="{}" data-name="{}" data-price="{}">{}</option>"#,
            p.id, p.name, price, p.name)
    }).collect();

    let content = format!(
        r#"<!-- 页面标题 -->
<div class="mb-6">
    <div class="flex items-center gap-2 text-sm text-gray-500 mb-2">
        <a href="/orders" class="hover:text-blue-600">订单列表</a>
        <span>/</span>
        <span class="text-gray-800">新建订单</span>
    </div>
    <h1 class="text-xl sm:text-2xl font-bold text-gray-800">新建订单</h1>
</div>

<!-- 订单表单 -->
<div class="bg-white rounded-xl shadow-sm border border-gray-100">
    <form id="orderForm" action="/orders/new" method="POST" class="p-4 sm:p-6">
        <div class="space-y-6">
            <!-- 客户信息 -->
            <div class="border-b border-gray-100 pb-6">
                <h3 class="text-base font-semibold text-gray-800 mb-4">👤 客户信息</h3>
                <div class="grid grid-cols-1 md:grid-cols-2 gap-4 mb-4">
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-2">选择已有客户</label>
                        <select id="customerSelect" class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500" onchange="fillCustomerInfo()">
                            <option value="">-- 手动输入客户信息 --</option>
                            {}
                        </select>
                    </div>
                </div>
                <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
                    <input type="hidden" name="customer_id" id="customerId">
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-2">客户姓名</label>
                        <input type="text" name="customer_name" id="customerName" class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500" placeholder="客户姓名">
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-2">联系电话</label>
                        <input type="text" name="customer_mobile" id="customerMobile" class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500" placeholder="联系电话">
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-2">邮箱</label>
                        <input type="email" name="customer_email" id="customerEmail" class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500" placeholder="邮箱地址">
                    </div>
                </div>
            </div>

            <!-- 收货地址 -->
            <div class="border-b border-gray-100 pb-6">
                <h3 class="text-base font-semibold text-gray-800 mb-4">📍 收货地址</h3>
                <div id="addressSelectContainer" class="mb-4 hidden">
                    <label class="block text-sm font-medium text-gray-700 mb-2">选择已有地址</label>
                    <select id="addressSelect" class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500" onchange="fillAddressInfo()">
                        <option value="">-- 手动输入地址 --</option>
                    </select>
                    <p id="noAddressTip" class="text-sm text-gray-500 mt-1 hidden">该客户暂无收货地址，请手动填写</p>
                </div>
                <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-2">收件人 <span class="text-red-500">*</span></label>
                        <input type="text" name="receiver_name" id="receiverName" required class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500">
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-2">手机号 <span class="text-red-500">*</span></label>
                        <input type="text" name="receiver_phone" id="receiverPhone" required class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500">
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-2">国家 <span class="text-red-500">*</span></label>
                        <input type="text" name="country" id="country" required value="US" class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500">
                    </div>
                    <div class="md:col-span-2">
                        <label class="block text-sm font-medium text-gray-700 mb-2">详细地址 <span class="text-red-500">*</span></label>
                        <input type="text" name="address" id="address" required class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500">
                    </div>
                    <div class="md:col-span-2">
                        <label class="flex items-center gap-2">
                            <input type="checkbox" name="save_address" id="saveAddress" class="rounded border-gray-300">
                            <span class="text-sm text-gray-600">保存到客户地址列表</span>
                        </label>
                    </div>
                </div>
            </div>

            <!-- 订单商品（支持动态添加） -->
            <div class="border-b border-gray-100 pb-6">
                <div class="flex items-center justify-between mb-4">
                    <h3 class="text-base font-semibold text-gray-800">📦 订单商品</h3>
                    <button type="button" onclick="addItem()" class="px-3 py-1 text-sm bg-blue-100 text-blue-700 rounded-lg hover:bg-blue-200">+ 添加商品</button>
                </div>
                <div id="itemsContainer">
                    <div class="item-row grid grid-cols-12 gap-2 mb-2 items-end">
                        <div class="col-span-5">
                            <label class="block text-sm font-medium text-gray-700 mb-1">商品</label>
                            <select name="item_product[]" class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm" onchange="updateItemPrice(this)">
                                <option value="">-- 选择商品 --</option>
                                {}
                            </select>
                        </div>
                        <div class="col-span-2">
                            <label class="block text-sm font-medium text-gray-700 mb-1">数量</label>
                            <input type="number" name="item_quantity[]" min="1" value="1" class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm" onchange="calculateTotal()">
                        </div>
                        <div class="col-span-3">
                            <label class="block text-sm font-medium text-gray-700 mb-1">单价(USD) <span class="price-tip text-xs text-gray-400 font-normal">(参考价格)</span></label>
                            <div class="flex gap-1">
                                <input type="number" name="item_price[]" step="0.01" min="0" placeholder="请输入报价" class="flex-1 px-3 py-2 border border-gray-300 rounded-lg text-sm" onchange="calculateTotal()">
                                <button type="button" onclick="showHistoryPrice(this)" class="px-2 py-2 text-xs bg-gray-100 text-gray-600 rounded-lg hover:bg-gray-200" title="查看历史成交价格">📜</button>
                            </div>
                        </div>
                        <div class="col-span-1">
                            <label class="block text-sm font-medium text-gray-700 mb-1">小计</label>
                            <input type="text" name="item_subtotal[]" readonly class="w-full px-3 py-2 border border-gray-200 rounded-lg text-sm bg-gray-50">
                        </div>
                        <div class="col-span-1">
                            <button type="button" onclick="removeItem(this)" class="w-full px-3 py-2 text-red-600 hover:bg-red-50 rounded-lg text-sm">✕</button>
                        </div>
                    </div>
                </div>
            </div>

            <!-- 金额汇总 -->
            <div class="border-b border-gray-100 pb-6">
                <h3 class="text-base font-semibold text-gray-800 mb-4">💰 金额汇总</h3>
                <div class="grid grid-cols-1 md:grid-cols-4 gap-4">
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-2">商品总额</label>
                        <input type="text" id="subtotal" readonly class="w-full px-4 py-2 border border-gray-200 rounded-lg bg-gray-50" value="$0.00">
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-2">运费</label>
                        <input type="number" name="shipping_fee" step="0.01" min="0" value="0" class="w-full px-4 py-2 border border-gray-300 rounded-lg" onchange="calculateTotal()">
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-2">优惠</label>
                        <input type="number" name="discount_amount" step="0.01" min="0" value="0" class="w-full px-4 py-2 border border-gray-300 rounded-lg" onchange="calculateTotal()">
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-2">订单总额</label>
                        <input type="text" id="totalAmount" readonly class="w-full px-4 py-2 border border-blue-300 rounded-lg bg-blue-50 text-blue-700 font-semibold" value="$0.00">
                    </div>
                </div>
            </div>

            <!-- 条款信息 -->
            <div class="border-b border-gray-100 pb-6">
                <h3 class="text-base font-semibold text-gray-800 mb-4">📝 条款信息</h3>
                <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-2">付款条款</label>
                        <input type="text" name="payment_terms" value="100% before shipment" class="w-full px-4 py-2 border border-gray-300 rounded-lg">
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-2">交货条款</label>
                        <input type="text" name="delivery_terms" value="EXW" class="w-full px-4 py-2 border border-gray-300 rounded-lg">
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-2">交货期</label>
                        <input type="text" name="lead_time" value="3-7 working days" class="w-full px-4 py-2 border border-gray-300 rounded-lg">
                    </div>
                </div>
            </div>

            <!-- 备注 -->
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-2">备注</label>
                <textarea name="customer_note" rows="2" class="w-full px-4 py-2 border border-gray-300 rounded-lg" placeholder="订单备注..."></textarea>
            </div>
        </div>

        <!-- 提交按钮 -->
        <div class="mt-6 flex items-center gap-4">
            <button type="submit" class="px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors">创建订单（未成交状态）</button>
            <a href="/orders" class="px-6 py-2 bg-gray-100 text-gray-700 rounded-lg hover:bg-gray-200 transition-colors">取消</a>
        </div>
    </form>
</div>

<script>
// 客户地址缓存
let customerAddresses = [];

// 填充客户信息
function fillCustomerInfo() {{
    const select = document.getElementById('customerSelect');
    const option = select.options[select.selectedIndex];
    if (option.value) {{
        document.getElementById('customerId').value = option.value;
        document.getElementById('customerName').value = option.dataset.name || '';
        document.getElementById('customerEmail').value = option.dataset.email || '';
        document.getElementById('customerMobile').value = option.dataset.mobile || '';
        // 加载客户地址
        loadCustomerAddresses(option.value);
    }} else {{
        document.getElementById('customerId').value = '';
        // 隐藏地址选择框
        document.getElementById('addressSelectContainer').classList.add('hidden');
        clearAddressFields();
    }}
}}

// 加载客户地址
async function loadCustomerAddresses(customerId) {{
    const container = document.getElementById('addressSelectContainer');
    const select = document.getElementById('addressSelect');
    const noTip = document.getElementById('noAddressTip');

    try {{
        const response = await fetch('/api/v1/customers/' + customerId + '/addresses');
        const result = await response.json();

        if (result.code === 200 && result.data && result.data.length > 0) {{
            customerAddresses = result.data;
            container.classList.remove('hidden');
            noTip.classList.add('hidden');

            // 填充地址选项
            select.innerHTML = '<option value="">-- 手动输入地址 --</option>';
            result.data.forEach(addr => {{
                const fullAddr = (addr.country || '') + (addr.province || '') + (addr.city || '') + (addr.district || '') + addr.address;
                const label = addr.receiver_name + ' / ' + fullAddr.substring(0, 30) + (fullAddr.length > 30 ? '...' : '') + (addr.is_default ? ' [默认]' : '');
                const opt = document.createElement('option');
                opt.value = addr.id;
                opt.textContent = label;
                opt.dataset.receiver_name = addr.receiver_name;
                opt.dataset.receiver_phone = addr.receiver_phone;
                opt.dataset.country = addr.country || '';
                opt.dataset.province = addr.province || '';
                opt.dataset.city = addr.city || '';
                opt.dataset.district = addr.district || '';
                opt.dataset.address = addr.address;
                opt.dataset.is_default = addr.is_default;
                select.appendChild(opt);
            }});

            // 自动选择默认地址
            const defaultAddr = result.data.find(a => a.is_default);
            if (defaultAddr) {{
                select.value = defaultAddr.id;
                fillAddressInfo();
            }}
        }} else {{
            customerAddresses = [];
            container.classList.remove('hidden');
            noTip.classList.remove('hidden');
            select.innerHTML = '<option value="">-- 手动输入地址 --</option>';
            clearAddressFields();
        }}
    }} catch (e) {{
        console.error('加载地址失败:', e);
        container.classList.add('hidden');
    }}
}}

// 填充地址信息
function fillAddressInfo() {{
    const select = document.getElementById('addressSelect');
    const option = select.options[select.selectedIndex];

    if (option.value) {{
        document.getElementById('receiverName').value = option.dataset.receiver_name || '';
        document.getElementById('receiverPhone').value = option.dataset.receiver_phone || '';
        document.getElementById('country').value = option.dataset.country || 'US';
        document.getElementById('address').value = option.dataset.address || '';
    }} else {{
        clearAddressFields();
    }}
}}

// 清空地址字段
function clearAddressFields() {{
    document.getElementById('receiverName').value = '';
    document.getElementById('receiverPhone').value = '';
    document.getElementById('country').value = 'US';
    document.getElementById('address').value = '';
}}

// 更新商品单价
async function updateItemPrice(select) {{
    const row = select.closest('.item-row');
    const option = select.options[select.selectedIndex];
    const priceInput = row.querySelector('input[name="item_price[]"]');
    const priceTip = row.querySelector('.price-tip');

    if (option.value) {{
        // 如果有预设价格，先设置
        if (option.dataset.price) {{
            priceInput.value = option.dataset.price;
        }}

        // 获取参考价格
        try {{
            const response = await fetch('/api/v1/products/' + option.value + '/price-summary');
            const result = await response.json();
            if (result.code === 200 && result.data) {{
                const refPriceCny = result.data.reference_price_cny || result.data.avg_cost_cny;
                if (refPriceCny) {{
                    // 假设汇率7.2，转换为USD
                    const refPriceUsd = (refPriceCny / 7.2).toFixed(2);
                    priceInput.placeholder = '参考: $' + refPriceUsd;
                    if (priceTip) {{
                        priceTip.textContent = '(参考: $' + refPriceUsd + ')';
                    }}
                }} else {{
                    priceInput.placeholder = '请输入报价';
                    if (priceTip) {{
                        priceTip.textContent = '(参考价格)';
                    }}
                }}
            }}
        }} catch (e) {{
            console.error('获取参考价格失败:', e);
        }}
    }} else {{
        priceInput.value = '';
        priceInput.placeholder = '';
        if (priceTip) {{
            priceTip.textContent = '(参考价格)';
        }}
    }}
    calculateTotal();
}}

// 添加商品行
function addItem() {{
    const container = document.getElementById('itemsContainer');
    const row = container.querySelector('.item-row').cloneNode(true);
    // 清空值
    row.querySelector('select').value = '';
    row.querySelector('input[name="item_quantity[]"]').value = '1';
    row.querySelector('input[name="item_price[]"]').value = '';
    row.querySelector('input[name="item_subtotal[]"]').value = '';
    container.appendChild(row);
}}

// 删除商品行
function removeItem(btn) {{
    const container = document.getElementById('itemsContainer');
    if (container.querySelectorAll('.item-row').length > 1) {{
        btn.closest('.item-row').remove();
        calculateTotal();
    }}
}}

// 计算总额
function calculateTotal() {{
    let subtotal = 0;
    const rows = document.querySelectorAll('.item-row');
    rows.forEach(row => {{
        const qty = parseFloat(row.querySelector('input[name="item_quantity[]"]').value) || 0;
        const price = parseFloat(row.querySelector('input[name="item_price[]"]').value) || 0;
        const itemSubtotal = qty * price;
        row.querySelector('input[name="item_subtotal[]"]').value = '$' + itemSubtotal.toFixed(2);
        subtotal += itemSubtotal;
    }});

    const shipping = parseFloat(document.querySelector('input[name="shipping_fee"]').value) || 0;
    const discount = parseFloat(document.querySelector('input[name="discount_amount"]').value) || 0;
    const total = subtotal + shipping - discount;

    document.getElementById('subtotal').value = '$' + subtotal.toFixed(2);
    document.getElementById('totalAmount').value = '$' + total.toFixed(2);
}}

// 页面加载时计算一次
calculateTotal();

// 显示历史成交价格
async function showHistoryPrice(btn) {{
    const row = btn.closest('.item-row');
    const select = row.querySelector('select[name="item_product[]"]');
    const productId = select.value;

    if (!productId) {{
        alert('请先选择商品');
        return;
    }}

    try {{
        const response = await fetch('/api/v1/products/' + productId + '/history-prices?limit=10');
        const result = await response.json();

        if (result.code === 200 && result.data && result.data.length > 0) {{
            let msg = '📜 历史成交价格\\n\\n';
            result.data.forEach(p => {{
                const date = new Date(p.order_date).toLocaleDateString();
                const customer = p.customer_name || '未知客户';
                msg += `${{date}} | $${{p.unit_price.toFixed(2)}} | ${{p.quantity}}件 | ${{customer}}\\n`;
            }});
            alert(msg);
        }} else {{
            alert('暂无历史成交记录');
        }}
    }} catch (e) {{
        console.error('获取历史价格失败:', e);
        alert('获取历史价格失败');
    }}
}}
</script>"#,
        customer_options,
        product_options
    );

    render_layout("新建订单", "orders", Some(user), &content)
}

/// 创建订单表单
#[derive(Debug, Deserialize)]
pub struct OrderForm {
    customer_id: Option<i64>,
    customer_name: Option<String>,
    customer_mobile: Option<String>,
    customer_email: Option<String>,
    receiver_name: String,
    receiver_phone: String,
    country: String,
    address: String,
    save_address: Option<String>, // 保存到客户地址列表
    // 多商品支持
    item_product: Vec<String>,
    item_quantity: Vec<i64>,
    item_price: Vec<f64>,
    shipping_fee: Option<f64>,
    discount_amount: Option<f64>,
    customer_note: Option<String>,
    payment_terms: Option<String>,
    delivery_terms: Option<String>,
    lead_time: Option<String>,
}

/// 创建订单处理
pub async fn order_create_handler(
    State(state): State<AppState>,
    Form(form): Form<OrderForm>,
) -> Result<impl IntoResponse, Html<String>> {
    let queries = OrderQueries::new(state.db.pool());

    // 处理客户信息 - 自动创建客户
    let mut customer_id = form.customer_id;

    // 如果没有 customer_id 但有客户手机号，检查或创建客户
    if customer_id.is_none() {
        if let Some(ref mobile) = form.customer_mobile {
            if !mobile.is_empty() {
                let customer_queries = CustomerQueries::new(state.db.pool());

                // 先查找是否存在
                let existing = customer_queries.get_by_mobile(mobile).await.ok().flatten();

                if let Some(customer) = existing {
                    // 客户已存在，使用其 ID
                    customer_id = Some(customer.id);
                    info!("Found existing customer: id={}, mobile={}", customer.id, mobile);
                } else if let Some(ref name) = form.customer_name {
                    // 客户不存在，自动创建
                    let create_req = CreateCustomerRequest {
                        name: name.clone(),
                        mobile: mobile.clone(),
                        email: form.customer_email.clone(),
                        status: Some(1),
                        notes: Some("Auto-created from order".to_string()),
                        source: Some("order".to_string()),
                    };

                    match customer_queries.create(&create_req).await {
                        Ok(new_customer) => {
                            customer_id = Some(new_customer.id);
                            info!("Auto-created customer: id={}, name={}, mobile={}",
                                new_customer.id, name, mobile);
                        }
                        Err(e) => {
                            info!("Failed to auto-create customer: {}", e);
                            // 继续创建订单，只是不关联客户
                        }
                    }
                }
            }
        }
    }

    // 构建商品列表
    let mut items: Vec<OrderItemRequest> = Vec::new();
    for i in 0..form.item_product.len() {
        let product_id = form.item_product.get(i).and_then(|s| s.parse::<i64>().ok());
        let quantity = form.item_quantity.get(i).copied().unwrap_or(1);
        let unit_price = form.item_price.get(i).copied().unwrap_or(0.0);

        // 获取产品名称
        let product_name = if let Some(pid) = product_id {
            let product_queries = ProductQueries::new(state.db.pool());
            product_queries.get_by_id(pid).await.ok().flatten()
                .map(|p| p.name).unwrap_or_else(|| format!("Product #{}", pid))
        } else {
            format!("Item {}", i + 1)
        };

        items.push(OrderItemRequest {
            product_id,
            sku_id: None,
            product_name,
            product_code: None,
            sku_code: None,
            sku_spec: None,
            product_image: None,
            quantity,
            unit_price,
        });
    }

    // 如果没有商品，添加一个默认商品
    if items.is_empty() {
        items.push(OrderItemRequest {
            product_id: None,
            sku_id: None,
            product_name: "Default Item".to_string(),
            product_code: None,
            sku_code: None,
            sku_spec: None,
            product_image: None,
            quantity: 1,
            unit_price: 0.0,
        });
    }

    let req = CreateOrderRequest {
        platform: "manual".to_string(),
        platform_order_id: None,
        customer_id,
        customer_name: form.customer_name.clone(),
        customer_mobile: form.customer_mobile.clone(),
        customer_email: form.customer_email.clone(),
        order_type: Some(1),
        items,
        shipping_fee: form.shipping_fee,
        discount_amount: form.discount_amount,
        customer_note: form.customer_note.clone(),
        receiver_name: form.receiver_name.clone(),
        receiver_phone: form.receiver_phone.clone(),
        country: form.country.clone(),
        province: None,
        city: None,
        district: None,
        address: form.address.clone(),
        postal_code: None,
        payment_terms: form.payment_terms.clone(),
        delivery_terms: form.delivery_terms.clone(),
        lead_time: form.lead_time.clone(),
    };

    match queries.create(&req).await {
        Ok(order) => {
            info!("Order created: id={}, code={}", order.id, order.order_code);

            // 如果勾选了保存地址且有客户ID，保存到客户地址列表
            if form.save_address.is_some() {
                if let Some(cid) = customer_id {
                    let customer_queries = CustomerQueries::new(state.db.pool());
                    let address_req = CreateAddressRequest {
                        receiver_name: form.receiver_name.clone(),
                        receiver_phone: form.receiver_phone.clone(),
                        country: form.country.clone(),
                        country_code: None,
                        province: None,
                        city: None,
                        district: None,
                        address: form.address.clone(),
                        postal_code: None,
                        address_type: Some(1),
                        is_default: false,
                    };
                    match customer_queries.create_address(cid, &address_req).await {
                        Ok(_) => info!("Address saved to customer: customer_id={}", cid),
                        Err(e) => info!("Failed to save address: {}", e),
                    }
                }
            }

            Ok(Redirect::to(&format!("/orders/{}", order.id)))
        }
        Err(e) => {
            info!("Failed to create order: {}", e);
            Err(Html(format!(
                r#"<!DOCTYPE html><html><body><script>alert('创建订单失败: {}');history.back();</script></body></html>"#,
                e
            )))
        }
    }
}

/// 订单详情页面
pub async fn order_detail_page(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<i64>,
) -> Html<String> {
    let user = get_user_from_extension(&auth_user);

    let queries = OrderQueries::new(state.db.pool());
    let order = match queries.get_detail(id).await {
        Ok(Some(o)) => o,
        _ => {
            let content = r#"<div class="text-center py-12">
                <p class="text-4xl mb-4">📋</p>
                <p class="text-gray-600 mb-4">订单不存在</p>
                <a href="/orders" class="text-blue-600 hover:text-blue-800">返回订单列表</a>
            </div>"#;
            return render_layout("订单详情", "orders", Some(user), content);
        }
    };

    // 使用新的状态定义
    let status_text = crate::templates::orders::order_status_text(order.order.order_status);
    let status_class = crate::templates::orders::order_status_class(order.order.order_status);
    let status_badge = format!(r#"<span class="px-3 py-1 text-sm font-medium {} rounded-full">{}</span>"#, status_class, status_text);

    // 判断可下载的文件
    let can_download_pi = crate::templates::orders::can_download_pi(order.order.order_status);
    let can_download_ci = crate::templates::orders::can_download_ci(order.order.order_status);

    // 根据状态生成操作按钮
    let action_buttons = match order.order.order_status {
        1 => {
            // 未成交 - 可编辑、可下载PI，可锁定价格，可取消
            format!(r#"
                <a href="/orders/{}/edit" class="px-4 py-2 bg-orange-600 text-white rounded-lg hover:bg-orange-700 text-sm">编辑</a>
                <a href="/api/v1/orders/{}/download-pi" target="_blank" class="px-4 py-2 bg-green-600 text-white rounded-lg hover:bg-green-700 text-sm">下载 PI</a>
                <button onclick="changeStatus(2)" class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 text-sm">锁定价格</button>
                <button onclick="changeStatus(6)" class="px-4 py-2 bg-red-100 text-red-700 rounded-lg hover:bg-red-200 text-sm">取消订单</button>
            "#, order.order.id, order.order.id)
        }
        2 => {
            // 价格锁定 - 可下载PI，可标记已付款，可取消
            format!(r#"
                <a href="/api/v1/orders/{}/download-pi" target="_blank" class="px-4 py-2 bg-green-600 text-white rounded-lg hover:bg-green-700 text-sm">下载 PI</a>
                <button onclick="changeStatus(3)" class="px-4 py-2 bg-green-600 text-white rounded-lg hover:bg-green-700 text-sm">标记已付款</button>
                <button onclick="changeStatus(6)" class="px-4 py-2 bg-red-100 text-red-700 rounded-lg hover:bg-red-200 text-sm">取消订单</button>
            "#, order.order.id)
        }
        3 => {
            // 已付款 - 可下载CI，可标记已发货
            format!(r#"
                <a href="/api/v1/orders/{}/download-ci" target="_blank" class="px-4 py-2 bg-green-600 text-white rounded-lg hover:bg-green-700 text-sm">下载 CI</a>
                <button onclick="changeStatus(4)" class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 text-sm">标记已发货</button>
            "#, order.order.id)
        }
        4 => {
            // 已发货 - 可下载CI，可标记已收货
            format!(r#"
                <a href="/api/v1/orders/{}/download-ci" target="_blank" class="px-4 py-2 bg-green-600 text-white rounded-lg hover:bg-green-700 text-sm">下载 CI</a>
                <button onclick="changeStatus(5)" class="px-4 py-2 bg-green-600 text-white rounded-lg hover:bg-green-700 text-sm">标记已收货</button>
            "#, order.order.id)
        }
        5 => {
            // 已收货 - 可下载CI
            format!(r#"<a href="/api/v1/orders/{}/download-ci" target="_blank" class="px-4 py-2 bg-green-600 text-white rounded-lg hover:bg-green-700 text-sm">下载 CI</a>"#, order.order.id)
        }
        _ => String::new()
    };

    let payment_status = match order.order.payment_status {
        1 => r#"<span class="text-gray-500">未支付</span>"#,
        2 => r#"<span class="text-yellow-600">部分支付</span>"#,
        3 => r#"<span class="text-green-600">已支付</span>"#,
        4 => r#"<span class="text-red-600">已退款</span>"#,
        _ => r#"<span class="text-orange-600">部分退款</span>"#,
    };

    let items_html: String = order.items.iter().map(|item| {
        format!(
            r#"<tr class="hover:bg-gray-50">
                <td class="px-4 py-3">{}</td>
                <td class="px-4 py-3 text-right">{}</td>
                <td class="px-4 py-3 text-right">${:.2}</td>
                <td class="px-4 py-3 text-right">${:.2}</td>
            </tr>"#,
            item.product_name,
            item.quantity,
            item.unit_price,
            item.total_amount
        )
    }).collect();

    let address_html = if let Some(ref addr) = order.address {
        format!(
            r#"<div class="text-sm text-gray-600">
                <p><strong>收件人:</strong> {} {}</p>
                <p><strong>地址:</strong> {} {} {} {} {}</p>
            </div>"#,
            addr.receiver_name,
            addr.receiver_phone,
            addr.country,
            addr.province.as_deref().unwrap_or(""),
            addr.city.as_deref().unwrap_or(""),
            addr.district.as_deref().unwrap_or(""),
            addr.address
        )
    } else {
        r#"<p class="text-gray-500">暂无收货地址</p>"#.to_string()
    };

    // 状态流转提示
    let status_hint = match order.order.order_status {
        1 => r#"<div class="bg-blue-50 text-blue-700 p-3 rounded-lg text-sm">💡 提示: 确认价格后点击"锁定价格"，然后可以发送 PI 给客户</div>"#,
        2 => r#"<div class="bg-blue-50 text-blue-700 p-3 rounded-lg text-sm">💡 提示: 收到客户付款后点击"标记已付款"，然后可以下载 CI</div>"#,
        3 => r#"<div class="bg-blue-50 text-blue-700 p-3 rounded-lg text-sm">💡 提示: 发货后点击"标记已发货"</div>"#,
        4 => r#"<div class="bg-blue-50 text-blue-700 p-3 rounded-lg text-sm">💡 提示: 客户确认收货后点击"标记已收货"</div>"#,
        _ => ""
    };

    let content = format!(
        r#"<!-- 页面标题 -->
<div class="mb-6">
    <div class="flex items-center gap-2 text-sm text-gray-500 mb-2">
        <a href="/orders" class="hover:text-blue-600">订单列表</a>
        <span>/</span>
        <span class="text-gray-800">{}</span>
    </div>
    <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div class="flex items-center gap-3">
            <h1 class="text-xl sm:text-2xl font-bold text-gray-800">订单详情</h1>
            {}
        </div>
        <div class="flex flex-wrap gap-2">
            {}
            <a href="/orders" class="px-4 py-2 bg-gray-100 text-gray-700 rounded-lg hover:bg-gray-200 text-sm">返回列表</a>
        </div>
    </div>
</div>

<!-- 状态提示 -->
{}

<!-- 订单信息 -->
<div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
    <!-- 主信息 -->
    <div class="lg:col-span-2 space-y-6">
        <div class="bg-white rounded-xl shadow-sm border border-gray-100 p-4 sm:p-6">
            <h3 class="text-base font-semibold text-gray-800 mb-4">📦 订单商品</h3>
            <table class="w-full">
                <thead class="bg-gray-50">
                    <tr>
                        <th class="px-4 py-3 text-left text-sm font-semibold text-gray-700">商品</th>
                        <th class="px-4 py-3 text-right text-sm font-semibold text-gray-700">数量</th>
                        <th class="px-4 py-3 text-right text-sm font-semibold text-gray-700">单价</th>
                        <th class="px-4 py-3 text-right text-sm font-semibold text-gray-700">小计</th>
                    </tr>
                </thead>
                <tbody class="divide-y divide-gray-100">{}</tbody>
            </table>
        </div>

        <div class="bg-white rounded-xl shadow-sm border border-gray-100 p-4 sm:p-6">
            <h3 class="text-base font-semibold text-gray-800 mb-4">📍 收货地址</h3>
            {}
        </div>
    </div>

    <!-- 侧边信息 -->
    <div class="space-y-6">
        <div class="bg-white rounded-xl shadow-sm border border-gray-100 p-4 sm:p-6">
            <h3 class="text-base font-semibold text-gray-800 mb-4">💰 金额信息</h3>
            <div class="space-y-3">
                <div class="flex justify-between"><span class="text-gray-600">商品总额</span><span>${:.2}</span></div>
                <div class="flex justify-between"><span class="text-gray-600">运费</span><span>${:.2}</span></div>
                <div class="flex justify-between"><span class="text-gray-600">优惠</span><span>-${:.2}</span></div>
                <div class="flex justify-between pt-3 border-t border-gray-100 font-semibold">
                    <span>订单总额</span><span class="text-lg text-blue-600">${:.2}</span>
                </div>
                <div class="flex justify-between"><span class="text-gray-600">支付状态</span>{}</div>
            </div>
        </div>

        <div class="bg-white rounded-xl shadow-sm border border-gray-100 p-4 sm:p-6">
            <h3 class="text-base font-semibold text-gray-800 mb-4">📋 基本信息</h3>
            <div class="space-y-2 text-sm">
                <div class="flex justify-between"><span class="text-gray-500">订单号</span><span class="font-mono">{}</span></div>
                <div class="flex justify-between"><span class="text-gray-500">来源</span><span>{}</span></div>
                <div class="flex justify-between"><span class="text-gray-500">创建时间</span><span>{}</span></div>
            </div>
        </div>

        <!-- 可下载文件 -->
        <div class="bg-white rounded-xl shadow-sm border border-gray-100 p-4 sm:p-6">
            <h3 class="text-base font-semibold text-gray-800 mb-4">📄 可下载文件</h3>
            <div class="space-y-2">
                <div class="flex items-center justify-between">
                    <span class="text-gray-600">PI (形式发票)</span>
                    {}
                </div>
                <div class="flex items-center justify-between">
                    <span class="text-gray-600">CI (商业发票)</span>
                    {}
                </div>
            </div>
        </div>
    </div>
</div>

<script>
function changeStatus(newStatus) {{
    const statusNames = {{1: '未成交', 2: '价格锁定', 3: '已付款', 4: '已发货', 5: '已收货', 6: '已取消'}};
    if (confirm('确认要将订单状态改为 "' + statusNames[newStatus] + '" 吗？')) {{
        fetch('/api/v1/orders/' + {} + '/status', {{
            method: 'POST',
            headers: {{ 'Content-Type': 'application/json' }},
            body: JSON.stringify({{ status: newStatus }})
        }})
        .then(r => r.json())
        .then(data => {{
            if (data.code === 200) {{
                window.location.reload();
            }} else {{
                alert('操作失败: ' + (data.message || '未知错误'));
            }}
        }})
        .catch(e => alert('操作失败: ' + e));
    }}
}}
</script>"#,
        order.order.order_code,
        status_badge,
        action_buttons,
        status_hint,
        items_html,
        address_html,
        order.order.subtotal,
        order.order.shipping_fee,
        order.order.discount_amount,
        order.order.total_amount,
        payment_status,
        order.order.order_code,
        order.order.platform,
        order.order.created_at.format("%Y-%m-%d %H:%M"),
        if can_download_pi {
            format!(r#"<a href="/api/v1/orders/{}/download-pi" target="_blank" class="text-green-600 hover:text-green-800 text-sm">下载</a>"#, order.order.id)
        } else {
            r#"<span class="text-gray-400 text-sm">不可用</span>"#.to_string()
        },
        if can_download_ci {
            format!(r#"<a href="/api/v1/orders/{}/download-ci" target="_blank" class="text-green-600 hover:text-green-800 text-sm">下载</a>"#, order.order.id)
        } else {
            r#"<span class="text-gray-400 text-sm">不可用</span>"#.to_string()
        },
        order.order.id
    );

    render_layout("订单详情", "orders", Some(user), &content)
}

/// 订单编辑页面
pub async fn order_edit_page(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<i64>,
) -> Html<String> {
    let user = get_user_from_extension(&auth_user);

    let queries = OrderQueries::new(state.db.pool());
    let order = match queries.get_detail(id).await {
        Ok(Some(o)) if o.order.order_status == 1 => o, // 只有未成交状态可编辑
        _ => {
            let content = r#"<div class="text-center py-12">
                <p class="text-4xl mb-4">⚠️</p>
                <p class="text-gray-600 mb-4">订单不存在或不可编辑（只有未成交状态可编辑）</p>
                <a href="/orders" class="text-blue-600 hover:text-blue-800">返回订单列表</a>
            </div>"#;
            return render_layout("编辑订单", "orders", Some(user), content);
        }
    };

    // 获取客户列表
    let customer_queries = CustomerQueries::new(state.db.pool());
    let customers = customer_queries.list(1, 100, None, None, None, None).await
        .map(|r| r.items).unwrap_or_default();

    // 获取产品列表
    let product_queries = ProductQueries::new(state.db.pool());
    let products = product_queries.list(1, 100, None, None, None, None).await
        .map(|r| r.items).unwrap_or_default();

    let customer_options: String = customers.iter().map(|c| {
        let selected = order.order.customer_id.map(|id| id == c.id).unwrap_or(false);
        format!(r#"<option value="{}" data-name="{}" data-email="{}" data-mobile="{}" {}>{}</option>"#,
            c.id, c.name, c.email.as_deref().unwrap_or(""), c.mobile.as_deref().unwrap_or(""),
            if selected { "selected" } else { "" }, c.name)
    }).collect();

    let product_options: String = products.iter().map(|p| {
        let price = p.sale_price_cny.unwrap_or(0.0);
        format!(r#"<option value="{}" data-name="{}" data-price="{}">{}</option>"#,
            p.id, p.name, price, p.name)
    }).collect();

    // 商品行
    let items_html: String = order.items.iter().enumerate().map(|(i, item)| {
        format!(r#"<div class="item-row grid grid-cols-12 gap-2 mb-2 items-end">
            <div class="col-span-5">
                <label class="block text-sm font-medium text-gray-700 mb-1">商品</label>
                <select name="item_product[]" class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm" onchange="updateItemPrice(this)">
                    <option value="">-- 选择商品 --</option>
                    {}
                </select>
            </div>
            <div class="col-span-2">
                <label class="block text-sm font-medium text-gray-700 mb-1">数量</label>
                <input type="number" name="item_quantity[]" min="1" value="{}" class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm" onchange="calculateTotal()">
            </div>
            <div class="col-span-2">
                <label class="block text-sm font-medium text-gray-700 mb-1">单价(USD)</label>
                <input type="number" name="item_price[]" step="0.01" min="0" value="{:.2}" class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm" onchange="calculateTotal()">
            </div>
            <div class="col-span-2">
                <label class="block text-sm font-medium text-gray-700 mb-1">小计</label>
                <input type="text" name="item_subtotal[]" readonly class="w-full px-3 py-2 border border-gray-200 rounded-lg text-sm bg-gray-50" value="${:.2}">
            </div>
            <div class="col-span-1">
                <button type="button" onclick="removeItem(this)" class="w-full px-3 py-2 text-red-600 hover:bg-red-50 rounded-lg text-sm">✕</button>
            </div>
        </div>"#,
            product_options.clone(),
            item.quantity,
            item.unit_price,
            item.total_amount
        )
    }).collect();

    let content = format!(
        r#"<!-- 页面标题 -->
<div class="mb-6">
    <div class="flex items-center gap-2 text-sm text-gray-500 mb-2">
        <a href="/orders" class="hover:text-blue-600">订单列表</a>
        <span>/</span>
        <a href="/orders/{}" class="hover:text-blue-600">{}</a>
        <span>/</span>
        <span class="text-gray-800">编辑</span>
    </div>
    <h1 class="text-xl sm:text-2xl font-bold text-gray-800">编辑订单</h1>
</div>

<!-- 订单表单 -->
<div class="bg-white rounded-xl shadow-sm border border-gray-100">
    <form id="orderForm" action="/orders/{}/edit" method="POST" class="p-4 sm:p-6">
        <div class="space-y-6">
            <!-- 客户信息 -->
            <div class="border-b border-gray-100 pb-6">
                <h3 class="text-base font-semibold text-gray-800 mb-4">👤 客户信息</h3>
                <div class="grid grid-cols-1 md:grid-cols-2 gap-4 mb-4">
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-2">选择已有客户</label>
                        <select id="customerSelect" class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500" onchange="fillCustomerInfo()">
                            <option value="">-- 手动输入客户信息 --</option>
                            {}
                        </select>
                    </div>
                </div>
                <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
                    <input type="hidden" name="customer_id" id="customerId" value="{}">
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-2">客户姓名</label>
                        <input type="text" name="customer_name" id="customerName" value="{}" class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500">
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-2">联系电话</label>
                        <input type="text" name="customer_mobile" id="customerMobile" value="{}" class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500">
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-2">邮箱</label>
                        <input type="email" name="customer_email" id="customerEmail" value="{}" class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500">
                    </div>
                </div>
            </div>

            <!-- 收货地址 -->
            <div class="border-b border-gray-100 pb-6">
                <h3 class="text-base font-semibold text-gray-800 mb-4">📍 收货地址</h3>
                <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-2">收件人 <span class="text-red-500">*</span></label>
                        <input type="text" name="receiver_name" required value="{}" class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500">
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-2">手机号 <span class="text-red-500">*</span></label>
                        <input type="text" name="receiver_phone" required value="{}" class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500">
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-2">国家 <span class="text-red-500">*</span></label>
                        <input type="text" name="country" required value="{}" class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500">
                    </div>
                    <div class="md:col-span-2">
                        <label class="block text-sm font-medium text-gray-700 mb-2">详细地址 <span class="text-red-500">*</span></label>
                        <input type="text" name="address" required value="{}" class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500">
                    </div>
                </div>
            </div>

            <!-- 订单商品 -->
            <div class="border-b border-gray-100 pb-6">
                <div class="flex items-center justify-between mb-4">
                    <h3 class="text-base font-semibold text-gray-800">📦 订单商品</h3>
                    <button type="button" onclick="addItem()" class="px-3 py-1 text-sm bg-blue-100 text-blue-700 rounded-lg hover:bg-blue-200">+ 添加商品</button>
                </div>
                <div id="itemsContainer">{}</div>
            </div>

            <!-- 金额汇总 -->
            <div class="border-b border-gray-100 pb-6">
                <h3 class="text-base font-semibold text-gray-800 mb-4">💰 金额汇总</h3>
                <div class="grid grid-cols-1 md:grid-cols-4 gap-4">
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-2">商品总额</label>
                        <input type="text" id="subtotal" readonly class="w-full px-4 py-2 border border-gray-200 rounded-lg bg-gray-50" value="${:.2}">
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-2">运费</label>
                        <input type="number" name="shipping_fee" step="0.01" min="0" value="{:.2}" class="w-full px-4 py-2 border border-gray-300 rounded-lg" onchange="calculateTotal()">
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-2">优惠</label>
                        <input type="number" name="discount_amount" step="0.01" min="0" value="{:.2}" class="w-full px-4 py-2 border border-gray-300 rounded-lg" onchange="calculateTotal()">
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-2">订单总额</label>
                        <input type="text" id="totalAmount" readonly class="w-full px-4 py-2 border border-blue-300 rounded-lg bg-blue-50 text-blue-700 font-semibold" value="${:.2}">
                    </div>
                </div>
            </div>

            <!-- 条款信息 -->
            <div class="border-b border-gray-100 pb-6">
                <h3 class="text-base font-semibold text-gray-800 mb-4">📝 条款信息</h3>
                <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-2">付款条款</label>
                        <input type="text" name="payment_terms" value="{}" class="w-full px-4 py-2 border border-gray-300 rounded-lg">
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-2">交货条款</label>
                        <input type="text" name="delivery_terms" value="{}" class="w-full px-4 py-2 border border-gray-300 rounded-lg">
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-2">交货期</label>
                        <input type="text" name="lead_time" value="{}" class="w-full px-4 py-2 border border-gray-300 rounded-lg">
                    </div>
                </div>
            </div>

            <!-- 备注 -->
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-2">备注</label>
                <textarea name="customer_note" rows="2" class="w-full px-4 py-2 border border-gray-300 rounded-lg" placeholder="订单备注...">{}</textarea>
            </div>
        </div>

        <!-- 提交按钮 -->
        <div class="mt-6 flex items-center gap-4">
            <button type="submit" class="px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors">保存修改</button>
            <a href="/orders/{}" class="px-6 py-2 bg-gray-100 text-gray-700 rounded-lg hover:bg-gray-200 transition-colors">取消</a>
        </div>
    </form>
</div>

<script>
function fillCustomerInfo() {{
    const select = document.getElementById('customerSelect');
    const option = select.options[select.selectedIndex];
    if (option.value) {{
        document.getElementById('customerId').value = option.value;
        document.getElementById('customerName').value = option.dataset.name || '';
        document.getElementById('customerEmail').value = option.dataset.email || '';
        document.getElementById('customerMobile').value = option.dataset.mobile || '';
    }} else {{
        document.getElementById('customerId').value = '';
    }}
}}

function updateItemPrice(select) {{
    const row = select.closest('.item-row');
    const option = select.options[select.selectedIndex];
    const priceInput = row.querySelector('input[name="item_price[]"]');
    if (option.value && option.dataset.price) {{
        priceInput.value = option.dataset.price;
    }}
    calculateTotal();
}}

function addItem() {{
    const container = document.getElementById('itemsContainer');
    const row = container.querySelector('.item-row').cloneNode(true);
    row.querySelector('select').value = '';
    row.querySelector('input[name="item_quantity[]"]').value = '1';
    row.querySelector('input[name="item_price[]"]').value = '';
    row.querySelector('input[name="item_subtotal[]"]').value = '';
    container.appendChild(row);
}}

function removeItem(btn) {{
    const container = document.getElementById('itemsContainer');
    if (container.querySelectorAll('.item-row').length > 1) {{
        btn.closest('.item-row').remove();
        calculateTotal();
    }}
}}

function calculateTotal() {{
    let subtotal = 0;
    const rows = document.querySelectorAll('.item-row');
    rows.forEach(row => {{
        const qty = parseFloat(row.querySelector('input[name="item_quantity[]"]').value) || 0;
        const price = parseFloat(row.querySelector('input[name="item_price[]"]').value) || 0;
        const itemSubtotal = qty * price;
        row.querySelector('input[name="item_subtotal[]"]').value = '$' + itemSubtotal.toFixed(2);
        subtotal += itemSubtotal;
    }});

    const shipping = parseFloat(document.querySelector('input[name="shipping_fee"]').value) || 0;
    const discount = parseFloat(document.querySelector('input[name="discount_amount"]').value) || 0;
    const total = subtotal + shipping - discount;

    document.getElementById('subtotal').value = '$' + subtotal.toFixed(2);
    document.getElementById('totalAmount').value = '$' + total.toFixed(2);
}}

calculateTotal();
</script>"#,
        order.order.id,
        order.order.order_code,
        order.order.id,
        customer_options,
        order.order.customer_id.map(|id| id.to_string()).unwrap_or_default(),
        order.order.customer_name.as_deref().unwrap_or(""),
        order.order.customer_mobile.as_deref().unwrap_or(""),
        order.order.customer_email.as_deref().unwrap_or(""),
        order.address.as_ref().map(|a| a.receiver_name.as_str()).unwrap_or(""),
        order.address.as_ref().map(|a| a.receiver_phone.as_str()).unwrap_or(""),
        order.address.as_ref().map(|a| a.country.as_str()).unwrap_or("US"),
        order.address.as_ref().map(|a| a.address.as_str()).unwrap_or(""),
        items_html,
        order.order.subtotal,
        order.order.shipping_fee,
        order.order.discount_amount,
        order.order.total_amount,
        order.order.payment_terms.as_deref().unwrap_or("100% before shipment"),
        order.order.delivery_terms.as_deref().unwrap_or("EXW"),
        order.order.lead_time.as_deref().unwrap_or("3-7 working days"),
        order.order.customer_note.as_deref().unwrap_or(""),
        order.order.id
    );

    render_layout("编辑订单", "orders", Some(user), &content)
}

/// 订单更新表单
#[derive(Debug, Deserialize)]
pub struct OrderEditForm {
    customer_id: Option<i64>,
    customer_name: Option<String>,
    customer_mobile: Option<String>,
    customer_email: Option<String>,
    receiver_name: String,
    receiver_phone: String,
    country: String,
    address: String,
    item_product: Vec<String>,
    item_quantity: Vec<i64>,
    item_price: Vec<f64>,
    shipping_fee: Option<f64>,
    discount_amount: Option<f64>,
    customer_note: Option<String>,
    payment_terms: Option<String>,
    delivery_terms: Option<String>,
    lead_time: Option<String>,
}

/// 订单更新处理
pub async fn order_update_handler(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Form(form): Form<OrderEditForm>,
) -> Result<impl IntoResponse, Html<String>> {
    let queries = OrderQueries::new(state.db.pool());

    // 检查订单状态
    let order = match queries.get_by_id(id).await {
        Ok(Some(o)) if o.order_status == 1 => o, // 只有未成交状态可编辑
        _ => {
            return Err(Html(r#"<!DOCTYPE html><html><body><script>alert('订单不存在或不可编辑');history.back();</script></body></html>"#.to_string()));
        }
    };

    // 更新订单信息
    match queries.update_order_detail(
        id,
        &form.customer_name,
        &form.customer_mobile,
        &form.customer_email,
        &form.receiver_name,
        &form.receiver_phone,
        &form.country,
        &form.address,
        form.shipping_fee.unwrap_or(0.0),
        form.discount_amount.unwrap_or(0.0),
        form.customer_note.as_deref(),
        form.payment_terms.as_deref(),
        form.delivery_terms.as_deref(),
        form.lead_time.as_deref(),
    ).await {
        Ok(_) => {
            info!("Order updated: id={}", id);
            Ok(Redirect::to(&format!("/orders/{}", id)))
        }
        Err(e) => {
            info!("Failed to update order: {}", e);
            Err(Html(format!(
                r#"<!DOCTYPE html><html><body><script>alert('更新订单失败: {}');history.back();</script></body></html>"#,
                e
            )))
        }
    }
}

// ============================================================================
// 库存新增/详情页面
// ============================================================================

/// 新增库存页面（入库操作）
pub async fn inventory_new_page(
    Extension(auth_user): Extension<AuthUser>,
) -> Html<String> {
    let user = get_user_from_extension(&auth_user);

    let content = r#"<!-- 页面标题 -->
<div class="mb-6">
    <div class="flex items-center gap-2 text-sm text-gray-500 mb-2">
        <a href="/inventory" class="hover:text-blue-600">库存列表</a>
        <span>/</span>
        <span class="text-gray-800">库存入库</span>
    </div>
    <h1 class="text-xl sm:text-2xl font-bold text-gray-800">库存入库</h1>
</div>

<!-- 入库表单 -->
<div class="bg-white rounded-xl shadow-sm border border-gray-100">
    <form action="/inventory/new" method="POST" class="p-4 sm:p-6">
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4 sm:gap-6">
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-2">SKU ID <span class="text-red-500">*</span></label>
                <input type="number" name="sku_id" required min="1" class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500" placeholder="输入SKU ID">
            </div>
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-2">入库数量 <span class="text-red-500">*</span></label>
                <input type="number" name="quantity" required min="1" class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500" placeholder="入库数量">
            </div>
            <div class="md:col-span-2">
                <label class="block text-sm font-medium text-gray-700 mb-2">备注</label>
                <textarea name="note" rows="2" class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500" placeholder="入库备注..."></textarea>
            </div>
        </div>

        <div class="mt-6 flex items-center gap-4">
            <button type="submit" class="px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors">确认入库</button>
            <a href="/inventory" class="px-6 py-2 bg-gray-100 text-gray-700 rounded-lg hover:bg-gray-200 transition-colors">取消</a>
        </div>
    </form>
</div>"#;

    render_layout("库存入库", "inventory", Some(user), content)
}

/// 库存入库表单
#[derive(Debug, Deserialize)]
pub struct InventoryForm {
    sku_id: i64,
    quantity: i64,
    note: Option<String>,
}

/// 库存入库处理
pub async fn inventory_create_handler(
    State(state): State<AppState>,
    Form(form): Form<InventoryForm>,
) -> Result<impl IntoResponse, Html<String>> {
    let queries = InventoryQueries::new(state.db.pool());

    let req = UpdateInventoryRequest {
        quantity: form.quantity,
        note: form.note.clone(),
        damaged_quantity: None,
    };

    match queries.update(form.sku_id, &req, None).await {
        Ok(Some(inventory)) => {
            info!("Inventory updated: sku_id={}, qty={}", form.sku_id, form.quantity);
            Ok(Redirect::to(&format!("/inventory/{}", inventory.sku_id)))
        }
        Ok(None) => {
            Err(Html(r#"<!DOCTYPE html><html><body><script>alert('SKU不存在');history.back();</script></body></html>"#.to_string()))
        }
        Err(e) => {
            Err(Html(format!(
                r#"<!DOCTYPE html><html><body><script>alert('入库失败: {}');history.back();</script></body></html>"#,
                e
            )))
        }
    }
}

/// 库存详情页面
pub async fn inventory_detail_page(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<i64>,
) -> Html<String> {
    let user = get_user_from_extension(&auth_user);

    let queries = InventoryQueries::new(state.db.pool());
    let inventory = match queries.get_by_sku(id).await {
        Ok(Some(i)) => i,
        _ => {
            let content = r#"<div class="text-center py-12">
                <p class="text-4xl mb-4">📊</p>
                <p class="text-gray-600 mb-4">库存记录不存在</p>
                <a href="/inventory" class="text-blue-600 hover:text-blue-800">返回库存列表</a>
            </div>"#;
            return render_layout("库存详情", "inventory", Some(user), content);
        }
    };

    // 获取最近流水记录
    let movements = queries.get_movements(Some(id), 1, 5).await.unwrap_or_else(|_| PagedResponse::new(vec![], 1, 5, 0));

    let status_badge = if inventory.available_quantity <= 0 {
        r#"<span class="px-3 py-1 text-sm font-medium bg-red-100 text-red-700 rounded-full">缺货</span>"#
    } else if inventory.is_low_stock() {
        r#"<span class="px-3 py-1 text-sm font-medium bg-yellow-100 text-yellow-700 rounded-full">低库存</span>"#
    } else {
        r#"<span class="px-3 py-1 text-sm font-medium bg-green-100 text-green-700 rounded-full">正常</span>"#
    };

    // 构建流水记录表格
    let movement_rows: String = movements.items.iter().map(|m| {
        let type_text = match m.movement_type {
            1 => r#"<span class="text-green-600">入库</span>"#,
            2 => r#"<span class="text-red-600">出库</span>"#,
            3 => r#"<span class="text-blue-600">调拨</span>"#,
            4 => r#"<span class="text-purple-600">盘点</span>"#,
            5 => r#"<span class="text-orange-600">损耗</span>"#,
            6 => r#"<span class="text-yellow-600">锁定</span>"#,
            7 => r#"<span class="text-teal-600">解锁</span>"#,
            _ => r#"<span class="text-gray-600">未知</span>"#,
        };
        let qty_class = if m.quantity > 0 { "text-green-600" } else { "text-red-600" };
        let qty_text = if m.quantity >= 0 { format!("+{}", m.quantity) } else { m.quantity.to_string() };
        format!(
            r#"<tr class="hover:bg-gray-50">
                <td class="px-4 py-3 text-sm">{}</td>
                <td class="px-4 py-3 text-sm font-medium {}">{}</td>
                <td class="px-4 py-3 text-sm text-gray-500">{} → {}</td>
                <td class="px-4 py-3 text-sm text-gray-500">{}</td>
                <td class="px-4 py-3 text-sm text-gray-500">{}</td>
            </tr>"#,
            type_text,
            qty_class,
            qty_text,
            m.before_quantity,
            m.after_quantity,
            m.note.as_deref().unwrap_or("-"),
            m.created_at.format("%m-%d %H:%M")
        )
    }).collect();

    let movement_rows = if movement_rows.is_empty() {
        r#"<tr><td colspan="5" class="px-4 py-6 text-center text-gray-500">暂无流水记录</td></tr>"#.to_string()
    } else {
        movement_rows
    };

    let content = format!(
        r#"<!-- 页面标题 -->
<div class="mb-6">
    <div class="flex items-center gap-2 text-sm text-gray-500 mb-2">
        <a href="/inventory" class="hover:text-blue-600">库存列表</a>
        <span>/</span>
        <span class="text-gray-800">库存详情</span>
    </div>
    <div class="flex items-center justify-between">
        <div class="flex items-center gap-3">
            <h1 class="text-xl sm:text-2xl font-bold text-gray-800">库存详情</h1>
            {}
        </div>
        <div class="flex items-center gap-2">
            <a href="/inventory/{}/adjust" class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors text-sm">调整库存</a>
            <a href="/inventory" class="px-4 py-2 bg-gray-100 text-gray-700 rounded-lg hover:bg-gray-200 transition-colors text-sm">返回列表</a>
        </div>
    </div>
</div>

<!-- 库存信息 -->
<div class="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-6">
    <div class="bg-white rounded-xl shadow-sm border border-gray-100 p-4 sm:p-6">
        <h3 class="text-base font-semibold text-gray-800 mb-4">📊 库存数量</h3>
        <div class="grid grid-cols-2 gap-4">
            <div class="text-center p-4 bg-blue-50 rounded-lg">
                <p class="text-3xl font-bold text-blue-600">{}</p>
                <p class="text-sm text-gray-600 mt-1">总库存</p>
            </div>
            <div class="text-center p-4 bg-green-50 rounded-lg">
                <p class="text-3xl font-bold text-green-600">{}</p>
                <p class="text-sm text-gray-600 mt-1">可用库存</p>
            </div>
            <div class="text-center p-4 bg-yellow-50 rounded-lg">
                <p class="text-3xl font-bold text-yellow-600">{}</p>
                <p class="text-sm text-gray-600 mt-1">锁定库存</p>
            </div>
            <div class="text-center p-4 bg-gray-50 rounded-lg">
                <p class="text-3xl font-bold text-gray-600">{}</p>
                <p class="text-sm text-gray-600 mt-1">安全库存</p>
            </div>
        </div>
    </div>

    <div class="bg-white rounded-xl shadow-sm border border-gray-100 p-4 sm:p-6">
        <h3 class="text-base font-semibold text-gray-800 mb-4">📦 SKU 信息</h3>
        <div class="space-y-3">
            <div class="flex justify-between"><span class="text-gray-500">SKU ID</span><span class="font-mono">{}</span></div>
            <div class="flex justify-between"><span class="text-gray-500">损坏数量</span><span class="font-mono">{}</span></div>
            <div class="flex justify-between"><span class="text-gray-500">仓库 ID</span><span class="font-mono">{}</span></div>
            <div class="flex justify-between"><span class="text-gray-500">更新时间</span><span>{}</span></div>
        </div>
    </div>
</div>

<!-- 最近流水记录 -->
<div class="bg-white rounded-xl shadow-sm border border-gray-100 overflow-hidden">
    <div class="px-4 sm:px-6 py-4 border-b border-gray-100 flex items-center justify-between">
        <h3 class="text-base font-semibold text-gray-800">📋 最近流水记录</h3>
        <a href="/inventory/{}/movements" class="text-sm text-blue-600 hover:text-blue-800">查看全部 →</a>
    </div>
    <div class="overflow-x-auto">
        <table class="w-full">
            <thead class="bg-gray-50">
                <tr>
                    <th class="px-4 py-3 text-left text-sm font-medium text-gray-700">类型</th>
                    <th class="px-4 py-3 text-left text-sm font-medium text-gray-700">数量</th>
                    <th class="px-4 py-3 text-left text-sm font-medium text-gray-700">变动</th>
                    <th class="px-4 py-3 text-left text-sm font-medium text-gray-700">备注</th>
                    <th class="px-4 py-3 text-left text-sm font-medium text-gray-700">时间</th>
                </tr>
            </thead>
            <tbody class="divide-y divide-gray-100">{}</tbody>
        </table>
    </div>
</div>"#,
        status_badge,
        inventory.sku_id,
        inventory.total_quantity,
        inventory.available_quantity,
        inventory.locked_quantity,
        inventory.safety_stock,
        inventory.sku_id,
        inventory.damaged_quantity,
        inventory.warehouse_id.map(|id| id.to_string()).unwrap_or("-".to_string()),
        inventory.updated_at.format("%Y-%m-%d %H:%M"),
        inventory.sku_id,
        movement_rows
    );

    render_layout("库存详情", "inventory", Some(user), &content)
}

/// 库存调整页面
pub async fn inventory_adjust_page(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<i64>,
) -> Html<String> {
    let user = get_user_from_extension(&auth_user);

    let queries = InventoryQueries::new(state.db.pool());
    let inventory = match queries.get_by_sku(id).await {
        Ok(Some(i)) => i,
        _ => {
            let content = r#"<div class="text-center py-12">
                <p class="text-4xl mb-4">📊</p>
                <p class="text-gray-600 mb-4">库存记录不存在</p>
                <a href="/inventory" class="text-blue-600 hover:text-blue-800">返回库存列表</a>
            </div>"#;
            return render_layout("库存调整", "inventory", Some(user), content);
        }
    };

    let content = format!(
        r#"<!-- 页面标题 -->
<div class="mb-6">
    <div class="flex items-center gap-2 text-sm text-gray-500 mb-2">
        <a href="/inventory" class="hover:text-blue-600">库存列表</a>
        <span>/</span>
        <a href="/inventory/{}" class="hover:text-blue-600">库存详情</a>
        <span>/</span>
        <span class="text-gray-800">库存调整</span>
    </div>
    <h1 class="text-xl sm:text-2xl font-bold text-gray-800">库存调整</h1>
</div>

<!-- 当前库存信息 -->
<div class="bg-white rounded-xl shadow-sm border border-gray-100 p-4 sm:p-6 mb-6">
    <h3 class="text-base font-semibold text-gray-800 mb-4">📊 当前库存信息</h3>
    <div class="grid grid-cols-2 sm:grid-cols-4 gap-4">
        <div class="text-center p-3 bg-blue-50 rounded-lg">
            <p class="text-2xl font-bold text-blue-600">{}</p>
            <p class="text-sm text-gray-600">总库存</p>
        </div>
        <div class="text-center p-3 bg-green-50 rounded-lg">
            <p class="text-2xl font-bold text-green-600">{}</p>
            <p class="text-sm text-gray-600">可用库存</p>
        </div>
        <div class="text-center p-3 bg-yellow-50 rounded-lg">
            <p class="text-2xl font-bold text-yellow-600">{}</p>
            <p class="text-sm text-gray-600">锁定库存</p>
        </div>
        <div class="text-center p-3 bg-gray-50 rounded-lg">
            <p class="text-2xl font-bold text-gray-600">{}</p>
            <p class="text-sm text-gray-600">损坏库存</p>
        </div>
    </div>
</div>

<!-- 调整表单 -->
<div class="bg-white rounded-xl shadow-sm border border-gray-100">
    <form action="/inventory/{}/adjust" method="POST" class="p-4 sm:p-6">
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4 sm:gap-6">
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-2">调整类型 <span class="text-red-500">*</span></label>
                <select name="adjust_type" required class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500">
                    <option value="inbound">入库（采购入库）</option>
                    <option value="outbound">出库（销售出库）</option>
                    <option value="adjustment">盘点调整</option>
                    <option value="damage">损耗（损坏/过期）</option>
                    <option value="lock">锁定</option>
                    <option value="unlock">解锁</option>
                </select>
            </div>
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-2">变动数量 <span class="text-red-500">*</span></label>
                <input type="number" name="quantity" required class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500" placeholder="输入数量（正数）" min="1">
                <p class="mt-1 text-xs text-gray-500">入库/解锁填正数，出库/损耗/锁定填正数（系统自动处理方向）</p>
            </div>
            <div class="md:col-span-2">
                <label class="block text-sm font-medium text-gray-700 mb-2">备注 <span class="text-red-500">*</span></label>
                <textarea name="note" required rows="2" class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500" placeholder="请填写调整原因（必填）"></textarea>
            </div>
        </div>

        <div class="mt-6 flex items-center gap-4">
            <button type="submit" class="px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors">确认调整</button>
            <a href="/inventory/{}" class="px-6 py-2 bg-gray-100 text-gray-700 rounded-lg hover:bg-gray-200 transition-colors">取消</a>
        </div>
    </form>
</div>

<!-- 调整类型说明 -->
<div class="mt-6 bg-gray-50 rounded-xl p-4 sm:p-6">
    <h3 class="text-base font-semibold text-gray-800 mb-3">📝 调整类型说明</h3>
    <div class="grid grid-cols-1 sm:grid-cols-2 gap-3 text-sm text-gray-600">
        <div><span class="font-medium text-green-600">入库</span>：采购入库，增加总库存和可用库存</div>
        <div><span class="font-medium text-red-600">出库</span>：销售出库，减少总库存和可用库存</div>
        <div><span class="font-medium text-purple-600">盘点调整</span>：盘点后调整库存到实际数量</div>
        <div><span class="font-medium text-orange-600">损耗</span>：损坏、过期等，减少可用库存</div>
        <div><span class="font-medium text-yellow-600">锁定</span>：订单锁定，可用→锁定</div>
        <div><span class="font-medium text-teal-600">解锁</span>：订单取消，锁定→可用</div>
    </div>
</div>"#,
        inventory.sku_id,
        inventory.total_quantity,
        inventory.available_quantity,
        inventory.locked_quantity,
        inventory.damaged_quantity,
        inventory.sku_id,
        inventory.sku_id
    );

    render_layout("库存调整", "inventory", Some(user), &content)
}

/// 库存调整表单
#[derive(Debug, Deserialize)]
pub struct InventoryAdjustForm {
    adjust_type: String,
    quantity: i64,
    note: String,
}

/// 库存调整处理
pub async fn inventory_adjust_handler(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<i64>,
    Form(form): Form<InventoryAdjustForm>,
) -> Result<impl IntoResponse, Html<String>> {
    let queries = InventoryQueries::new(state.db.pool());
    let operator_id = Some(auth_user.user_id);

    // 获取当前库存
    let current = match queries.get_by_sku(id).await {
        Ok(Some(i)) => i,
        _ => {
            return Err(Html(r#"<!DOCTYPE html><html><body><script>alert('库存记录不存在');history.back();</script></body></html>"#.to_string()))
        }
    };

    let result = match form.adjust_type.as_str() {
        "inbound" => {
            // 入库：增加总库存和可用库存
            queries.adjust_inventory(id, form.quantity, form.quantity, 0, 0, &form.note, operator_id).await
        }
        "outbound" => {
            // 出库：减少总库存和可用库存
            if current.available_quantity < form.quantity {
                return Err(Html(format!(
                    r#"<!DOCTYPE html><html><body><script>alert('可用库存不足，当前可用: {}');history.back();</script></body></html>"#,
                    current.available_quantity
                )));
            }
            queries.adjust_inventory(id, -form.quantity, -form.quantity, 0, 0, &form.note, operator_id).await
        }
        "adjustment" => {
            // 盘点调整：直接设置可用库存为目标值
            let diff = form.quantity - current.available_quantity;
            queries.adjust_inventory(id, diff, diff, 0, 0, &form.note, operator_id).await
        }
        "damage" => {
            // 损耗：减少可用库存，增加损坏库存
            if current.available_quantity < form.quantity {
                return Err(Html(format!(
                    r#"<!DOCTYPE html><html><body><script>alert('可用库存不足，当前可用: {}');history.back();</script></body></html>"#,
                    current.available_quantity
                )));
            }
            queries.adjust_inventory(id, -form.quantity, -form.quantity, 0, form.quantity, &form.note, operator_id).await
        }
        "lock" => {
            // 锁定：可用→锁定
            if current.available_quantity < form.quantity {
                return Err(Html(format!(
                    r#"<!DOCTYPE html><html><body><script>alert('可用库存不足，当前可用: {}');history.back();</script></body></html>"#,
                    current.available_quantity
                )));
            }
            queries.adjust_inventory(id, 0, -form.quantity, form.quantity, 0, &form.note, operator_id).await
        }
        "unlock" => {
            // 解锁：锁定→可用
            if current.locked_quantity < form.quantity {
                return Err(Html(format!(
                    r#"<!DOCTYPE html><html><body><script>alert('锁定库存不足，当前锁定: {}');history.back();</script></body></html>"#,
                    current.locked_quantity
                )));
            }
            queries.adjust_inventory(id, 0, form.quantity, -form.quantity, 0, &form.note, operator_id).await
        }
        _ => {
            return Err(Html(r#"<!DOCTYPE html><html><body><script>alert('无效的调整类型');history.back();</script></body></html>"#.to_string()))
        }
    };

    match result {
        Ok(_) => {
            info!("Inventory adjusted: sku_id={}, type={}, qty={}", id, form.adjust_type, form.quantity);
            Ok(Redirect::to(&format!("/inventory/{}", id)))
        }
        Err(e) => {
            Err(Html(format!(
                r#"<!DOCTYPE html><html><body><script>alert('调整失败: {}');history.back();</script></body></html>"#,
                e
            )))
        }
    }
}

/// 库存流水页面
pub async fn inventory_movements_page(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<i64>,
    Query(query): Query<PageQuery>,
) -> Html<String> {
    let user = get_user_from_extension(&auth_user);
    let page = query.page.unwrap_or(1).max(1);
    let page_size = 20;

    let queries = InventoryQueries::new(state.db.pool());

    // 获取库存信息
    let inventory = queries.get_by_sku(id).await.ok().flatten();

    // 获取流水记录
    let result = queries.get_movements(Some(id), page, page_size).await.unwrap_or_else(|_| PagedResponse::new(vec![], page, page_size, 0));

    let rows: String = result.items.iter().map(|m| {
        let type_text = match m.movement_type {
            1 => r#"<span class="px-2 py-1 text-xs bg-green-100 text-green-700 rounded-full">入库</span>"#,
            2 => r#"<span class="px-2 py-1 text-xs bg-red-100 text-red-700 rounded-full">出库</span>"#,
            3 => r#"<span class="px-2 py-1 text-xs bg-blue-100 text-blue-700 rounded-full">调拨</span>"#,
            4 => r#"<span class="px-2 py-1 text-xs bg-purple-100 text-purple-700 rounded-full">盘点</span>"#,
            5 => r#"<span class="px-2 py-1 text-xs bg-orange-100 text-orange-700 rounded-full">损耗</span>"#,
            6 => r#"<span class="px-2 py-1 text-xs bg-yellow-100 text-yellow-700 rounded-full">锁定</span>"#,
            7 => r#"<span class="px-2 py-1 text-xs bg-teal-100 text-teal-700 rounded-full">解锁</span>"#,
            _ => r#"<span class="px-2 py-1 text-xs bg-gray-100 text-gray-700 rounded-full">未知</span>"#,
        };
        let qty_class = if m.quantity > 0 { "text-green-600 font-medium" } else { "text-red-600 font-medium" };
        let qty_text = if m.quantity >= 0 { format!("+{}", m.quantity) } else { m.quantity.to_string() };
        let reference = match (&m.reference_type, &m.reference_code) {
            (Some(ref_type), Some(code)) => format!("{}: {}", ref_type, code),
            (Some(ref_type), None) => ref_type.clone(),
            _ => "-".to_string(),
        };
        format!(
            r#"<tr class="hover:bg-gray-50">
                <td class="px-4 sm:px-6 py-4"><span class="font-mono text-sm">{}</span></td>
                <td class="px-4 sm:px-6 py-4">{}</td>
                <td class="px-4 sm:px-6 py-4 {}">{}</td>
                <td class="px-4 sm:px-6 py-4 text-gray-500">{} → {}</td>
                <td class="px-4 sm:px-6 py-4 text-gray-500">{}</td>
                <td class="px-4 sm:px-6 py-4 text-gray-500">{}</td>
                <td class="px-4 sm:px-6 py-4 text-gray-500">{}</td>
            </tr>"#,
            m.movement_code,
            type_text,
            qty_class,
            qty_text,
            m.before_quantity,
            m.after_quantity,
            reference,
            m.note.as_deref().unwrap_or("-"),
            m.created_at.format("%Y-%m-%d %H:%M")
        )
    }).collect();

    let rows = if rows.is_empty() {
        r#"<tr><td colspan="7" class="px-6 py-12 text-center"><div class="text-gray-500"><p class="text-4xl mb-2">📋</p><p>暂无流水记录</p></div></td></tr>"#.to_string()
    } else {
        rows
    };

    let total_pages = ((result.pagination.total as f64) / (page_size as f64)).ceil() as u32;
    let pagination = if total_pages > 1 {
        let prev_btn = if page > 1 {
            format!(r#"<a href="/inventory/{}/movements?page={}" class="px-4 py-2 text-sm text-gray-600 hover:bg-gray-100 rounded-lg">上一页</a>"#, id, page - 1)
        } else {
            String::new()
        };
        let next_btn = if page < total_pages {
            format!(r#"<a href="/inventory/{}/movements?page={}" class="px-4 py-2 text-sm text-gray-600 hover:bg-gray-100 rounded-lg">下一页</a>"#, id, page + 1)
        } else {
            String::new()
        };
        format!(
            r#"<div class="flex items-center justify-between mt-4">
                <p class="text-sm text-gray-600">共 {} 条记录，第 {}/{} 页</p>
                <div class="flex items-center gap-2">{}{}</div>
            </div>"#,
            result.pagination.total, page, total_pages, prev_btn, next_btn
        )
    } else {
        String::new()
    };

    let inventory_info = if let Some(ref inv) = inventory {
        format!(
            r#"<div class="bg-white rounded-xl shadow-sm border border-gray-100 p-4 mb-6">
                <div class="flex items-center justify-between">
                    <div>
                        <span class="text-gray-500">SKU ID: </span>
                        <span class="font-mono">{}</span>
                    </div>
                    <div class="flex items-center gap-4 text-sm">
                        <span><span class="text-gray-500">总库存:</span> <span class="font-medium">{}</span></span>
                        <span><span class="text-gray-500">可用:</span> <span class="font-medium text-green-600">{}</span></span>
                        <span><span class="text-gray-500">锁定:</span> <span class="font-medium text-yellow-600">{}</span></span>
                    </div>
                </div>
            </div>"#,
            inv.sku_id,
            inv.total_quantity,
            inv.available_quantity,
            inv.locked_quantity
        )
    } else {
        String::new()
    };

    let content = format!(
        r#"<!-- 页面标题 -->
<div class="mb-6">
    <div class="flex items-center gap-2 text-sm text-gray-500 mb-2">
        <a href="/inventory" class="hover:text-blue-600">库存列表</a>
        <span>/</span>
        <a href="/inventory/{}" class="hover:text-blue-600">库存详情</a>
        <span>/</span>
        <span class="text-gray-800">库存流水</span>
    </div>
    <div class="flex items-center justify-between">
        <h1 class="text-xl sm:text-2xl font-bold text-gray-800">库存流水</h1>
        <div class="flex items-center gap-2">
            <a href="/inventory/{}/adjust" class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors text-sm">调整库存</a>
            <a href="/inventory/{}" class="px-4 py-2 bg-gray-100 text-gray-700 rounded-lg hover:bg-gray-200 transition-colors text-sm">返回详情</a>
        </div>
    </div>
</div>

{}

<!-- 流水表格 -->
<div class="bg-white rounded-xl shadow-sm border border-gray-100 overflow-hidden">
    <div class="overflow-x-auto">
        <table class="w-full min-w-[800px]">
            <thead class="bg-gray-50 border-b border-gray-200">
                <tr>
                    <th class="px-4 sm:px-6 py-4 text-left text-sm font-semibold text-gray-700">流水号</th>
                    <th class="px-4 sm:px-6 py-4 text-left text-sm font-semibold text-gray-700">类型</th>
                    <th class="px-4 sm:px-6 py-4 text-left text-sm font-semibold text-gray-700">数量</th>
                    <th class="px-4 sm:px-6 py-4 text-left text-sm font-semibold text-gray-700">变动</th>
                    <th class="px-4 sm:px-6 py-4 text-left text-sm font-semibold text-gray-700">关联单据</th>
                    <th class="px-4 sm:px-6 py-4 text-left text-sm font-semibold text-gray-700">备注</th>
                    <th class="px-4 sm:px-6 py-4 text-left text-sm font-semibold text-gray-700">时间</th>
                </tr>
            </thead>
            <tbody class="divide-y divide-gray-100">{}</tbody>
        </table>
    </div>
    {}
</div>"#,
        id,
        id,
        id,
        inventory_info,
        rows,
        pagination
    );

    render_layout("库存流水", "inventory", Some(user), &content)
}

// ============================================================================
// 客户新增/详情页面
// ============================================================================

/// 新增客户页面（简化版）
pub async fn customer_new_page(
    Extension(auth_user): Extension<AuthUser>,
) -> Html<String> {
    let user = get_user_from_extension(&auth_user);

    let content = r#"<!-- 页面标题 -->
<div class="mb-6">
    <div class="flex items-center gap-2 text-sm text-gray-500 mb-2">
        <a href="/customers" class="hover:text-blue-600">客户列表</a>
        <span>/</span>
        <span class="text-gray-800">新增客户</span>
    </div>
    <h1 class="text-xl sm:text-2xl font-bold text-gray-800">新增客户</h1>
</div>

<!-- 客户表单 -->
<div class="bg-white rounded-xl shadow-sm border border-gray-100">
    <form action="/customers/new" method="POST" class="p-4 sm:p-6">
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4 sm:gap-6">
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-2">姓名 <span class="text-red-500">*</span></label>
                <input type="text" name="name" required class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500" placeholder="客户姓名">
            </div>
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-2">手机号 <span class="text-red-500">*</span></label>
                <input type="text" name="mobile" required class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500" placeholder="手机号（唯一）">
            </div>
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-2">邮箱</label>
                <input type="email" name="email" class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500" placeholder="邮箱地址">
            </div>
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-2">状态</label>
                <select name="status" class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500">
                    <option value="1">正常</option>
                    <option value="2">冻结</option>
                    <option value="3">黑名单</option>
                </select>
            </div>
            <div class="md:col-span-2">
                <label class="block text-sm font-medium text-gray-700 mb-2">备注</label>
                <textarea name="notes" rows="2" class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500" placeholder="备注信息..."></textarea>
            </div>
        </div>

        <div class="mt-6 flex items-center gap-4">
            <button type="submit" class="px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors">保存客户</button>
            <a href="/customers" class="px-6 py-2 bg-gray-100 text-gray-700 rounded-lg hover:bg-gray-200 transition-colors">取消</a>
        </div>
    </form>
</div>"#;

    render_layout("新增客户", "customers", Some(user), content)
}

/// 创建客户表单（简化版）
#[derive(Debug, Deserialize)]
pub struct CustomerForm {
    name: String,
    mobile: String,  // 改为必填
    email: Option<String>,
    status: Option<i64>,
    notes: Option<String>,
}

/// 表单方法覆盖（用于 DELETE 等）
#[derive(Debug, Deserialize)]
pub struct MethodForm {
    pub _method: Option<String>,
}

/// 创建客户处理
pub async fn customer_create_handler(
    State(state): State<AppState>,
    Form(form): Form<CustomerForm>,
) -> Result<impl IntoResponse, Html<String>> {
    let queries = CustomerQueries::new(state.db.pool());

    let req = CreateCustomerRequest {
        name: form.name.clone(),
        mobile: form.mobile.clone(),
        email: form.email.clone(),
        status: form.status,
        notes: form.notes.clone(),
        source: None,
    };

    match queries.create(&req).await {
        Ok(customer) => {
            info!("Customer created: id={}", customer.id);
            Ok(Redirect::to(&format!("/customers/{}", customer.id)))
        }
        Err(e) => {
            Err(Html(format!(
                r#"<!DOCTYPE html><html><body><script>alert('创建客户失败: {}');history.back();</script></body></html>"#,
                e
            )))
        }
    }
}

/// 客户详情页面（含地址管理）
pub async fn customer_detail_page(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<i64>,
) -> Html<String> {
    let user = get_user_from_extension(&auth_user);

    let queries = CustomerQueries::new(state.db.pool());
    let customer = match queries.get_by_id(id).await {
        Ok(Some(c)) => c,
        _ => {
            let content = r#"<div class="text-center py-12">
                <p class="text-4xl mb-4">👥</p>
                <p class="text-gray-600 mb-4">客户不存在</p>
                <a href="/customers" class="text-blue-600 hover:text-blue-800">返回客户列表</a>
            </div>"#;
            return render_layout("客户详情", "customers", Some(user), content);
        }
    };

    // 获取客户地址列表
    let addresses = queries.get_addresses(id).await.unwrap_or_default();

    let status_badge = match customer.status {
        1 => r#"<span class="px-3 py-1 text-sm font-medium bg-green-100 text-green-700 rounded-full">正常</span>"#,
        2 => r#"<span class="px-3 py-1 text-sm font-medium bg-yellow-100 text-yellow-700 rounded-full">冻结</span>"#,
        _ => r#"<span class="px-3 py-1 text-sm font-medium bg-red-100 text-red-700 rounded-full">黑名单</span>"#,
    };

    // 地址列表
    let address_rows: String = if addresses.is_empty() {
        r#"<tr><td colspan="5" class="px-4 py-8 text-center text-gray-500">暂无收货地址</td></tr>"#.to_string()
    } else {
        addresses.iter().map(|a| {
            let default_badge = if a.is_default {
                r#"<span class="ml-2 px-2 py-0.5 text-xs bg-blue-100 text-blue-700 rounded">默认</span>"#
            } else {
                ""
            };
            let full_address = format!("{}{}{}{}{}",
                a.country,
                a.province.as_ref().map(|p| format!(" {}", p)).unwrap_or_default(),
                a.city.as_ref().map(|c| format!(" {}", c)).unwrap_or_default(),
                a.district.as_ref().map(|d| format!(" {}", d)).unwrap_or_default(),
                a.address
            );
            let set_default_btn = if a.is_default {
                String::new()
            } else {
                format!(r#"<form action="/customers/{}/addresses/{}/set-default" method="POST" class="inline"><button type="submit" class="text-green-600 hover:text-green-800 text-sm">设为默认</button></form><span class="text-gray-300">|</span>"#, id, a.id)
            };
            format!(
                r#"<tr class="hover:bg-gray-50">
                    <td class="px-4 py-3">{}</td>
                    <td class="px-4 py-3">{}</td>
                    <td class="px-4 py-3 text-sm">{}</td>
                    <td class="px-4 py-3 text-center">{}</td>
                    <td class="px-4 py-3 text-center space-x-2">
                        <a href="/customers/{}/addresses/{}/edit" class="text-blue-600 hover:text-blue-800 text-sm">编辑</a>
                        {}
                        <form action="/customers/{}/addresses/{}" method="POST" class="inline" onsubmit="return confirm('确定删除此地址？')">
                            <input type="hidden" name="_method" value="DELETE">
                            <button type="submit" class="text-red-600 hover:text-red-800 text-sm">删除</button>
                        </form>
                    </td>
                </tr>"#,
                a.receiver_name,
                a.receiver_phone,
                full_address,
                default_badge,
                id,
                a.id,
                set_default_btn,
                id,
                a.id
            )
        }).collect()
    };

    let content = format!(
        r#"<!-- 页面标题 -->
<div class="mb-6">
    <div class="flex items-center gap-2 text-sm text-gray-500 mb-2">
        <a href="/customers" class="hover:text-blue-600">客户列表</a>
        <span>/</span>
        <span class="text-gray-800">{}</span>
    </div>
    <div class="flex items-center justify-between">
        <div class="flex items-center gap-3">
            <h1 class="text-xl sm:text-2xl font-bold text-gray-800">{}</h1>
            {}
        </div>
        <div class="flex items-center gap-2">
            <a href="/customers/{}/edit" class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors text-sm">编辑</a>
            <a href="/customers" class="px-4 py-2 bg-gray-100 text-gray-700 rounded-lg hover:bg-gray-200 transition-colors text-sm">返回列表</a>
        </div>
    </div>
</div>

<!-- 客户信息 -->
<div class="grid grid-cols-1 lg:grid-cols-3 gap-6 mb-6">
    <div class="lg:col-span-2 bg-white rounded-xl shadow-sm border border-gray-100 p-4 sm:p-6">
        <h3 class="text-base font-semibold text-gray-800 mb-4">👤 基本信息</h3>
        <div class="grid grid-cols-2 md:grid-cols-3 gap-4">
            <div>
                <p class="text-xs text-gray-500">客户编码</p>
                <p class="font-mono text-sm">{}</p>
            </div>
            <div>
                <p class="text-xs text-gray-500">姓名</p>
                <p class="font-medium">{}</p>
            </div>
            <div>
                <p class="text-xs text-gray-500">手机号</p>
                <p>{}</p>
            </div>
            <div>
                <p class="text-xs text-gray-500">邮箱</p>
                <p>{}</p>
            </div>
            <div>
                <p class="text-xs text-gray-500">备注</p>
                <p class="text-sm">{}</p>
            </div>
            <div>
                <p class="text-xs text-gray-500">注册时间</p>
                <p>{}</p>
            </div>
        </div>
    </div>

    <div class="space-y-6">
        <div class="bg-white rounded-xl shadow-sm border border-gray-100 p-4 sm:p-6">
            <h3 class="text-base font-semibold text-gray-800 mb-4">📊 消费统计</h3>
            <div class="space-y-3">
                <div class="flex justify-between"><span class="text-gray-500">订单数</span><span class="font-semibold">{}</span></div>
                <div class="flex justify-between"><span class="text-gray-500">消费总额</span><span class="font-semibold text-green-600">¥{:.2}</span></div>
                <div class="flex justify-between"><span class="text-gray-500">积分</span><span class="font-semibold">{}</span></div>
            </div>
        </div>
    </div>
</div>

<!-- 收货地址管理 -->
<div class="bg-white rounded-xl shadow-sm border border-gray-100 overflow-hidden">
    <div class="flex items-center justify-between px-4 sm:px-6 py-4 border-b border-gray-100">
        <h3 class="text-base font-semibold text-gray-800">📍 收货地址</h3>
        <button type="button" onclick="document.getElementById('addressForm').classList.toggle('hidden')" class="px-3 py-1 text-sm bg-blue-600 text-white rounded hover:bg-blue-700">添加地址</button>
    </div>

    <!-- 添加地址表单 -->
    <div id="addressForm" class="hidden border-b border-gray-100 p-4 sm:p-6 bg-gray-50">
        <form action="/customers/{}/addresses" method="POST" class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">收件人 <span class="text-red-500">*</span></label>
                <input type="text" name="receiver_name" required class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm">
            </div>
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">手机号 <span class="text-red-500">*</span></label>
                <input type="text" name="receiver_phone" required class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm">
            </div>
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">国家</label>
                <input type="text" name="country" value="中国" class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm">
            </div>
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">省份</label>
                <input type="text" name="province" class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm">
            </div>
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">城市</label>
                <input type="text" name="city" class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm">
            </div>
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">区县</label>
                <input type="text" name="district" class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm">
            </div>
            <div class="md:col-span-2">
                <label class="block text-sm font-medium text-gray-700 mb-1">详细地址 <span class="text-red-500">*</span></label>
                <input type="text" name="address" required class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm">
            </div>
            <div class="md:col-span-2 flex items-center gap-4">
                <label class="flex items-center gap-2">
                    <input type="checkbox" name="is_default" class="rounded">
                    <span class="text-sm text-gray-700">设为默认地址</span>
                </label>
                <button type="submit" class="px-4 py-2 bg-blue-600 text-white rounded-lg text-sm hover:bg-blue-700">保存地址</button>
                <button type="button" onclick="document.getElementById('addressForm').classList.add('hidden')" class="px-4 py-2 bg-gray-200 text-gray-700 rounded-lg text-sm hover:bg-gray-300">取消</button>
            </div>
        </form>
    </div>

    <!-- 地址列表 -->
    <div class="overflow-x-auto">
        <table class="w-full">
            <thead class="bg-gray-50">
                <tr>
                    <th class="px-4 py-3 text-left text-sm font-semibold text-gray-700">收件人</th>
                    <th class="px-4 py-3 text-left text-sm font-semibold text-gray-700">手机号</th>
                    <th class="px-4 py-3 text-left text-sm font-semibold text-gray-700">地址</th>
                    <th class="px-4 py-3 text-center text-sm font-semibold text-gray-700">状态</th>
                    <th class="px-4 py-3 text-center text-sm font-semibold text-gray-700">操作</th>
                </tr>
            </thead>
            <tbody class="divide-y divide-gray-100">{}</tbody>
        </table>
    </div>
</div>"#,
        customer.name,
        customer.name,
        status_badge,
        id,
        customer.customer_code,
        customer.name,
        customer.mobile.unwrap_or_default(),
        customer.email.unwrap_or_default(),
        customer.notes.unwrap_or_default(),
        customer.created_at.format("%Y-%m-%d %H:%M"),
        customer.total_orders,
        customer.total_amount,
        customer.points,
        id,
        address_rows
    );

    render_layout("客户详情", "customers", Some(user), &content)
}

/// 客户编辑页面
pub async fn customer_edit_page(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<i64>,
) -> Html<String> {
    let user = get_user_from_extension(&auth_user);

    let queries = CustomerQueries::new(state.db.pool());
    let customer = match queries.get_by_id(id).await {
        Ok(Some(c)) => c,
        _ => {
            let content = r#"<div class="text-center py-12">
                <p class="text-4xl mb-4">👥</p>
                <p class="text-gray-600 mb-4">客户不存在</p>
                <a href="/customers" class="text-blue-600 hover:text-blue-800">返回客户列表</a>
            </div>"#;
            return render_layout("编辑客户", "customers", Some(user), content);
        }
    };

    let status_selected_1 = if customer.status == 1 { "selected" } else { "" };
    let status_selected_2 = if customer.status == 2 { "selected" } else { "" };
    let status_selected_3 = if customer.status == 3 { "selected" } else { "" };

    let content = format!(
        r#"<!-- 页面标题 -->
<div class="mb-6">
    <div class="flex items-center gap-2 text-sm text-gray-500 mb-2">
        <a href="/customers" class="hover:text-blue-600">客户列表</a>
        <span>/</span>
        <a href="/customers/{}" class="hover:text-blue-600">{}</a>
        <span>/</span>
        <span class="text-gray-800">编辑</span>
    </div>
    <h1 class="text-xl sm:text-2xl font-bold text-gray-800">编辑客户</h1>
</div>

<!-- 客户表单 -->
<div class="bg-white rounded-xl shadow-sm border border-gray-100">
    <form action="/customers/{}/edit" method="POST" class="p-4 sm:p-6">
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4 sm:gap-6">
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-2">姓名 <span class="text-red-500">*</span></label>
                <input type="text" name="name" value="{}" required class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500">
            </div>
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-2">手机号 <span class="text-red-500">*</span></label>
                <input type="text" name="mobile" value="{}" required class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500">
            </div>
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-2">邮箱</label>
                <input type="email" name="email" value="{}" class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500">
            </div>
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-2">状态</label>
                <select name="status" class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500">
                    <option value="1" {}>正常</option>
                    <option value="2" {}>冻结</option>
                    <option value="3" {}>黑名单</option>
                </select>
            </div>
            <div class="md:col-span-2">
                <label class="block text-sm font-medium text-gray-700 mb-2">备注</label>
                <textarea name="notes" rows="2" class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500" placeholder="备注信息...">{}</textarea>
            </div>
        </div>

        <div class="mt-6 flex items-center gap-4">
            <button type="submit" class="px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors">保存</button>
            <a href="/customers/{}" class="px-6 py-2 bg-gray-100 text-gray-700 rounded-lg hover:bg-gray-200 transition-colors">取消</a>
        </div>
    </form>
</div>"#,
        id,
        customer.name,
        id,
        customer.name,
        customer.mobile.unwrap_or_default(),
        customer.email.unwrap_or_default(),
        status_selected_1,
        status_selected_2,
        status_selected_3,
        customer.notes.unwrap_or_default(),
        id
    );

    render_layout("编辑客户", "customers", Some(user), &content)
}

/// 更新客户处理
pub async fn customer_update_handler(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Form(form): Form<CustomerForm>,
) -> Result<impl IntoResponse, Html<String>> {
    let queries = CustomerQueries::new(state.db.pool());

    let req = UpdateCustomerRequest {
        name: Some(form.name.clone()),
        mobile: Some(form.mobile.clone()),
        email: form.email.clone(),
        status: form.status,
        notes: form.notes.clone(),
    };

    match queries.update(id, &req).await {
        Ok(Some(_)) => {
            info!("Customer updated: id={}", id);
            Ok(Redirect::to(&format!("/customers/{}", id)))
        }
        Ok(None) => {
            Err(Html(r#"<div class="text-center py-12"><p>客户不存在</p><a href="/customers">返回列表</a></div>"#.to_string()))
        }
        Err(e) => {
            Err(Html(format!(
                r#"<!DOCTYPE html><html><body><script>alert('更新客户失败: {}');history.back();</script></body></html>"#,
                e
            )))
        }
    }
}

/// 客户地址表单
#[derive(Debug, Deserialize)]
pub struct AddressForm {
    pub receiver_name: String,
    pub receiver_phone: String,
    pub country: String,
    pub province: Option<String>,
    pub city: Option<String>,
    pub district: Option<String>,
    pub address: String,
    pub is_default: Option<String>,
}

/// 添加客户地址处理
pub async fn customer_address_add_handler(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Form(form): Form<AddressForm>,
) -> Result<impl IntoResponse, Html<String>> {
    let queries = CustomerQueries::new(state.db.pool());

    let req = CreateAddressRequest {
        receiver_name: form.receiver_name,
        receiver_phone: form.receiver_phone,
        country: form.country,
        country_code: None,
        province: form.province,
        city: form.city,
        district: form.district,
        address: form.address,
        postal_code: None,
        address_type: Some(1),
        is_default: form.is_default.is_some(),
    };

    match queries.create_address(id, &req).await {
        Ok(_) => {
            info!("Customer address added: customer_id={}", id);
            Ok(Redirect::to(&format!("/customers/{}", id)))
        }
        Err(e) => {
            Err(Html(format!(
                r#"<!DOCTYPE html><html><body><script>alert('添加地址失败: {}');history.back();</script></body></html>"#,
                e
            )))
        }
    }
}

/// 删除客户地址处理（通过 _method=DELETE）
pub async fn customer_address_delete_handler(
    State(state): State<AppState>,
    Path((id, address_id)): Path<(i64, i64)>,
    Form(form): Form<MethodForm>,
) -> Result<impl IntoResponse, Html<String>> {
    // 检查是否是 DELETE 方法
    if form._method.as_deref() != Some("DELETE") {
        return Ok(Redirect::to(&format!("/customers/{}", id)));
    }

    let queries = CustomerQueries::new(state.db.pool());

    match queries.delete_address(id, address_id).await {
        Ok(true) => {
            info!("Customer address deleted: customer_id={}, address_id={}", id, address_id);
            Ok(Redirect::to(&format!("/customers/{}", id)))
        }
        Ok(false) => {
            Err(Html(r#"<div class="text-center py-12"><p>地址不存在</p><a href="/customers">返回列表</a></div>"#.to_string()))
        }
        Err(e) => {
            Err(Html(format!(
                r#"<!DOCTYPE html><html><body><script>alert('删除地址失败: {}');history.back();</script></body></html>"#,
                e
            )))
        }
    }
}

/// 地址编辑页面
pub async fn customer_address_edit_page(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Path((customer_id, address_id)): Path<(i64, i64)>,
) -> Html<String> {
    let user = get_user_from_extension(&auth_user);

    let queries = CustomerQueries::new(state.db.pool());

    // 获取客户信息
    let customer = match queries.get_by_id(customer_id).await {
        Ok(Some(c)) => c,
        _ => {
            let content = r#"<div class="text-center py-12">
                <p class="text-4xl mb-4">👥</p>
                <p class="text-gray-600 mb-4">客户不存在</p>
                <a href="/customers" class="text-blue-600 hover:text-blue-800">返回客户列表</a>
            </div>"#;
            return render_layout("编辑地址", "customers", Some(user), content);
        }
    };

    // 获取地址信息
    let address = match queries.get_address_by_id(address_id).await {
        Ok(Some(a)) if a.customer_id == customer_id => a,
        _ => {
            let content = r#"<div class="text-center py-12">
                <p class="text-gray-600 mb-4">地址不存在</p>
                <a href="javascript:history.back()" class="text-blue-600 hover:text-blue-800">返回</a>
            </div>"#;
            return render_layout("编辑地址", "customers", Some(user), content);
        }
    };

    let content = format!(
        r#"<!-- 页面标题 -->
<div class="mb-6">
    <div class="flex items-center gap-2 text-sm text-gray-500 mb-2">
        <a href="/customers" class="hover:text-blue-600">客户列表</a>
        <span>/</span>
        <a href="/customers/{}" class="hover:text-blue-600">{}</a>
        <span>/</span>
        <span class="text-gray-800">编辑地址</span>
    </div>
    <h1 class="text-xl sm:text-2xl font-bold text-gray-800">📍 编辑收货地址</h1>
</div>

<!-- 编辑地址表单 -->
<div class="bg-white rounded-xl shadow-sm border border-gray-100 p-6">
    <form action="/customers/{}/addresses/{}/update" method="POST" class="grid grid-cols-1 md:grid-cols-2 gap-4">
        <div>
            <label class="block text-sm font-medium text-gray-700 mb-1">收件人 <span class="text-red-500">*</span></label>
            <input type="text" name="receiver_name" required value="{}" class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm">
        </div>
        <div>
            <label class="block text-sm font-medium text-gray-700 mb-1">手机号 <span class="text-red-500">*</span></label>
            <input type="text" name="receiver_phone" required value="{}" class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm">
        </div>
        <div>
            <label class="block text-sm font-medium text-gray-700 mb-1">国家</label>
            <input type="text" name="country" value="{}" class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm">
        </div>
        <div>
            <label class="block text-sm font-medium text-gray-700 mb-1">省份</label>
            <input type="text" name="province" value="{}" class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm">
        </div>
        <div>
            <label class="block text-sm font-medium text-gray-700 mb-1">城市</label>
            <input type="text" name="city" value="{}" class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm">
        </div>
        <div>
            <label class="block text-sm font-medium text-gray-700 mb-1">区县</label>
            <input type="text" name="district" value="{}" class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm">
        </div>
        <div class="md:col-span-2">
            <label class="block text-sm font-medium text-gray-700 mb-1">详细地址 <span class="text-red-500">*</span></label>
            <input type="text" name="address" required value="{}" class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm">
        </div>
        <div class="md:col-span-2 flex items-center gap-4">
            <label class="flex items-center gap-2">
                <input type="checkbox" name="is_default" class="rounded" {}>
                <span class="text-sm text-gray-700">设为默认地址</span>
            </label>
            <button type="submit" class="px-4 py-2 bg-blue-600 text-white rounded-lg text-sm hover:bg-blue-700">保存修改</button>
            <a href="/customers/{}" class="px-4 py-2 bg-gray-200 text-gray-700 rounded-lg text-sm hover:bg-gray-300">取消</a>
        </div>
    </form>
</div>"#,
        customer_id,
        customer.name,
        customer_id,
        address_id,
        address.receiver_name,
        address.receiver_phone,
        address.country,
        address.province.unwrap_or_default(),
        address.city.unwrap_or_default(),
        address.district.unwrap_or_default(),
        address.address,
        if address.is_default { "checked" } else { "" },
        customer_id
    );

    render_layout("编辑地址", "customers", Some(user), &content)
}

/// 更新地址处理
pub async fn customer_address_update_handler(
    State(state): State<AppState>,
    Path((customer_id, address_id)): Path<(i64, i64)>,
    Form(form): Form<AddressForm>,
) -> Result<impl IntoResponse, Html<String>> {
    let queries = CustomerQueries::new(state.db.pool());

    // 如果设置为默认地址，先清除其他默认地址
    if form.is_default.is_some() {
        let _ = sqlx::query("UPDATE customer_addresses SET is_default = 0 WHERE customer_id = ?")
            .bind(customer_id)
            .execute(state.db.pool())
            .await;
    }

    // 更新地址
    let result = sqlx::query(
        r#"
        UPDATE customer_addresses SET
            receiver_name = ?, receiver_phone = ?, country = ?,
            province = ?, city = ?, district = ?, address = ?, is_default = ?, updated_at = ?
        WHERE id = ? AND customer_id = ?
        "#
    )
    .bind(&form.receiver_name)
    .bind(&form.receiver_phone)
    .bind(&form.country)
    .bind(&form.province)
    .bind(&form.city)
    .bind(&form.district)
    .bind(&form.address)
    .bind(form.is_default.is_some())
    .bind(chrono::Utc::now().to_rfc3339())
    .bind(address_id)
    .bind(customer_id)
    .execute(state.db.pool())
    .await;

    match result {
        Ok(_) => {
            info!("Customer address updated: customer_id={}, address_id={}", customer_id, address_id);
            Ok(Redirect::to(&format!("/customers/{}", customer_id)))
        }
        Err(e) => {
            Err(Html(format!(
                r#"<!DOCTYPE html><html><body><script>alert('更新地址失败: {}');history.back();</script></body></html>"#,
                e
            )))
        }
    }
}

/// 设为默认地址处理
pub async fn customer_address_set_default_handler(
    State(state): State<AppState>,
    Path((customer_id, address_id)): Path<(i64, i64)>,
) -> Result<impl IntoResponse, Html<String>> {
    let queries = CustomerQueries::new(state.db.pool());

    match queries.set_default_address(customer_id, address_id).await {
        Ok(true) => {
            info!("Default address set: customer_id={}, address_id={}", customer_id, address_id);
            Ok(Redirect::to(&format!("/customers/{}", customer_id)))
        }
        Ok(false) => {
            Err(Html(r#"<div class="text-center py-12"><p>地址不存在</p><a href="/customers">返回列表</a></div>"#.to_string()))
        }
        Err(e) => {
            Err(Html(format!(
                r#"<!DOCTYPE html><html><body><script>alert('设置默认地址失败: {}');history.back();</script></body></html>"#,
                e
            )))
        }
    }
}

// ============================================================================
// 供应商新增/详情页面
// ============================================================================

/// 新增供应商页面（简化版）
pub async fn supplier_new_page(
    Extension(auth_user): Extension<AuthUser>,
) -> Html<String> {
    let user = get_user_from_extension(&auth_user);

    let content = r#"<!-- 页面标题 -->
<div class="mb-6">
    <div class="flex items-center gap-2 text-sm text-gray-500 mb-2">
        <a href="/suppliers" class="hover:text-blue-600">供应商列表</a>
        <span>/</span>
        <span class="text-gray-800">新增供应商</span>
    </div>
    <h1 class="text-xl sm:text-2xl font-bold text-gray-800">新增供应商</h1>
</div>

<!-- 供应商表单 -->
<div class="bg-white rounded-xl shadow-sm border border-gray-100">
    <form action="/suppliers/new" method="POST" class="p-4 sm:p-6">
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4 sm:gap-6">
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-2">名称 <span class="text-red-500">*</span></label>
                <input type="text" name="name" required class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500" placeholder="供应商名称">
            </div>
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-2">联系人</label>
                <input type="text" name="contact_person" class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500" placeholder="联系人姓名">
            </div>
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-2">联系电话</label>
                <input type="text" name="contact_phone" class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500" placeholder="联系电话">
            </div>
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-2">邮箱</label>
                <input type="email" name="contact_email" class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500" placeholder="邮箱地址">
            </div>
            <div class="md:col-span-2">
                <label class="block text-sm font-medium text-gray-700 mb-2">地址</label>
                <input type="text" name="address" class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500" placeholder="详细地址">
            </div>
            <div class="md:col-span-2">
                <label class="block text-sm font-medium text-gray-700 mb-2">备注</label>
                <textarea name="notes" rows="2" class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500" placeholder="备注信息..."></textarea>
            </div>
        </div>

        <div class="mt-6 flex items-center gap-4">
            <button type="submit" class="px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors">保存供应商</button>
            <a href="/suppliers" class="px-6 py-2 bg-gray-100 text-gray-700 rounded-lg hover:bg-gray-200 transition-colors">取消</a>
        </div>
    </form>
</div>"#;

    render_layout("新增供应商", "suppliers", Some(user), content)
}

/// 创建供应商表单（简化版）
#[derive(Debug, Deserialize)]
pub struct SupplierForm {
    name: String,
    contact_person: Option<String>,
    contact_phone: Option<String>,
    contact_email: Option<String>,
    address: Option<String>,
    notes: Option<String>,
}

/// 创建供应商处理
pub async fn supplier_create_handler(
    State(state): State<AppState>,
    Form(form): Form<SupplierForm>,
) -> Result<impl IntoResponse, Html<String>> {
    let queries = SupplierQueries::new(state.db.pool());

    let req = CreateSupplierRequest {
        supplier_code: None,  // 自动生成
        name: form.name.clone(),
        name_en: None,
        contact_person: form.contact_person.clone(),
        contact_phone: form.contact_phone.clone(),
        contact_email: form.contact_email.clone(),
        address: form.address.clone(),
        credit_code: None,
        tax_id: None,
        bank_name: None,
        bank_account: None,
        rating_level: None,
        rating_score: None,
        payment_terms: None,
        payment_method: None,
        notes: form.notes.clone(),
    };

    match queries.create(&req).await {
        Ok(supplier) => {
            info!("Supplier created: id={}", supplier.id);
            Ok(Redirect::to(&format!("/suppliers/{}", supplier.id)))
        }
        Err(e) => {
            Err(Html(format!(
                r#"<!DOCTYPE html><html><body><script>alert('创建供应商失败: {}');history.back();</script></body></html>"#,
                e
            )))
        }
    }
}

/// 供应商详情页面
pub async fn supplier_detail_page(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<i64>,
) -> Html<String> {
    let user = get_user_from_extension(&auth_user);

    let queries = SupplierQueries::new(state.db.pool());
    let supplier = match queries.get_by_id(id).await {
        Ok(Some(s)) => s,
        _ => {
            let content = r#"<div class="text-center py-12">
                <p class="text-4xl mb-4">🏭</p>
                <p class="text-gray-600 mb-4">供应商不存在</p>
                <a href="/suppliers" class="text-blue-600 hover:text-blue-800">返回供应商列表</a>
            </div>"#;
            return render_layout("供应商详情", "suppliers", Some(user), content);
        }
    };

    let status_badge = match supplier.status {
        1 => r#"<span class="px-3 py-1 text-sm font-medium bg-green-100 text-green-700 rounded-full">合作中</span>"#,
        2 => r#"<span class="px-3 py-1 text-sm font-medium bg-yellow-100 text-yellow-700 rounded-full">暂停</span>"#,
        _ => r#"<span class="px-3 py-1 text-sm font-medium bg-red-100 text-red-700 rounded-full">终止</span>"#,
    };

    let rating_badge = match supplier.rating_level.as_str() {
        "A" => r#"<span class="px-2 py-1 text-xs font-medium bg-green-100 text-green-700 rounded">A</span>"#,
        "B" => r#"<span class="px-2 py-1 text-xs font-medium bg-blue-100 text-blue-700 rounded">B</span>"#,
        "C" => r#"<span class="px-2 py-1 text-xs font-medium bg-yellow-100 text-yellow-700 rounded">C</span>"#,
        _ => r#"<span class="px-2 py-1 text-xs font-medium bg-red-100 text-red-700 rounded">D</span>"#,
    };

    let content = format!(
        r#"<!-- 页面标题 -->
<div class="mb-6">
    <div class="flex items-center gap-2 text-sm text-gray-500 mb-2">
        <a href="/suppliers" class="hover:text-blue-600">供应商列表</a>
        <span>/</span>
        <span class="text-gray-800">{}</span>
    </div>
    <div class="flex items-center justify-between">
        <div class="flex items-center gap-3">
            <h1 class="text-xl sm:text-2xl font-bold text-gray-800">{}</h1>
            {}
            {}
        </div>
        <div class="flex items-center gap-2">
            <a href="/suppliers/{}/edit" class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors text-sm">编辑</a>
            <a href="/suppliers" class="px-4 py-2 bg-gray-100 text-gray-700 rounded-lg hover:bg-gray-200 transition-colors text-sm">返回列表</a>
        </div>
    </div>
</div>

<!-- 供应商信息 -->
<div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
    <div class="lg:col-span-2 bg-white rounded-xl shadow-sm border border-gray-100 p-4 sm:p-6">
        <h3 class="text-base font-semibold text-gray-800 mb-4">🏭 基本信息</h3>
        <div class="grid grid-cols-2 md:grid-cols-3 gap-4">
            <div>
                <p class="text-xs text-gray-500">供应商编码</p>
                <p class="font-mono text-sm">{}</p>
            </div>
            <div>
                <p class="text-xs text-gray-500">名称</p>
                <p class="font-medium">{}</p>
            </div>
            <div>
                <p class="text-xs text-gray-500">联系人</p>
                <p>{}</p>
            </div>
            <div>
                <p class="text-xs text-gray-500">联系电话</p>
                <p>{}</p>
            </div>
            <div>
                <p class="text-xs text-gray-500">邮箱</p>
                <p>{}</p>
            </div>
            <div>
                <p class="text-xs text-gray-500">创建时间</p>
                <p>{}</p>
            </div>
            <div class="md:col-span-3">
                <p class="text-xs text-gray-500">地址</p>
                <p>{}</p>
            </div>
            <div class="md:col-span-3">
                <p class="text-xs text-gray-500">备注</p>
                <p class="text-sm">{}</p>
            </div>
        </div>
    </div>

    <div class="space-y-6">
        <div class="bg-white rounded-xl shadow-sm border border-gray-100 p-4 sm:p-6">
            <h3 class="text-base font-semibold text-gray-800 mb-4">📊 采购统计</h3>
            <div class="space-y-3">
                <div class="flex justify-between"><span class="text-gray-500">采购订单数</span><span class="font-semibold">{}</span></div>
                <div class="flex justify-between"><span class="text-gray-500">采购总额</span><span class="font-semibold text-green-600">¥{:.2}</span></div>
                <div class="flex justify-between"><span class="text-gray-500">账期（天）</span><span class="font-semibold">{}</span></div>
            </div>
        </div>
    </div>
</div>"#,
        supplier.name,
        supplier.name,
        status_badge,
        rating_badge,
        id,
        supplier.supplier_code,
        supplier.name,
        supplier.contact_person.unwrap_or_default(),
        supplier.contact_phone.unwrap_or_default(),
        supplier.contact_email.unwrap_or_default(),
        supplier.created_at.format("%Y-%m-%d %H:%M"),
        supplier.address.unwrap_or_default(),
        supplier.notes.unwrap_or_default(),
        supplier.total_orders,
        supplier.total_amount,
        supplier.payment_terms
    );

    render_layout("供应商详情", "suppliers", Some(user), &content)
}

/// 供应商编辑页面
pub async fn supplier_edit_page(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<i64>,
) -> Html<String> {
    let user = get_user_from_extension(&auth_user);

    let queries = SupplierQueries::new(state.db.pool());
    let supplier = match queries.get_by_id(id).await {
        Ok(Some(s)) => s,
        _ => {
            let content = r#"<div class="text-center py-12">
                <p class="text-4xl mb-4">🏭</p>
                <p class="text-gray-600 mb-4">供应商不存在</p>
                <a href="/suppliers" class="text-blue-600 hover:text-blue-800">返回供应商列表</a>
            </div>"#;
            return render_layout("编辑供应商", "suppliers", Some(user), content);
        }
    };

    let content = format!(
        r#"<!-- 页面标题 -->
<div class="mb-6">
    <div class="flex items-center gap-2 text-sm text-gray-500 mb-2">
        <a href="/suppliers" class="hover:text-blue-600">供应商列表</a>
        <span>/</span>
        <a href="/suppliers/{}" class="hover:text-blue-600">{}</a>
        <span>/</span>
        <span class="text-gray-800">编辑</span>
    </div>
    <h1 class="text-xl sm:text-2xl font-bold text-gray-800">编辑供应商</h1>
</div>

<!-- 供应商表单 -->
<div class="bg-white rounded-xl shadow-sm border border-gray-100">
    <form action="/suppliers/{}/edit" method="POST" class="p-4 sm:p-6">
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4 sm:gap-6">
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-2">名称 <span class="text-red-500">*</span></label>
                <input type="text" name="name" value="{}" required class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500">
            </div>
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-2">联系人</label>
                <input type="text" name="contact_person" value="{}" class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500">
            </div>
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-2">联系电话</label>
                <input type="text" name="contact_phone" value="{}" class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500">
            </div>
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-2">邮箱</label>
                <input type="email" name="contact_email" value="{}" class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500">
            </div>
            <div class="md:col-span-2">
                <label class="block text-sm font-medium text-gray-700 mb-2">地址</label>
                <input type="text" name="address" value="{}" class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500">
            </div>
            <div class="md:col-span-2">
                <label class="block text-sm font-medium text-gray-700 mb-2">备注</label>
                <textarea name="notes" rows="2" class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500">{}</textarea>
            </div>
        </div>

        <div class="mt-6 flex items-center gap-4">
            <button type="submit" class="px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors">保存</button>
            <a href="/suppliers/{}" class="px-6 py-2 bg-gray-100 text-gray-700 rounded-lg hover:bg-gray-200 transition-colors">取消</a>
        </div>
    </form>
</div>"#,
        id,
        supplier.name,
        id,
        supplier.name,
        supplier.contact_person.unwrap_or_default(),
        supplier.contact_phone.unwrap_or_default(),
        supplier.contact_email.unwrap_or_default(),
        supplier.address.unwrap_or_default(),
        supplier.notes.unwrap_or_default(),
        id
    );

    render_layout("编辑供应商", "suppliers", Some(user), &content)
}

/// 更新供应商处理
pub async fn supplier_update_handler(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Form(form): Form<SupplierForm>,
) -> Result<impl IntoResponse, Html<String>> {
    let queries = SupplierQueries::new(state.db.pool());

    let req = UpdateSupplierRequest {
        name: Some(form.name.clone()),
        name_en: None,
        contact_person: form.contact_person.clone(),
        contact_phone: form.contact_phone.clone(),
        contact_email: form.contact_email.clone(),
        address: form.address.clone(),
        credit_code: None,
        tax_id: None,
        bank_name: None,
        bank_account: None,
        rating_level: None,
        rating_score: None,
        payment_terms: None,
        payment_method: None,
        status: None,
        notes: form.notes.clone(),
    };

    match queries.update(id, &req).await {
        Ok(Some(_)) => {
            info!("Supplier updated: id={}", id);
            Ok(Redirect::to(&format!("/suppliers/{}", id)))
        }
        Ok(None) => {
            Err(Html(r#"<div class="text-center py-12"><p>供应商不存在</p><a href="/suppliers">返回列表</a></div>"#.to_string()))
        }
        Err(e) => {
            Err(Html(format!(
                r#"<!DOCTYPE html><html><body><script>alert('更新供应商失败: {}');history.back();</script></body></html>"#,
                e
            )))
        }
    }
}

// ============================================================================
// 采购管理页面
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct PurchaseQuery {
    page: Option<u32>,
    status: Option<i64>,
}

/// 采购管理列表页面
pub async fn purchase_page(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Query(query): Query<PurchaseQuery>,
) -> Html<String> {
    let user = get_user_from_extension(&auth_user);
    let page = query.page.unwrap_or(1).max(1);
    let page_size = 20;

    let queries = PurchaseQueries::new(state.db.pool());
    let filter = cicierp_models::purchase::PurchaseQuery {
        page: Some(page),
        page_size: Some(page_size),
        status: query.status,
        supplier_id: None,
        payment_status: None,
        delivery_status: None,
        date_from: None,
        date_to: None,
        keyword: None,
    };

    let (items, total) = queries.list(page, page_size, &filter).await.unwrap_or((vec![], 0));
    let total_pages = ((total as f64) / (page_size as f64)).ceil() as u32;

    let rows: String = items.iter().map(|p| {
        let status_badge = match p.status {
            1 => r#"<span class="px-2 py-1 text-xs bg-gray-100 text-gray-700 rounded">草稿</span>"#,
            2 => r#"<span class="px-2 py-1 text-xs bg-yellow-100 text-yellow-700 rounded">待审核</span>"#,
            3 => r#"<span class="px-2 py-1 text-xs bg-blue-100 text-blue-700 rounded">已审核</span>"#,
            4 => r#"<span class="px-2 py-1 text-xs bg-purple-100 text-purple-700 rounded">部分入库</span>"#,
            5 => r#"<span class="px-2 py-1 text-xs bg-green-100 text-green-700 rounded">已完成</span>"#,
            _ => r#"<span class="px-2 py-1 text-xs bg-red-100 text-red-600 rounded">已取消</span>"#,
        };
        // 多供应商模式下，显示"多供应商"或明细数量
        let supplier_display = p.supplier_name.as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| if p.item_count > 1 { "多供应商" } else { "-" });
        format!(
            r#"<tr class="hover:bg-gray-50">
                <td class="px-4 py-3"><span class="font-mono text-sm">{}</span></td>
                <td class="px-4 py-3">{}</td>
                <td class="px-4 py-3 text-center">{}</td>
                <td class="px-4 py-3 text-right">¥{:.2}</td>
                <td class="px-4 py-3 text-center">{}</td>
                <td class="px-4 py-3 text-center">
                    <a href="/purchase/{}" class="text-blue-600 hover:text-blue-800 text-sm">查看</a>
                </td>
            </tr>"#,
            p.order_code,
            supplier_display,
            p.item_count,
            p.total_amount,
            status_badge,
            p.id
        )
    }).collect();

    let rows = if rows.is_empty() {
        r#"<tr><td colspan="6" class="px-6 py-12 text-center"><div class="text-gray-500"><p class="text-4xl mb-2">🛒</p><p>暂无采购单数据</p></div></td></tr>"#.to_string()
    } else {
        rows
    };

    let pagination = if total_pages > 1 {
        format!(r#"<div class="mt-4 text-sm text-gray-600">共 {} 条，第 {}/{} 页</div>"#, total, page, total_pages)
    } else {
        String::new()
    };

    let status_filter = query.status.unwrap_or(0);
    let status_buttons = format!(
        r#"<div class="flex items-center gap-2">
            <a href="/purchase" class="px-3 py-1 text-sm rounded-full {}">全部</a>
            <a href="/purchase?status=1" class="px-3 py-1 text-sm rounded-full {}">草稿</a>
            <a href="/purchase?status=2" class="px-3 py-1 text-sm rounded-full {}">待审核</a>
            <a href="/purchase?status=3" class="px-3 py-1 text-sm rounded-full {}">已审核</a>
            <a href="/purchase?status=5" class="px-3 py-1 text-sm rounded-full {}">已完成</a>
        </div>"#,
        if status_filter == 0 { "bg-blue-100 text-blue-700" } else { "text-gray-600 hover:bg-gray-100" },
        if status_filter == 1 { "bg-blue-100 text-blue-700" } else { "text-gray-600 hover:bg-gray-100" },
        if status_filter == 2 { "bg-blue-100 text-blue-700" } else { "text-gray-600 hover:bg-gray-100" },
        if status_filter == 3 { "bg-blue-100 text-blue-700" } else { "text-gray-600 hover:bg-gray-100" },
        if status_filter == 5 { "bg-blue-100 text-blue-700" } else { "text-gray-600 hover:bg-gray-100" },
    );

    let content = format!(
        r#"<!-- 页面标题 -->
<div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4 mb-6">
    <div>
        <h1 class="text-xl sm:text-2xl font-bold text-gray-800">采购管理</h1>
        <p class="text-gray-600 mt-1 text-sm sm:text-base">一单多供应商采购模式</p>
    </div>
    <div>
        <a href="/purchase/new" class="inline-flex items-center px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors">
            <svg class="w-5 h-5 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"></path></svg>
            新建采购单
        </a>
    </div>
</div>

<!-- 状态筛选 -->
<div class="bg-white rounded-xl shadow-sm border border-gray-100 p-4 mb-6">
    <div class="flex items-center gap-4">
        <span class="text-sm text-gray-500">状态:</span>
        {}
    </div>
</div>

<!-- 采购单表格 -->
<div class="bg-white rounded-xl shadow-sm border border-gray-100 overflow-hidden">
    <div class="overflow-x-auto">
        <table class="w-full min-w-[700px]">
            <thead class="bg-gray-50 border-b border-gray-200">
                <tr>
                    <th class="px-4 sm:px-6 py-4 text-left text-sm font-semibold text-gray-700">采购单号</th>
                    <th class="px-4 sm:px-6 py-4 text-left text-sm font-semibold text-gray-700">供应商</th>
                    <th class="px-4 sm:px-6 py-4 text-center text-sm font-semibold text-gray-700">明细数</th>
                    <th class="px-4 sm:px-6 py-4 text-right text-sm font-semibold text-gray-700">金额</th>
                    <th class="px-4 sm:px-6 py-4 text-center text-sm font-semibold text-gray-700">状态</th>
                    <th class="px-4 sm:px-6 py-4 text-center text-sm font-semibold text-gray-700">操作</th>
                </tr>
            </thead>
            <tbody class="divide-y divide-gray-100">{}</tbody>
        </table>
    </div>
</div>
{}"#,
        status_buttons,
        rows,
        pagination
    );

    render_layout("采购管理", "purchase", Some(user), &content)
}

// ============================================================================
// 采购单创建页面
// ============================================================================

/// 采购单创建页面
pub async fn purchase_new_page(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
) -> Html<String> {
    let user = get_user_from_extension(&auth_user);

    // 获取供应商列表
    let supplier_queries = SupplierQueries::new(state.db.pool());
    let suppliers = match supplier_queries.list(1, 1000, Some(1), None, None).await {
        Ok(resp) => resp.items,
        Err(_) => vec![],
    };

    // 获取产品列表
    let product_queries = ProductQueries::new(state.db.pool());
    let products = match product_queries.list(1, 1000, None, None, None, None).await {
        Ok(resp) => resp.items,
        Err(_) => vec![],
    };

    let supplier_options: String = suppliers.iter().map(|s| {
        format!(r#"<option value="{}">{}</option>"#, s.id, s.name)
    }).collect();

    let product_options: String = products.iter().map(|p| {
        format!(r#"<option value="{}" data-name="{}" data-price="{}">{}</option>"#,
            p.id, p.name, p.sale_price_cny.unwrap_or(0.0), p.name)
    }).collect();

    let content = format!(
        r#"<!-- 页面标题 -->
<div class="flex items-center justify-between mb-6">
    <div>
        <h1 class="text-xl sm:text-2xl font-bold text-gray-800">新建采购单</h1>
        <p class="text-gray-600 mt-1 text-sm sm:text-base">一单多供应商采购模式</p>
    </div>
    <a href="/purchase" class="text-gray-600 hover:text-gray-800">
        <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path></svg>
    </a>
</div>

<form id="purchase-form" method="POST" action="/purchase/new">
    <!-- 基本信息 -->
    <div class="bg-white rounded-xl shadow-sm border border-gray-100 p-6 mb-6">
        <h3 class="font-semibold text-gray-800 mb-4">基本信息</h3>
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">预计到货日期</label>
                <input type="date" name="expected_date" class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500">
            </div>
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">备注</label>
                <input type="text" name="internal_note" placeholder="内部备注" class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500">
            </div>
        </div>
    </div>

    <!-- 添加产品 -->
    <div class="bg-white rounded-xl shadow-sm border border-gray-100 p-6 mb-6">
        <h3 class="font-semibold text-gray-800 mb-4">添加产品</h3>
        <div class="grid grid-cols-1 md:grid-cols-5 gap-4 items-end">
            <div class="md:col-span-2">
                <label class="block text-sm font-medium text-gray-700 mb-1">产品</label>
                <select id="product-select" class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500">
                    <option value="">选择产品...</option>
                    {}
                </select>
            </div>
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">供应商</label>
                <select id="supplier-select" class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500">
                    <option value="">选择供应商...</option>
                    {}
                </select>
            </div>
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">数量</label>
                <input type="number" id="quantity-input" min="1" value="1" class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500">
            </div>
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">单价</label>
                <div class="flex gap-2">
                    <input type="number" id="price-input" step="0.01" min="0" value="0" class="flex-1 px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500">
                    <button type="button" onclick="addPurchaseItem()" class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors">
                        添加
                    </button>
                </div>
            </div>
        </div>
    </div>

    <!-- 产品列表 -->
    <div class="bg-white rounded-xl shadow-sm border border-gray-100 p-6 mb-6">
        <h3 class="font-semibold text-gray-800 mb-4">采购明细</h3>
        <div class="overflow-x-auto">
            <table class="w-full">
                <thead class="bg-gray-50">
                    <tr>
                        <th class="px-4 py-3 text-left text-sm font-semibold text-gray-700">产品</th>
                        <th class="px-4 py-3 text-left text-sm font-semibold text-gray-700">供应商</th>
                        <th class="px-4 py-3 text-right text-sm font-semibold text-gray-700">数量</th>
                        <th class="px-4 py-3 text-right text-sm font-semibold text-gray-700">单价</th>
                        <th class="px-4 py-3 text-right text-sm font-semibold text-gray-700">小计</th>
                        <th class="px-4 py-3 text-center text-sm font-semibold text-gray-700">操作</th>
                    </tr>
                </thead>
                <tbody id="items-body">
                    <tr id="empty-row">
                        <td colspan="6" class="px-4 py-8 text-center text-gray-500">
                            请添加采购产品
                        </td>
                    </tr>
                </tbody>
                <tfoot class="bg-gray-50">
                    <tr>
                        <td colspan="4" class="px-4 py-3 text-right font-semibold text-gray-700">合计:</td>
                        <td class="px-4 py-3 text-right font-semibold text-gray-800">¥<span id="total-amount">0.00</span></td>
                        <td></td>
                    </tr>
                </tfoot>
            </table>
        </div>
        <input type="hidden" name="items" id="items-json">
    </div>

    <!-- 提交按钮 -->
    <div class="flex justify-end gap-4">
        <a href="/purchase" class="px-6 py-2 border border-gray-300 text-gray-700 rounded-lg hover:bg-gray-50 transition-colors">
            取消
        </a>
        <button type="submit" class="px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors">
            创建采购单
        </button>
    </div>
</form>

<script>
let purchaseItems = [];

function addPurchaseItem() {{
    const productSelect = document.getElementById('product-select');
    const supplierSelect = document.getElementById('supplier-select');
    const quantityInput = document.getElementById('quantity-input');
    const priceInput = document.getElementById('price-input');

    const productId = productSelect.value;
    const productName = productSelect.options[productSelect.selectedIndex].text;
    const supplierId = supplierSelect.value;
    const supplierName = supplierSelect.options[supplierSelect.selectedIndex].text;
    const quantity = parseInt(quantityInput.value) || 0;
    const price = parseFloat(priceInput.value) || 0;

    if (!productId || !supplierId || quantity <= 0 || price <= 0) {{
        alert('请填写完整的产品信息');
        return;
    }}

    // 检查产品是否已添加
    if (purchaseItems.find(item => item.productId === productId)) {{
        alert('该产品已添加，请勿重复添加');
        return;
    }}

    const subtotal = quantity * price;
    purchaseItems.push({{
        productId,
        productName,
        supplierId,
        supplierName,
        quantity,
        price,
        subtotal
    }});

    renderItems();

    // 重置输入
    productSelect.value = '';
    supplierSelect.value = '';
    quantityInput.value = 1;
    priceInput.value = 0;
}}

function removeItem(index) {{
    purchaseItems.splice(index, 1);
    renderItems();
}}

function renderItems() {{
    const tbody = document.getElementById('items-body');
    const emptyRow = document.getElementById('empty-row');

    if (purchaseItems.length === 0) {{
        tbody.innerHTML = '<tr id="empty-row"><td colspan="6" class="px-4 py-8 text-center text-gray-500">请添加采购产品</td></tr>';
        document.getElementById('total-amount').textContent = '0.00';
        document.getElementById('items-json').value = '';
        return;
    }}

    let html = '';
    let total = 0;
    purchaseItems.forEach((item, index) => {{
        total += item.subtotal;
        html += `<tr class="border-b border-gray-100">
            <td class="px-4 py-3">${{item.productName}}</td>
            <td class="px-4 py-3">${{item.supplierName}}</td>
            <td class="px-4 py-3 text-right">${{item.quantity}}</td>
            <td class="px-4 py-3 text-right">¥${{item.price.toFixed(2)}}</td>
            <td class="px-4 py-3 text-right">¥${{item.subtotal.toFixed(2)}}</td>
            <td class="px-4 py-3 text-center">
                <button type="button" onclick="removeItem(${{index}})" class="text-red-600 hover:text-red-800">删除</button>
            </td>
        </tr>`;
    }});
    tbody.innerHTML = html;
    document.getElementById('total-amount').textContent = total.toFixed(2);

    // 更新隐藏字段
    const itemsJson = purchaseItems.map(item => ({{
        product_id: parseInt(item.productId),
        product_name: item.productName,
        supplier_id: parseInt(item.supplierId),
        quantity: item.quantity,
        unit_price: item.price
    }}));
    document.getElementById('items-json').value = JSON.stringify(itemsJson);
}}

document.getElementById('purchase-form').addEventListener('submit', function(e) {{
    if (purchaseItems.length === 0) {{
        e.preventDefault();
        alert('请至少添加一个产品');
    }}
}});
</script>"#,
        product_options,
        supplier_options
    );

    render_layout("新建采购单", "purchase", Some(user), &content)
}

/// 采购单创建处理
#[derive(Debug, Deserialize)]
pub struct PurchaseCreateForm {
    expected_date: Option<String>,
    internal_note: Option<String>,
    items: String,  // JSON 字符串
}

pub async fn purchase_create_handler(
    State(state): State<AppState>,
    Extension(_auth_user): Extension<AuthUser>,
    Form(form): Form<PurchaseCreateForm>,
) -> Response {
    // 解析 JSON
    let items: Vec<cicierp_models::purchase::PurchaseItemRequest> = match serde_json::from_str(&form.items) {
        Ok(items) => items,
        Err(e) => {
            return Html(format!(r#"<div class="p-4 text-red-600">解析产品数据失败: {}</div>"#, e)).into_response();
        }
    };

    if items.is_empty() {
        return Html(r#"<div class="p-4 text-red-600">请至少添加一个产品</div>"#.to_string()).into_response();
    }

    let req = cicierp_models::purchase::CreatePurchaseOrderRequest {
        items,
        expected_date: form.expected_date,
        supplier_note: None,
        internal_note: form.internal_note,
        tax_amount: None,
    };

    let queries = PurchaseQueries::new(state.db.pool());
    match queries.create(&req).await {
        Ok(order) => Redirect::to(&format!("/purchase/{}", order.id)).into_response(),
        Err(e) => Html(format!(r#"<div class="p-4 text-red-600">创建采购单失败: {}</div>"#, e)).into_response(),
    }
}

// ============================================================================
// 采购单详情页面
// ============================================================================

/// 采购单详情页面
pub async fn purchase_detail_page(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<i64>,
) -> Html<String> {
    let user = get_user_from_extension(&auth_user);

    let queries = PurchaseQueries::new(state.db.pool());
    let detail = match queries.get_detail(id).await {
        Ok(Some(d)) => d,
        Ok(None) => return Html(r#"<div class="p-4 text-red-600">采购单不存在</div>"#.to_string()),
        Err(e) => return Html(format!(r#"<div class="p-4 text-red-600">查询失败: {}</div>"#, e)),
    };

    let status_badge = match detail.order.status {
        1 => r#"<span class="px-3 py-1 text-sm bg-gray-100 text-gray-700 rounded-full">草稿</span>"#,
        2 => r#"<span class="px-3 py-1 text-sm bg-yellow-100 text-yellow-700 rounded-full">待审核</span>"#,
        3 => r#"<span class="px-3 py-1 text-sm bg-blue-100 text-blue-700 rounded-full">已审核</span>"#,
        4 => r#"<span class="px-3 py-1 text-sm bg-purple-100 text-purple-700 rounded-full">部分入库</span>"#,
        5 => r#"<span class="px-3 py-1 text-sm bg-green-100 text-green-700 rounded-full">已完成</span>"#,
        _ => r#"<span class="px-3 py-1 text-sm bg-red-100 text-red-600 rounded-full">已取消</span>"#,
    };

    // 按供应商分组
    let mut supplier_groups: std::collections::HashMap<String, Vec<&cicierp_models::purchase::PurchaseOrderItem>> = std::collections::HashMap::new();
    for item in &detail.items {
        let key = item.supplier_name.clone().unwrap_or_else(|| "未知供应商".to_string());
        supplier_groups.entry(key).or_insert_with(Vec::new).push(item);
    }

    let supplier_sections: String = supplier_groups.iter().map(|(supplier_name, items)| {
        let items_html: String = items.iter().map(|item| {
            format!(
                r#"<tr class="border-b border-gray-100">
                    <td class="px-4 py-3">{}</td>
                    <td class="px-4 py-3 text-center">{}</td>
                    <td class="px-4 py-3 text-center">{}</td>
                    <td class="px-4 py-3 text-right">¥{:.2}</td>
                    <td class="px-4 py-3 text-right">¥{:.2}</td>
                </tr>"#,
                item.product_name,
                item.quantity,
                item.received_qty,
                item.unit_price,
                item.subtotal
            )
        }).collect();

        let group_total: f64 = items.iter().map(|i| i.subtotal).sum();

        format!(
            r#"<div class="mb-6">
                <div class="flex items-center justify-between bg-gray-50 px-4 py-2 rounded-t-lg border border-gray-200 border-b-0">
                    <h4 class="font-medium text-gray-800">📦 {}</h4>
                    <span class="text-sm text-gray-600">小计: ¥{:.2}</span>
                </div>
                <div class="overflow-x-auto border border-gray-200 rounded-b-lg">
                    <table class="w-full">
                        <thead class="bg-gray-50">
                            <tr>
                                <th class="px-4 py-2 text-left text-sm font-semibold text-gray-700">产品</th>
                                <th class="px-4 py-2 text-center text-sm font-semibold text-gray-700">采购数量</th>
                                <th class="px-4 py-2 text-center text-sm font-semibold text-gray-700">已入库</th>
                                <th class="px-4 py-2 text-right text-sm font-semibold text-gray-700">单价</th>
                                <th class="px-4 py-2 text-right text-sm font-semibold text-gray-700">小计</th>
                            </tr>
                        </thead>
                        <tbody>{}</tbody>
                    </table>
                </div>
            </div>"#,
            supplier_name,
            group_total,
            items_html
        )
    }).collect();

    let action_buttons = if detail.order.status == 1 {
        r#"<form method="POST" action="" class="inline">
            <button type="submit" class="px-4 py-2 bg-yellow-500 text-white rounded-lg hover:bg-yellow-600 transition-colors">
                提交审核
            </button>
        </form>"#
    } else if detail.order.status == 2 {
        r#"<form method="POST" action="" class="inline">
            <button type="submit" class="px-4 py-2 bg-green-500 text-white rounded-lg hover:bg-green-600 transition-colors">
                审批通过
            </button>
        </form>"#
    } else {
        ""
    };

    let content = format!(
        r#"<!-- 页面标题 -->
<div class="flex items-center justify-between mb-6">
    <div class="flex items-center gap-4">
        <a href="/purchase" class="text-gray-600 hover:text-gray-800">
            <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7"></path></svg>
        </a>
        <div>
            <h1 class="text-xl sm:text-2xl font-bold text-gray-800">采购单详情</h1>
            <p class="text-gray-600 mt-1 text-sm">{} - {} 项产品</p>
        </div>
    </div>
    <div class="flex items-center gap-4">
        {}
        {}
    </div>
</div>

<!-- 基本信息 -->
<div class="bg-white rounded-xl shadow-sm border border-gray-100 p-6 mb-6">
    <div class="grid grid-cols-1 md:grid-cols-4 gap-6">
        <div>
            <span class="text-sm text-gray-500">采购单号</span>
            <p class="font-mono font-semibold">{}</p>
        </div>
        <div>
            <span class="text-sm text-gray-500">总金额</span>
            <p class="font-semibold text-lg">¥{:.2}</p>
        </div>
        <div>
            <span class="text-sm text-gray-500">预计到货</span>
            <p>{}</p>
        </div>
        <div>
            <span class="text-sm text-gray-500">创建时间</span>
            <p>{}</p>
        </div>
    </div>
    {}
</div>

<!-- 产品明细（按供应商分组） -->
<div class="bg-white rounded-xl shadow-sm border border-gray-100 p-6">
    <h3 class="font-semibold text-gray-800 mb-4">产品明细</h3>
    {}
</div>"#,
        detail.order.order_code,
        detail.items.len(),
        status_badge,
        action_buttons,
        detail.order.order_code,
        detail.order.total_amount,
        detail.order.expected_date.as_deref().unwrap_or("-"),
        detail.order.created_at,
        if detail.order.internal_note.is_some() { format!(r#"<div class="mt-4 pt-4 border-t border-gray-200"><span class="text-sm text-gray-500">备注:</span> {}</div>"#, detail.order.internal_note.as_deref().unwrap_or("")) } else { String::new() },
        supplier_sections
    );

    render_layout("采购单详情", "purchase", Some(user), &content)
}

// ============================================================================
// 物流管理页面
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct LogisticsQuery {
    page: Option<u32>,
    status: Option<i64>,
}

/// 物流管理列表页面
pub async fn logistics_page(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Query(query): Query<LogisticsQuery>,
) -> Html<String> {
    let user = get_user_from_extension(&auth_user);
    let page = query.page.unwrap_or(1).max(1);
    let page_size = 20;

    // 获取发货单列表
    let shipment_queries = ShipmentQueries::new(state.db.pool());
    let filter = cicierp_models::logistics::ShipmentQuery {
        page: Some(page),
        page_size: Some(page_size),
        order_id: None,
        logistics_id: None,
        status: query.status,
        tracking_number: None,
        date_from: None,
        date_to: None,
    };

    let (shipments, total) = shipment_queries.list(page, page_size, &filter).await.unwrap_or((vec![], 0));

    // 获取物流公司列表
    let company_queries = LogisticsCompanyQueries::new(state.db.pool());
    let companies = company_queries.list().await.unwrap_or_default();

    let shipment_rows: String = shipments.iter().map(|s| {
        let status_badge = match s.status {
            1 => r#"<span class="px-2 py-1 text-xs bg-yellow-100 text-yellow-700 rounded">待发货</span>"#,
            2 => r#"<span class="px-2 py-1 text-xs bg-blue-100 text-blue-700 rounded">运输中</span>"#,
            3 => r#"<span class="px-2 py-1 text-xs bg-green-100 text-green-700 rounded">已签收</span>"#,
            _ => r#"<span class="px-2 py-1 text-xs bg-gray-100 text-gray-600 rounded">已取消</span>"#,
        };
        format!(
            r#"<tr class="hover:bg-gray-50">
                <td class="px-4 py-3"><span class="font-mono text-sm">{}</span></td>
                <td class="px-4 py-3">{}</td>
                <td class="px-4 py-3"><span class="font-mono text-sm">{}</span></td>
                <td class="px-4 py-3">{}</td>
                <td class="px-4 py-3 text-center">{}</td>
                <td class="px-4 py-3 text-center">
                    <a href="/logistics" class="text-blue-600 hover:text-blue-800 text-sm">查看</a>
                </td>
            </tr>"#,
            s.shipment_code,
            s.logistics_name.clone().unwrap_or_default(),
            s.tracking_number.clone().unwrap_or_default(),
            s.receiver_name,
            status_badge
        )
    }).collect();

    let shipment_rows = if shipment_rows.is_empty() {
        r#"<tr><td colspan="6" class="px-6 py-12 text-center"><div class="text-gray-500"><p class="text-4xl mb-2">🚚</p><p>暂无发货单数据</p></div></td></tr>"#.to_string()
    } else {
        shipment_rows
    };

    let company_rows: String = companies.iter().take(5).map(|c| {
        let status = if c.status == 1 { "正常" } else { "禁用" };
        format!(
            r#"<tr class="hover:bg-gray-50">
                <td class="px-3 py-2">{}</td>
                <td class="px-3 py-2">{}</td>
                <td class="px-3 py-2 text-center">{}</td>
            </tr>"#,
            c.code,
            c.name,
            status
        )
    }).collect();

    let total_pages = ((total as f64) / (page_size as f64)).ceil() as u32;
    let pagination = if total_pages > 1 {
        format!(r#"<div class="mt-4 text-sm text-gray-600">共 {} 条，第 {}/{} 页</div>"#, total, page, total_pages)
    } else {
        String::new()
    };

    let content = format!(
        r#"<!-- 页面标题 -->
<div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4 mb-6">
    <div>
        <h1 class="text-xl sm:text-2xl font-bold text-gray-800">物流管理</h1>
        <p class="text-gray-600 mt-1 text-sm sm:text-base">管理发货单和物流公司</p>
    </div>
</div>

<div class="grid grid-cols-1 lg:grid-cols-4 gap-6">
    <!-- 发货单列表 -->
    <div class="lg:col-span-3">
        <div class="bg-white rounded-xl shadow-sm border border-gray-100 overflow-hidden">
            <div class="p-4 border-b border-gray-100">
                <h3 class="font-semibold text-gray-800">📦 发货单列表</h3>
            </div>
            <div class="overflow-x-auto">
                <table class="w-full min-w-[700px]">
                    <thead class="bg-gray-50 border-b border-gray-200">
                        <tr>
                            <th class="px-4 sm:px-6 py-4 text-left text-sm font-semibold text-gray-700">发货单号</th>
                            <th class="px-4 sm:px-6 py-4 text-left text-sm font-semibold text-gray-700">物流公司</th>
                            <th class="px-4 sm:px-6 py-4 text-left text-sm font-semibold text-gray-700">快递单号</th>
                            <th class="px-4 sm:px-6 py-4 text-left text-sm font-semibold text-gray-700">收件人</th>
                            <th class="px-4 sm:px-6 py-4 text-center text-sm font-semibold text-gray-700">状态</th>
                            <th class="px-4 sm:px-6 py-4 text-center text-sm font-semibold text-gray-700">操作</th>
                        </tr>
                    </thead>
                    <tbody class="divide-y divide-gray-100">{}</tbody>
                </table>
            </div>
            {}
        </div>
    </div>

    <!-- 物流公司 -->
    <div class="bg-white rounded-xl shadow-sm border border-gray-100 overflow-hidden">
        <div class="p-4 border-b border-gray-100">
            <h3 class="font-semibold text-gray-800">🚚 物流公司</h3>
        </div>
        <div class="overflow-x-auto">
            <table class="w-full">
                <thead class="bg-gray-50">
                    <tr>
                        <th class="px-3 py-2 text-left text-xs font-semibold text-gray-700">编码</th>
                        <th class="px-3 py-2 text-left text-xs font-semibold text-gray-700">名称</th>
                        <th class="px-3 py-2 text-center text-xs font-semibold text-gray-700">状态</th>
                    </tr>
                </thead>
                <tbody class="divide-y divide-gray-100">{}</tbody>
            </table>
        </div>
    </div>
</div>"#,
        shipment_rows,
        pagination,
        company_rows
    );

    render_layout("物流管理", "logistics", Some(user), &content)
}

// ============================================================================
// 分析报告
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct AnalyticsQuery {
    pub year: Option<i32>,
    pub month: Option<i32>,
}

/// 分析报告页面
pub async fn analytics_page(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Query(query): Query<AnalyticsQuery>,
) -> Html<String> {
    let user = get_user_from_extension(&auth_user);

    // 获取当前年月或使用查询参数
    let now = Local::now();
    let year = query.year.unwrap_or_else(|| now.year());
    let month = query.month.unwrap_or_else(|| now.month() as i32);

    // 获取分析数据
    let queries = OrderQueries::new(state.db.pool());
    let report = queries.get_analytics(year, month).await.unwrap_or_else(|_| {
        cicierp_db::queries::orders::AnalyticsReport {
            sales_by_currency: vec![],
            top_products: vec![],
            platform_distribution: vec![],
        }
    });

    // 币种销售统计
    let sales_rows: String = report.sales_by_currency.iter().map(|s| {
        let profit_rate = if s.total_sales > 0.0 {
            (s.total_profit / s.total_sales * 100.0) as i32
        } else {
            0
        };
        let currency_symbol = if s.currency == "USD" { "$" } else { "¥" };
        format!(
            r#"<tr class="hover:bg-gray-50">
                <td class="px-4 py-3 font-medium">{}</td>
                <td class="px-4 py-3 text-right">{}{:.2}</td>
                <td class="px-4 py-3 text-right">{}{:.2}</td>
                <td class="px-4 py-3 text-right font-medium text-green-600">{}{:.2}</td>
                <td class="px-4 py-3 text-center">{}%</td>
                <td class="px-4 py-3 text-center">{}</td>
            </tr>"#,
            s.currency,
            currency_symbol, s.total_sales,
            currency_symbol, s.total_cost,
            currency_symbol, s.total_profit,
            profit_rate,
            s.order_count
        )
    }).collect();

    let sales_rows = if sales_rows.is_empty() {
        r#"<tr><td colspan="6" class="px-6 py-8 text-center text-gray-500">暂无销售数据</td></tr>"#.to_string()
    } else {
        sales_rows
    };

    // 产品销量 Top 10
    let product_rows: String = report.top_products.iter().enumerate().map(|(i, p)| {
        format!(
            r#"<tr class="hover:bg-gray-50">
                <td class="px-4 py-3 text-center font-bold text-gray-400">{}</td>
                <td class="px-4 py-3">{}</td>
                <td class="px-4 py-3 text-right font-medium">{}</td>
                <td class="px-4 py-3 text-right">${:.2}</td>
            </tr>"#,
            i + 1,
            p.product_name,
            p.total_quantity,
            p.total_sales
        )
    }).collect();

    let product_rows = if product_rows.is_empty() {
        r#"<tr><td colspan="4" class="px-6 py-8 text-center text-gray-500">暂无产品数据</td></tr>"#.to_string()
    } else {
        product_rows
    };

    // 平台分布
    let platform_rows: String = report.platform_distribution.iter().map(|p| {
        let platform_name = match p.platform.as_str() {
            "ali_import" => "阿里进口",
            "import" => "自主进口",
            _ => &p.platform,
        };
        format!(
            r#"<tr class="hover:bg-gray-50">
                <td class="px-4 py-3">{}</td>
                <td class="px-4 py-3 text-center">{}</td>
                <td class="px-4 py-3 text-right">${:.2}</td>
            </tr>"#,
            platform_name,
            p.order_count,
            p.total_sales
        )
    }).collect();

    let platform_rows = if platform_rows.is_empty() {
        r#"<tr><td colspan="3" class="px-6 py-8 text-center text-gray-500">暂无平台数据</td></tr>"#.to_string()
    } else {
        platform_rows
    };

    // 年月选择器
    let current_year = Local::now().year();
    let year_options: String = (2024..=current_year).map(|y| {
        if y == year {
            format!(r#"<option value="{}" selected>{}</option>"#, y, y)
        } else {
            format!(r#"<option value="{}">{}</option>"#, y, y)
        }
    }).collect();

    let month_options: String = (1..=12).map(|m| {
        if m == month {
            format!(r#"<option value="{}" selected>{}月</option>"#, m, m)
        } else {
            format!(r#"<option value="{}">{}月</option>"#, m, m)
        }
    }).collect();

    let content = format!(
        r#"<!-- 页面标题 -->
<div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4 mb-6">
    <div>
        <h1 class="text-xl sm:text-2xl font-bold text-gray-800">分析报告</h1>
        <p class="text-gray-600 mt-1 text-sm sm:text-base">月度销售数据分析</p>
    </div>
    <div class="flex items-center gap-3">
        <form method="GET" class="flex items-center gap-2">
            <select name="year" class="px-3 py-2 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500">
                {}
            </select>
            <select name="month" class="px-3 py-2 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500">
                {}
            </select>
            <button type="submit" class="px-4 py-2 bg-blue-600 text-white rounded-lg text-sm hover:bg-blue-700 transition-colors">
                查询
            </button>
        </form>
    </div>
</div>

<div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
    <!-- 销售额统计 -->
    <div class="bg-white rounded-xl shadow-sm border border-gray-100 overflow-hidden">
        <div class="p-4 border-b border-gray-100 bg-gradient-to-r from-blue-50 to-indigo-50">
            <h3 class="font-semibold text-gray-800">💰 销售额统计（按币种）</h3>
        </div>
        <div class="overflow-x-auto">
            <table class="w-full">
                <thead class="bg-gray-50">
                    <tr>
                        <th class="px-4 py-3 text-left text-sm font-semibold text-gray-700">币种</th>
                        <th class="px-4 py-3 text-right text-sm font-semibold text-gray-700">销售额</th>
                        <th class="px-4 py-3 text-right text-sm font-semibold text-gray-700">成本</th>
                        <th class="px-4 py-3 text-right text-sm font-semibold text-gray-700">利润</th>
                        <th class="px-4 py-3 text-center text-sm font-semibold text-gray-700">利润率</th>
                        <th class="px-4 py-3 text-center text-sm font-semibold text-gray-700">订单数</th>
                    </tr>
                </thead>
                <tbody class="divide-y divide-gray-100">{}</tbody>
            </table>
        </div>
    </div>

    <!-- 平台分布 -->
    <div class="bg-white rounded-xl shadow-sm border border-gray-100 overflow-hidden">
        <div class="p-4 border-b border-gray-100 bg-gradient-to-r from-green-50 to-emerald-50">
            <h3 class="font-semibold text-gray-800">📊 平台分布</h3>
        </div>
        <div class="overflow-x-auto">
            <table class="w-full">
                <thead class="bg-gray-50">
                    <tr>
                        <th class="px-4 py-3 text-left text-sm font-semibold text-gray-700">平台</th>
                        <th class="px-4 py-3 text-center text-sm font-semibold text-gray-700">订单数</th>
                        <th class="px-4 py-3 text-right text-sm font-semibold text-gray-700">销售额</th>
                    </tr>
                </thead>
                <tbody class="divide-y divide-gray-100">{}</tbody>
            </table>
        </div>
    </div>

    <!-- 产品销量 Top 10 -->
    <div class="bg-white rounded-xl shadow-sm border border-gray-100 overflow-hidden lg:col-span-2">
        <div class="p-4 border-b border-gray-100 bg-gradient-to-r from-orange-50 to-amber-50">
            <h3 class="font-semibold text-gray-800">🏆 产品销量 Top 10</h3>
        </div>
        <div class="overflow-x-auto">
            <table class="w-full">
                <thead class="bg-gray-50">
                    <tr>
                        <th class="px-4 py-3 text-center text-sm font-semibold text-gray-700 w-16">排名</th>
                        <th class="px-4 py-3 text-left text-sm font-semibold text-gray-700">产品名称</th>
                        <th class="px-4 py-3 text-right text-sm font-semibold text-gray-700">销量</th>
                        <th class="px-4 py-3 text-right text-sm font-semibold text-gray-700">销售额</th>
                    </tr>
                </thead>
                <tbody class="divide-y divide-gray-100">{}</tbody>
            </table>
        </div>
    </div>
</div>

<!-- 数据说明 -->
<div class="mt-6 bg-blue-50 rounded-xl border border-blue-100 p-4">
    <div class="flex items-start gap-3">
        <span class="text-2xl">💡</span>
        <div class="text-sm text-gray-700">
            <p class="font-medium mb-1">数据说明</p>
            <ul class="list-disc list-inside space-y-1 text-gray-600">
                <li>利润 = 销售额 - 成本（order_items.cost_price × quantity）</li>
                <li>仅统计当月已完成的订单</li>
                <li>平台：阿里进口 = 从阿里巴巴导入的订单，自主进口 = 手动创建的订单</li>
            </ul>
        </div>
    </div>
</div>"#,
        year_options,
        month_options,
        sales_rows,
        platform_rows,
        product_rows
    );

    render_layout("分析报告", "analytics", Some(user), &content)
}
