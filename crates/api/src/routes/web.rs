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
use serde::{Deserialize, Deserializer};
use tracing::info;

/// 将 HTML 表单中空字符串的整数字段反序列化为 None
fn empty_string_as_none_i64<'de, D>(de: D) -> Result<Option<i64>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(de)?;
    match s.as_deref() {
        None | Some("") => Ok(None),
        Some(v) => v.parse::<i64>().map(Some).map_err(serde::de::Error::custom),
    }
}

/// 将 HTML 表单中空字符串的浮点数字段反序列化为 None
fn empty_string_as_none_f64<'de, D>(de: D) -> Result<Option<f64>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(de)?;
    match s.as_deref() {
        None | Some("") => Ok(None),
        Some(v) => v.parse::<f64>().map(Some).map_err(serde::de::Error::custom),
    }
}

fn empty_string_as_none_i32<'de, D>(de: D) -> Result<Option<i32>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(de)?;
    match s.as_deref() {
        None | Some("") => Ok(None),
        Some(v) => v.parse::<i32>().map(Some).map_err(serde::de::Error::custom),
    }
}

fn empty_string_as_none_u32<'de, D>(de: D) -> Result<Option<u32>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(de)?;
    match s.as_deref() {
        None | Some("") => Ok(None),
        Some(v) => v.parse::<u32>().map(Some).map_err(serde::de::Error::custom),
    }
}

/// serde_urlencoded 对单值不自动包装成 Vec，需要自定义 visitor 同时接受 str 和 seq
fn str_or_vec_string<'de, D>(de: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    struct Visitor;
    impl<'de> serde::de::Visitor<'de> for Visitor {
        type Value = Vec<String>;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("string or sequence of strings")
        }
        fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
            Ok(if v.is_empty() { vec![] } else { vec![v.to_owned()] })
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut out = Vec::new();
            while let Some(v) = seq.next_element::<String>()? {
                out.push(v);
            }
            Ok(out)
        }
    }
    de.deserialize_any(Visitor)
}

fn str_or_vec_i64<'de, D>(de: D) -> Result<Vec<i64>, D::Error>
where
    D: Deserializer<'de>,
{
    struct Visitor;
    impl<'de> serde::de::Visitor<'de> for Visitor {
        type Value = Vec<i64>;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("integer or sequence of integers")
        }
        fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
            if v.is_empty() { return Ok(vec![]); }
            v.parse::<i64>().map(|n| vec![n]).map_err(E::custom)
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut out = Vec::new();
            while let Some(s) = seq.next_element::<String>()? {
                if !s.is_empty() {
                    out.push(s.parse::<i64>().map_err(|e| serde::de::Error::custom(e.to_string()))?);
                }
            }
            Ok(out)
        }
        fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Self::Value, E> { Ok(vec![v]) }
        fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Self::Value, E> { Ok(vec![v as i64]) }
    }
    de.deserialize_any(Visitor)
}

fn str_or_vec_f64<'de, D>(de: D) -> Result<Vec<f64>, D::Error>
where
    D: Deserializer<'de>,
{
    struct Visitor;
    impl<'de> serde::de::Visitor<'de> for Visitor {
        type Value = Vec<f64>;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("float or sequence of floats")
        }
        fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
            if v.is_empty() { return Ok(vec![]); }
            v.parse::<f64>().map(|n| vec![n]).map_err(E::custom)
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut out = Vec::new();
            while let Some(s) = seq.next_element::<String>()? {
                if !s.is_empty() {
                    out.push(s.parse::<f64>().map_err(|e| serde::de::Error::custom(e.to_string()))?);
                }
            }
            Ok(out)
        }
        fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Self::Value, E> { Ok(vec![v]) }
    }
    de.deserialize_any(Visitor)
}

use crate::middleware::auth::AuthUser;
use crate::state::AppState;
use crate::templates::base::{get_menus, UserInfo};
use crate::templates::dashboard::DashboardStats;
use cicierp_db::queries::{
    customers::{CustomerQueries, CreateAddressRequest},
    exchange_rates::ExchangeRateQueries,
    inventory::InventoryQueries,
    logistics::{LogisticsCompanyQueries, ShipmentQueries},
    orders::{OrderQueries, OrderFilterStats},
    product_content::ProductContentQueries,
    product_costs::{ProductCostQueries, ReferenceCostWrite},
    product_prices::{ProductPriceQueries, ReferencePriceWrite},
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
use cicierp_models::product::{CreateProductCostRequest, CreateProductRequest, UpdateProductRequest};
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
        .route("/products/export", get(product_export_handler))
        .route("/products/import/template", get(product_import_template_handler))
        .route("/products/import", post(product_import_handler))
        .route("/products/new", get(product_new_page).post(product_create_handler))
        .route("/products/:id", get(product_detail_page))
        .route("/products/:id/edit", get(product_edit_page).post(product_update_handler))
        // 订单
        .route("/orders", get(orders_page))
        .route("/orders/import/template", get(order_import_template_handler))
        .route("/orders/import", post(order_import_handler))
        .route("/orders/import/ae", get(order_import_ae_page).post(order_import_ae_handler))
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
        .route("/purchases/new", get(|| async { Redirect::to("/purchase/new") }))
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

        async function deleteItem(apiUrl, confirmMsg, reloadOnSuccess) {{
            if (!confirm(confirmMsg || '确认删除？此操作不可恢复。')) return;
            try {{
                const resp = await fetch(apiUrl, {{ method: 'DELETE' }});
                const data = await resp.json().catch(() => ({{}}));
                if (resp.ok) {{
                    showToast(data.message || '删除成功', 'success');
                    setTimeout(() => reloadOnSuccess ? location.reload() : location.reload(), 800);
                }} else {{
                    showToast(data.message || '删除失败', 'error');
                }}
            }} catch(e) {{
                showToast('网络错误，请重试', 'error');
            }}
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

    let order_queries = OrderQueries::new(state.db.pool());
    let customer_queries = CustomerQueries::new(state.db.pool());
    let inventory_queries = InventoryQueries::new(state.db.pool());

    // 今日日期
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let year = chrono::Local::now().year();
    let month = chrono::Local::now().month() as i32;

    // 并行查询各项数据
    let (
        pending_ship_count,
        today_followup_count,
        low_stock_count,
        this_month_stats,
    ) = tokio::join!(
        async {
            // 待发货订单（order_status=2 待发货，或 3 部分发货）
            let row: (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM orders WHERE order_status IN (2, 3)"
            )
            .fetch_one(state.db.pool())
            .await
            .unwrap_or((0,));
            row.0
        },
        customer_queries.today_followup_count(),
        async {
            let alerts = inventory_queries.get_alerts().await.unwrap_or_default();
            alerts.len() as i64
        },
        order_queries.get_analytics(year, month),
    );

    let today_followup_count = today_followup_count.unwrap_or(0);
    let this_month_stats = this_month_stats.unwrap_or_else(|_| cicierp_db::queries::orders::AnalyticsReport {
        sales_by_currency: vec![],
        top_products: vec![],
        platform_distribution: vec![],
    });

    // 本月销售额和利润
    let (month_sales_cny, month_sales_usd, month_profit_cny) = this_month_stats.sales_by_currency.iter().fold(
        (0.0f64, 0.0f64, 0.0f64),
        |(cny, usd, profit), row| {
            if row.currency == "CNY" {
                (cny + row.total_sales, usd, profit + row.total_profit)
            } else if row.currency == "USD" {
                (cny, usd + row.total_sales, profit + row.total_profit)
            } else {
                (cny, usd, profit)
            }
        }
    );

    // 今日待跟进客户列表
    let followup_customers = customer_queries.today_followup_list(5).await.unwrap_or_default();
    let followup_rows: String = followup_customers.iter().map(|c| {
        let date_str = c.next_followup_date.as_deref().unwrap_or("-");
        let is_overdue = c.next_followup_date.as_deref().map(|d| d < today.as_str()).unwrap_or(false);
        let date_color = if is_overdue { "text-red-600 font-semibold" } else { "text-orange-600" };
        format!(r#"<div class="flex items-center justify-between py-2 border-b border-gray-100 last:border-0">
            <div>
                <span class="font-medium text-gray-800">{}</span>
                <span class="text-xs text-gray-500 ml-2">{}</span>
            </div>
            <div class="flex items-center gap-3">
                <span class="text-xs {} ">{}</span>
                <a href="/customers/{}/edit" class="text-xs text-blue-600 hover:underline">跟进</a>
            </div>
        </div>"#,
            c.name,
            c.mobile.as_deref().unwrap_or(""),
            date_color,
            date_str,
            c.id,
        )
    }).collect();

    let followup_section = if followup_customers.is_empty() {
        r#"<p class="text-gray-500 text-sm text-center py-4">🎉 今日无待跟进客户</p>"#.to_string()
    } else {
        followup_rows
    };

    // 平台分布
    let platform_rows: String = this_month_stats.platform_distribution.iter().map(|p| {
        format!(r#"<div class="flex justify-between items-center py-1">
            <span class="text-sm text-gray-600">{}</span>
            <span class="text-sm font-medium">{} 单</span>
        </div>"#, p.platform, p.order_count)
    }).collect();

    let content = format!(
        r#"<!-- 欢迎区域 -->
<div class="mb-6">
    <h1 class="text-xl sm:text-2xl font-bold text-gray-800">今天要做什么？</h1>
    <p class="text-gray-500 mt-1 text-sm">{}</p>
</div>

<!-- 行动卡片 -->
<div class="grid grid-cols-2 lg:grid-cols-4 gap-3 sm:gap-4 mb-6">
    <a href="/orders?order_status=2" class="bg-white rounded-xl shadow-sm p-4 border border-gray-100 hover:border-blue-300 transition-colors">
        <div class="flex items-center justify-between mb-2">
            <span class="text-2xl">📦</span>
            <span class="text-xs text-blue-600 bg-blue-50 px-2 py-1 rounded-full">待处理</span>
        </div>
        <p class="text-2xl font-bold text-gray-800">{}</p>
        <p class="text-xs text-gray-500 mt-1">待发货订单</p>
    </a>

    <a href="/customers?lead_status=2" class="bg-white rounded-xl shadow-sm p-4 border border-gray-100 hover:border-orange-300 transition-colors">
        <div class="flex items-center justify-between mb-2">
            <span class="text-2xl">👤</span>
            <span class="text-xs text-orange-600 bg-orange-50 px-2 py-1 rounded-full">今日</span>
        </div>
        <p class="text-2xl font-bold text-gray-800">{}</p>
        <p class="text-xs text-gray-500 mt-1">待跟进客户</p>
    </a>

    <a href="/inventory?low_stock=true" class="bg-white rounded-xl shadow-sm p-4 border border-gray-100 hover:border-yellow-300 transition-colors">
        <div class="flex items-center justify-between mb-2">
            <span class="text-2xl">⚠️</span>
            <span class="text-xs text-yellow-600 bg-yellow-50 px-2 py-1 rounded-full">预警</span>
        </div>
        <p class="text-2xl font-bold text-gray-800">{}</p>
        <p class="text-xs text-gray-500 mt-1">低库存产品</p>
    </a>

    <div class="bg-white rounded-xl shadow-sm p-4 border border-gray-100">
        <div class="flex items-center justify-between mb-2">
            <span class="text-2xl">💰</span>
            <span class="text-xs text-green-600 bg-green-50 px-2 py-1 rounded-full">本月</span>
        </div>
        <p class="text-lg font-bold text-gray-800">¥{:.0}</p>
        <p class="text-xs text-gray-500 mt-1">本月利润（CNY）</p>
    </div>
</div>

<!-- 本月销售概览 + 今日待跟进 -->
<div class="grid grid-cols-1 lg:grid-cols-2 gap-4 mb-6">

    <!-- 本月销售 -->
    <div class="bg-white rounded-xl shadow-sm border border-gray-100">
        <div class="p-4 border-b border-gray-100 flex items-center justify-between">
            <h3 class="font-semibold text-gray-800">📊 本月销售概览</h3>
            <a href="/orders" class="text-xs text-blue-600 hover:underline">查看全部</a>
        </div>
        <div class="p-4 space-y-3">
            <div class="flex justify-between items-center">
                <span class="text-sm text-gray-600">销售额（CNY）</span>
                <span class="font-semibold text-gray-800">¥{:.2}</span>
            </div>
            <div class="flex justify-between items-center">
                <span class="text-sm text-gray-600">销售额（USD）</span>
                <span class="font-semibold text-gray-800">${:.2}</span>
            </div>
            <div class="flex justify-between items-center pt-2 border-t border-gray-100">
                <span class="text-sm text-gray-600">平台分布</span>
            </div>
            {}
        </div>
    </div>

    <!-- 今日待跟进 -->
    <div class="bg-white rounded-xl shadow-sm border border-gray-100">
        <div class="p-4 border-b border-gray-100 flex items-center justify-between">
            <h3 class="font-semibold text-gray-800">📞 今日待跟进客户</h3>
            <a href="/customers?lead_status=2" class="text-xs text-blue-600 hover:underline">查看全部</a>
        </div>
        <div class="p-4">
            {}
        </div>
    </div>
</div>

<!-- 快捷操作 -->
<div class="bg-white rounded-xl shadow-sm border border-gray-100">
    <div class="p-4 border-b border-gray-100">
        <h3 class="font-semibold text-gray-800">⚡ 快捷操作</h3>
    </div>
    <div class="p-4 grid grid-cols-2 sm:grid-cols-4 gap-3">
        <a href="/orders/new" class="flex flex-col items-center p-3 bg-blue-50 rounded-lg hover:bg-blue-100 transition-colors">
            <span class="text-xl mb-1">📝</span>
            <span class="text-xs text-gray-700">新建订单</span>
        </a>
        <a href="/customers/new" class="flex flex-col items-center p-3 bg-green-50 rounded-lg hover:bg-green-100 transition-colors">
            <span class="text-xl mb-1">👤</span>
            <span class="text-xs text-gray-700">新增客户</span>
        </a>
        <a href="/purchase/new" class="flex flex-col items-center p-3 bg-purple-50 rounded-lg hover:bg-purple-100 transition-colors">
            <span class="text-xl mb-1">🛒</span>
            <span class="text-xs text-gray-700">新建采购</span>
        </a>
        <a href="/inventory" class="flex flex-col items-center p-3 bg-orange-50 rounded-lg hover:bg-orange-100 transition-colors">
            <span class="text-xl mb-1">📊</span>
            <span class="text-xs text-gray-700">库存管理</span>
        </a>
    </div>
</div>"#,
        today,
        pending_ship_count,
        today_followup_count,
        low_stock_count,
        month_profit_cny,
        month_sales_cny,
        month_sales_usd,
        if platform_rows.is_empty() { "<p class='text-xs text-gray-400'>暂无数据</p>".to_string() } else { platform_rows },
        followup_section,
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
    #[serde(default, deserialize_with = "empty_string_as_none_i64")]
    supplier_id: Option<i64>,
    #[serde(default, deserialize_with = "empty_string_as_none_f64")]
    price_min: Option<f64>,
    #[serde(default, deserialize_with = "empty_string_as_none_f64")]
    price_max: Option<f64>,
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

    // Fetch suppliers for filter dropdown
    let supplier_queries = SupplierQueries::new(state.db.pool());
    let all_suppliers = supplier_queries
        .list(1, 1000, Some(1), None, None)
        .await
        .map(|r| r.items)
        .unwrap_or_default();

    let queries = ProductQueries::new(state.db.pool());
    let result = queries
        .list(
            page,
            page_size,
            None,
            None,
            None,
            query.keyword.as_deref(),
            query.supplier_id,
            query.price_min,
            query.price_max,
        )
        .await
        .unwrap_or_else(|_| PagedResponse::new(vec![], page, page_size, 0));

    let stats = queries.dashboard_stats(
        None,
        query.supplier_id,
        query.keyword.as_deref(),
    ).await.unwrap_or_default();

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

        // 利润显示
        let profit_usd_display = p.profit_usd.map(|v| {
            let color = if v >= 0.0 { "text-green-600" } else { "text-red-600" };
            format!(r#"<span class="{}">${:.2}</span>"#, color, v)
        }).unwrap_or_else(|| "-".to_string());

        let profit_margin_display = p.profit_margin.map(|v| {
            let color = if v >= 0.0 { "text-green-600" } else { "text-red-600" };
            format!(r#"<span class="{}">{:.1}%</span>"#, color, v)
        }).unwrap_or_else(|| "-".to_string());

        let model_display = p.model.as_deref().unwrap_or("-");
        let supplier_display = p.supplier_name.as_deref().unwrap_or("-");
        let platform_fees_display = p.platform_fees.as_deref().unwrap_or("-");

        rows.push_str(&format!(
            r#"<tr class="hover:bg-gray-50 transition-colors">
                <td class="px-2 py-3"><a href="/products/{}" class="font-mono text-xs text-blue-600 hover:text-blue-800 hover:underline">{}</a></td>
                <td class="px-2 py-3"><span class="font-medium text-gray-800 text-sm">{}</span></td>
                <td class="px-2 py-3"><span class="text-xs text-gray-600">{}</span></td>
                <td class="px-2 py-3"><span class="text-xs text-gray-500">{}</span></td>
                <td class="px-2 py-3 text-right"><span class="text-xs text-gray-600">{}</span></td>
                <td class="px-2 py-3 text-right"><span class="text-xs text-gray-600">{}</span></td>
                <td class="px-2 py-3 text-right"><span class="text-xs font-medium text-gray-800">{}</span></td>
                <td class="px-2 py-3 text-right"><span class="text-xs">{}</span></td>
                <td class="px-2 py-3 text-right"><span class="text-xs">{}</span></td>
                <td class="px-2 py-3"><span class="text-xs text-gray-500 whitespace-nowrap">{}</span></td>
                <td class="px-2 py-3 text-right"><span class="{} text-xs">{}</span></td>
                <td class="px-2 py-3 text-center">{}</td>
                <td class="px-2 py-3 text-center">
                    <div class="flex items-center justify-center gap-1">
                        <a href="/products/{}/edit" class="px-2 py-1 text-xs text-green-600 hover:text-green-800 hover:bg-green-50 rounded">编辑</a>
                        <button onclick="deleteItem('/api/v1/products/{}', '确认删除产品「{}」？', true)" class="px-2 py-1 text-xs text-red-600 hover:text-red-800 hover:bg-red-50 rounded">删除</button>
                    </div>
                </td>
            </tr>"#,
            p.id,
            p.product_code,
            p.name,
            model_display,
            supplier_display,
            cost_cny_display,
            cost_usd_display,
            price_usd_display,
            profit_usd_display,
            profit_margin_display,
            platform_fees_display,
            stock_class,
            p.stock_quantity.unwrap_or(0),
            status_badge,
            p.id,
            p.id,
            p.name
        ));
    }

    if rows.is_empty() {
        rows = r#"<tr><td colspan="13" class="px-6 py-12 text-center"><div class="text-gray-500"><p class="text-4xl mb-2">📦</p><p>暂无产品数据</p></div></td></tr>"#.to_string();
    }

    let pagination = if total_pages > 1 {
        // Build base query string preserving filters
        let base_params = {
            let mut p = Vec::new();
            if let Some(ref k) = query.keyword { p.push(format!("keyword={}", k)); }
            if let Some(sid) = query.supplier_id { p.push(format!("supplier_id={}", sid)); }
            if let Some(v) = query.price_min { p.push(format!("price_min={}", v)); }
            if let Some(v) = query.price_max { p.push(format!("price_max={}", v)); }
            p.join("&")
        };
        let sep = if base_params.is_empty() { "" } else { "&" };
        // 上一页按钮
        let prev_btn = if page > 1 {
            format!(r#"<a href="/products?page={}{}{}" class="px-4 py-2 text-sm text-gray-600 hover:bg-gray-100 rounded-lg border border-gray-300">上一页</a>"#, page - 1, sep, base_params)
        } else {
            r#"<span class="px-4 py-2 text-sm text-gray-400 rounded-lg border border-gray-200">上一页</span>"#.to_string()
        };
        // 下一页按钮
        let next_btn = if page < total_pages {
            format!(r#"<a href="/products?page={}{}{}" class="px-4 py-2 text-sm text-gray-600 hover:bg-gray-100 rounded-lg border border-gray-300">下一页</a>"#, page + 1, sep, base_params)
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

    // Build supplier filter options
    let supplier_options: String = all_suppliers.iter().map(|s| {
        let selected = query.supplier_id.map(|id| id == s.id).unwrap_or(false);
        format!(
            r#"<option value="{}" {}>{}</option>"#,
            s.id,
            if selected { "selected" } else { "" },
            s.name
        )
    }).collect();

    let keyword_val = query.keyword.as_deref().unwrap_or("");
    let price_min_val = query.price_min.map(|v| format!("{}", v)).unwrap_or_default();
    let price_max_val = query.price_max.map(|v| format!("{}", v)).unwrap_or_default();

    let content = format!(
        r#"<!-- 页面标题 -->
<div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4 mb-6">
    <div>
        <h1 class="text-xl sm:text-2xl font-bold text-gray-800">产品管理</h1>
        <p class="text-gray-600 mt-1 text-sm sm:text-base">管理所有产品信息</p>
    </div>
    <div class="flex flex-wrap items-center gap-2">
        <a href="/products/export" class="inline-flex items-center gap-1.5 px-3 py-2 bg-green-600 text-white text-sm rounded-lg hover:bg-green-700 transition-colors">
            ↓ 导出 Excel
        </a>
        <a href="/products/import/template" class="inline-flex items-center gap-1.5 px-3 py-2 bg-gray-100 text-gray-700 text-sm rounded-lg hover:bg-gray-200 transition-colors">
            ↓ 通用模板
        </a>
        <button onclick="document.getElementById('importModal').classList.remove('hidden')"
                class="inline-flex items-center gap-1.5 px-3 py-2 bg-amber-500 text-white text-sm rounded-lg hover:bg-amber-600 transition-colors">
            ↑ 通用导入
        </button>
        <a href="/products/new" class="inline-flex items-center justify-center gap-2 px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors">
            <span>+</span><span>新增产品</span>
        </a>
    </div>
</div>

<!-- 通用导入弹窗 -->
<div id="importModal" class="hidden fixed inset-0 z-50 flex items-center justify-center bg-black/50">
    <div class="bg-white rounded-xl shadow-xl w-full max-w-md mx-4 p-6">
        <div class="flex items-center justify-between mb-4">
            <h3 class="text-lg font-semibold text-gray-800">批量导入产品（通用）</h3>
            <button onclick="document.getElementById('importModal').classList.add('hidden')"
                    class="text-gray-400 hover:text-gray-600 text-2xl leading-none">&times;</button>
        </div>
        <div class="text-sm text-gray-600 mb-4 space-y-1">
            <p>列顺序：产品名称* / 英文名称 / 供应商名称 / 成本(CNY)* / 售价(CNY) / 售价(USD) / 状态 / 分类 / 品牌 / 备注</p>
            <p><a href="/products/import/template" class="text-blue-600 hover:underline">↓ 下载通用模板</a></p>
        </div>
        <form action="/products/import" method="POST" enctype="multipart/form-data">
            <div class="mb-4">
                <label class="block text-sm font-medium text-gray-700 mb-2">选择文件</label>
                <input type="file" name="file" accept=".xlsx"
                       class="block w-full text-sm text-gray-700 border border-gray-300 rounded-lg px-3 py-2 cursor-pointer" required>
            </div>
            <div class="flex justify-end gap-3">
                <button type="button" onclick="document.getElementById('importModal').classList.add('hidden')"
                        class="px-4 py-2 bg-gray-100 text-gray-700 rounded-lg hover:bg-gray-200">取消</button>
                <button type="submit" class="px-4 py-2 bg-amber-500 text-white rounded-lg hover:bg-amber-600">开始导入</button>
            </div>
        </form>
    </div>
</div>

<!-- 筛选栏 -->
<div class="bg-white rounded-xl shadow-sm border border-gray-100 p-4 mb-6">
    <form action="/products" method="GET" class="flex flex-col gap-3">
        <div class="flex flex-col sm:flex-row gap-3">
            <div class="flex-1">
                <input type="text" name="keyword" value="{}" placeholder="搜索产品编码、名称、型号..."
                       class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent text-sm">
            </div>
            <div class="sm:w-48">
                <select name="supplier_id" class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent text-sm">
                    <option value="">全部供应商</option>
                    {}
                </select>
            </div>
            <div class="flex gap-2 items-center">
                <input type="number" name="price_min" value="{}" placeholder="最低售价($)" step="0.01" min="0"
                       class="w-28 px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent text-sm">
                <span class="text-gray-400 text-sm">—</span>
                <input type="number" name="price_max" value="{}" placeholder="最高售价($)" step="0.01" min="0"
                       class="w-28 px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent text-sm">
            </div>
            <div class="flex gap-2">
                <button type="submit" class="px-5 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors text-sm">筛选</button>
                <a href="/products" class="px-5 py-2 bg-gray-100 text-gray-700 rounded-lg hover:bg-gray-200 transition-colors text-sm">重置</a>
            </div>
        </div>
    </form>
</div>

<!-- 看板统计 -->
<div style="display:flex;gap:16px;margin-bottom:20px;">
  <div style="flex:1;background:#f0f9ff;border:1px solid #bae6fd;border-radius:8px;padding:16px;text-align:center;">
    <div style="font-size:24px;font-weight:700;color:#0284c7;">{}</div>
    <div style="font-size:12px;color:#64748b;margin-top:4px;">商品数量</div>
  </div>
  <div style="flex:1;background:#f0fdf4;border:1px solid #bbf7d0;border-radius:8px;padding:16px;text-align:center;">
    <div style="font-size:24px;font-weight:700;color:#16a34a;">{}</div>
    <div style="font-size:12px;color:#64748b;margin-top:4px;">库存总数</div>
  </div>
  <div style="flex:1;background:#fefce8;border:1px solid #fde68a;border-radius:8px;padding:16px;text-align:center;">
    <div style="font-size:24px;font-weight:700;color:#ca8a04;">${:.2}</div>
    <div style="font-size:12px;color:#64748b;margin-top:4px;">平均销售价 (USD)</div>
  </div>
  <div style="flex:1;background:#fdf4ff;border:1px solid #e9d5ff;border-radius:8px;padding:16px;text-align:center;">
    <div style="font-size:24px;font-weight:700;color:#9333ea;">${:.0}</div>
    <div style="font-size:12px;color:#64748b;margin-top:4px;">理论库存价值 (USD)</div>
  </div>
</div>

<!-- 产品表格 -->
<div class="bg-white rounded-xl shadow-sm border border-gray-100 overflow-hidden">
    <div class="overflow-x-auto">
        <table class="w-full min-w-[1200px]">
            <thead class="bg-gray-50 border-b border-gray-200">
                <tr>
                    <th class="px-2 py-3 text-left text-xs font-semibold text-gray-700">产品编码</th>
                    <th class="px-2 py-3 text-left text-xs font-semibold text-gray-700">产品名称</th>
                    <th class="px-2 py-3 text-left text-xs font-semibold text-gray-700">型号</th>
                    <th class="px-2 py-3 text-left text-xs font-semibold text-gray-700">供应商</th>
                    <th class="px-2 py-3 text-right text-xs font-semibold text-gray-700">RMB成本</th>
                    <th class="px-2 py-3 text-right text-xs font-semibold text-gray-700">美金成本</th>
                    <th class="px-2 py-3 text-right text-xs font-semibold text-gray-700">美金卖价</th>
                    <th class="px-2 py-3 text-right text-xs font-semibold text-gray-700">利润($)(网站)</th>
                    <th class="px-2 py-3 text-right text-xs font-semibold text-gray-700">利润率</th>
                    <th class="px-2 py-3 text-left text-xs font-semibold text-gray-700">平台费率</th>
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
        keyword_val,
        supplier_options,
        price_min_val,
        price_max_val,
        stats.total_count,
        stats.total_stock as i64,
        stats.avg_price_usd,
        stats.total_stock_value,
        rows,
        pagination
    );

    render_layout("产品管理", "products", Some(user), &content)
}

/// 新增产品页面
pub async fn product_new_page(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
) -> Html<String> {
    let user = get_user_from_extension(&auth_user);

    // 获取缓冲汇率：市场汇率 - 0.05，保留两位小数
    let buffered_rate = {
        let queries = ExchangeRateQueries::new(state.db.pool());
        let rate = queries.get_latest_rate("USD", "CNY").await
            .ok().flatten()
            .map(|r| r.rate)
            .unwrap_or(7.2);
        format!("{:.2}", (rate - 0.05).max(0.0))
    };

    let content = format!(r#"<!-- 页面标题 -->
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

                <!-- 型号 -->
                <div>
                    <label for="model" class="block text-sm font-medium text-gray-700 mb-2">
                        型号
                    </label>
                    <input type="text" id="model" name="model"
                           class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                           placeholder="对外展示的型号，如 V3.0 / Pro Max">
                    <p class="text-xs text-gray-400 mt-1">对外展示字段，用于报价单和产品资料</p>
                </div>

                <!-- 单位 -->
                <div>
                    <label for="unit" class="block text-sm font-medium text-gray-700 mb-2">单位</label>
                    <select id="unit" name="unit" class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent">
                        <option value="pcs" selected>件 (pcs)</option>
                        <option value="set">套 (set)</option>
                        <option value="pair">对 (pair)</option>
                        <option value="box">箱 (box)</option>
                        <option value="m">米 (m)</option>
                        <option value="kg">千克 (kg)</option>
                    </select>
                </div>

                <!-- 分类 -->
                <div>
                    <label for="category_name_input" class="block text-sm font-medium text-gray-700 mb-2">
                        分类
                    </label>
                    <div class="relative">
                        <input type="text" id="category_name_input" autocomplete="off"
                               class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                               placeholder="输入分类名称搜索或新增">
                        <input type="hidden" id="category_id" name="category_id" value="">
                        <input type="hidden" id="category_name" name="category_name" value="">
                        <div id="category_dropdown" class="hidden absolute z-10 w-full bg-white border border-gray-200 rounded-lg shadow-lg mt-1 max-h-48 overflow-y-auto"></div>
                        <p class="text-xs text-gray-400 mt-1">输入已有分类可搜索选择；输入新名称保存时自动新增</p>
                    </div>
                </div>

                <!-- 品牌 -->
                <div>
                    <label for="brand_name_input" class="block text-sm font-medium text-gray-700 mb-2">
                        品牌
                    </label>
                    <div class="relative">
                        <input type="text" id="brand_name_input" autocomplete="off"
                               class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                               placeholder="输入品牌名称搜索或新增">
                        <input type="hidden" id="brand_id" name="brand_id" value="">
                        <input type="hidden" id="brand_name" name="brand_name" value="">
                        <div id="brand_dropdown" class="hidden absolute z-10 w-full bg-white border border-gray-200 rounded-lg shadow-lg mt-1 max-h-48 overflow-y-auto"></div>
                        <p class="text-xs text-gray-400 mt-1">输入已有品牌可搜索选择；输入新名称保存时自动新增</p>
                    </div>
                </div>

                <!-- 供应商 -->
                <div>
                    <label for="supplier_name_input" class="block text-sm font-medium text-gray-700 mb-2">
                        供应商
                    </label>
                    <div class="relative">
                        <input type="text" id="supplier_name_input" autocomplete="off"
                               class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                               placeholder="输入供应商名称搜索">
                        <input type="hidden" id="supplier_id" name="supplier_id" value="">
                        <input type="hidden" id="supplier_name" name="supplier_name" value="">
                        <div id="supplier_dropdown" class="hidden absolute z-10 w-full bg-white border border-gray-200 rounded-lg shadow-lg mt-1 max-h-48 overflow-y-auto"></div>
                        <p class="text-xs text-gray-400 mt-1">输入供应商名称搜索选择</p>
                    </div>
                </div>

                <!-- 重量 -->
                <div>
                    <label for="weight" class="block text-sm font-medium text-gray-700 mb-2">
                        重量 (kg) <span class="text-xs text-gray-400">(仅记录)</span>
                    </label>
                    <input type="number" id="weight" name="weight" step="0.001" min="0"
                           class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                           placeholder="0.000">
                </div>

                <!-- 体积 -->
                <div>
                    <label for="volume" class="block text-sm font-medium text-gray-700 mb-2">
                        体积 (m³) <span class="text-xs text-gray-400">(仅记录)</span>
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
                        <option value="3" selected>草稿</option>
                        <option value="1">上架 (需已设置售价)</option>
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

                <!-- 成本 USD（只读，自动由CNY/当前汇率计算） -->
                <div>
                    <label for="cost_usd" class="block text-sm font-medium text-gray-700 mb-2">
                        成本 (USD) <span class="text-xs text-gray-400">(自动)</span>
                    </label>
                    <input type="number" id="cost_usd" name="cost_usd" step="0.01" min="0"
                           class="w-full px-4 py-2 border border-gray-200 rounded-lg bg-gray-50 text-gray-500 cursor-not-allowed"
                           placeholder="0.00" readonly>
                    <p class="text-xs text-gray-400 mt-1" id="cost_formula_hint">公式: 成本(CNY) ÷ 汇率 = 成本(USD)</p>
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

        <!-- 第三部分：定价（三平台独立） -->
        <div class="mb-8">
            <h3 class="text-lg font-semibold text-gray-800 mb-4 pb-2 border-b border-gray-200">
                🏷️ 平台定价 <span class="text-sm font-normal text-gray-500">(按平台分别设置)</span>
            </h3>

            <!-- 全局汇率 -->
            <div class="mb-4 p-3 bg-blue-50 rounded-lg flex items-center gap-4">
                <label class="text-sm font-medium text-blue-800 whitespace-nowrap">💱 参考汇率 (USD/CNY)</label>
                <input type="number" id="price_exchange_rate" name="price_exchange_rate" step="0.01" min="0"
                       value="{buffered_rate}"
                       class="w-32 px-3 py-1.5 border border-blue-200 rounded-lg text-sm focus:ring-2 focus:ring-blue-500">
                <span class="text-xs text-blue-600">成本和售价的CNY/USD换算共用此汇率</span>
            </div>

            <!-- AliExpress 区块 -->
            <div class="mb-4 p-4 border border-orange-200 rounded-xl bg-orange-50">
                <div class="flex items-center gap-2 mb-3">
                    <span class="text-sm font-semibold text-orange-700">🛒 AliExpress</span>
                    <span class="text-xs text-orange-500 bg-orange-100 px-2 py-0.5 rounded-full">固定利润率 40%</span>
                </div>
                <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
                    <div>
                        <label class="block text-xs font-medium text-gray-600 mb-1">平台费率 (%)</label>
                        <input type="number" id="ae_fee_rate" name="ae_fee_rate" step="0.01" min="0" value="12.0"
                               class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:ring-2 focus:ring-orange-500"
                               placeholder="5.0">
                    </div>
                    <div>
                        <label class="block text-xs font-medium text-gray-600 mb-1">售价 CNY <span class="text-xs text-gray-400">(自动)</span></label>
                        <input type="number" id="ae_sale_price_cny" step="0.01" min="0" value=""
                               class="w-full px-3 py-2 border border-orange-200 rounded-lg text-sm bg-white text-gray-700 cursor-not-allowed"
                               readonly placeholder="0.00">
                    </div>
                    <div class="flex items-end">
                        <div id="ae_profit_preview" class="w-full p-2 rounded-lg bg-white border border-orange-200 text-xs text-gray-600">
                            <div>利润: <span id="ae_profit_val" class="font-medium">-</span></div>
                            <div>利润率: <span id="ae_profit_pct" class="font-medium">-</span></div>
                        </div>
                    </div>
                </div>
            </div>

            <!-- Alibaba 区块 -->
            <div class="mb-4 p-4 border border-yellow-200 rounded-xl bg-yellow-50">
                <div class="flex items-center gap-2 mb-3">
                    <span class="text-sm font-semibold text-yellow-700">🏪 Alibaba</span>
                    <span class="text-xs text-yellow-600 bg-yellow-100 px-2 py-0.5 rounded-full">固定利润率 15%</span>
                </div>
                <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
                    <div>
                        <label class="block text-xs font-medium text-gray-600 mb-1">平台费率 (%)</label>
                        <input type="number" id="ali_fee_rate" name="ali_fee_rate" step="0.01" min="0" value="2.5"
                               class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:ring-2 focus:ring-yellow-500"
                               placeholder="3.0">
                    </div>
                    <div>
                        <label class="block text-xs font-medium text-gray-600 mb-1">售价 USD <span class="text-xs text-gray-400">(自动)</span></label>
                        <input type="number" id="ali_sale_price_usd" step="0.01" min="0" value=""
                               class="w-full px-3 py-2 border border-yellow-200 rounded-lg text-sm bg-white text-gray-700 cursor-not-allowed"
                               readonly placeholder="0.00">
                    </div>
                    <div class="flex items-end">
                        <div id="ali_profit_preview" class="w-full p-2 rounded-lg bg-white border border-yellow-200 text-xs text-gray-600">
                            <div>利润: <span id="ali_profit_val" class="font-medium">-</span></div>
                            <div>利润率: <span id="ali_profit_pct" class="font-medium">-</span></div>
                        </div>
                    </div>
                </div>
            </div>

            <!-- Website 区块 -->
            <div class="p-4 border border-blue-200 rounded-xl bg-blue-50">
                <div class="flex items-center gap-2 mb-3">
                    <span class="text-sm font-semibold text-blue-700">🌐 Website</span>
                    <span class="text-xs text-blue-500 bg-blue-100 px-2 py-0.5 rounded-full">固定利润率 40%</span>
                </div>
                <div class="grid grid-cols-1 md:grid-cols-4 gap-3">
                    <div>
                        <label class="block text-xs font-medium text-gray-600 mb-1">平台费率 (%)</label>
                        <input type="number" id="web_fee_rate" name="web_fee_rate" step="0.01" min="0" value="0.0"
                               class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:ring-2 focus:ring-blue-500"
                               placeholder="2.5">
                    </div>
                    <div>
                        <label class="block text-xs font-medium text-gray-600 mb-1">售价 CNY <span class="text-xs text-gray-400">(自动)</span></label>
                        <input type="number" id="web_sale_price_cny" step="0.01" min="0" value=""
                               class="w-full px-3 py-2 border border-blue-200 rounded-lg text-sm bg-white text-gray-700 cursor-not-allowed"
                               readonly placeholder="0.00">
                    </div>
                    <div>
                        <label class="block text-xs font-medium text-gray-600 mb-1">售价 USD <span class="text-xs text-gray-400">(自动)</span></label>
                        <input type="number" id="web_sale_price_usd" step="0.01" min="0"
                               class="w-full px-3 py-2 border border-blue-200 rounded-lg text-sm bg-white text-gray-700 cursor-not-allowed"
                               readonly placeholder="0.00">
                    </div>
                    <div class="flex items-end">
                        <div id="web_profit_preview" class="w-full p-2 rounded-lg bg-white border border-blue-200 text-xs text-gray-600">
                            <div>利润: <span id="web_profit_val" class="font-medium">-</span></div>
                            <div>利润率: <span id="web_profit_pct" class="font-medium">-</span></div>
                        </div>
                    </div>
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
</div>
<script>
(function() {{
    function calcCostUsd() {{
        const cny  = parseFloat(document.getElementById('cost_cny')?.value) || 0;
        const rate = parseFloat(document.getElementById('price_exchange_rate')?.value) || {buffered_rate};
        const costUsdEl = document.getElementById('cost_usd');
        if (cny > 0 && costUsdEl) {{
            costUsdEl.value = (cny / rate).toFixed(2);
            const hint = document.getElementById('cost_formula_hint');
            if (hint) hint.textContent = `¥${{cny.toFixed(2)}} ÷ ${{rate.toFixed(2)}} = $${{(cny/rate).toFixed(2)}}`;
        }} else if (costUsdEl) {{
            costUsdEl.value = '';
        }}
    }}

    const aeMargin = 0.40;
    const aliMargin = 0.15;
    const webMargin = 0.40;
    const rate = () => parseFloat(document.getElementById('price_exchange_rate')?.value) || {buffered_rate};
    const costCnyVal = () => parseFloat(document.getElementById('cost_cny')?.value) || 0;
    const costUsdVal = () => costCnyVal() / rate();
    function calcSalePrice(cost, feeRate, margin) {{
        const denominator = 1 - feeRate - margin;
        if (cost <= 0 || denominator <= 0) return 0;
        return cost / denominator;
    }}

    function calcAEPrices() {{
        const fee = (parseFloat(document.getElementById('ae_fee_rate')?.value) || 12) / 100;
        const cny = calcSalePrice(costCnyVal(), fee, aeMargin);
        const priceEl = document.getElementById('ae_sale_price_cny');
        if (priceEl) priceEl.value = cny > 0 ? cny.toFixed(2) : '';
        updateAEProfit();
    }}

    function updateAEProfit() {{
        const cny = parseFloat(document.getElementById('ae_sale_price_cny')?.value) || 0;
        const fee = (parseFloat(document.getElementById('ae_fee_rate')?.value) || 12) / 100;
        const cost = costCnyVal();
        if (cny > 0 && cost > 0) {{
            const profit = cny - cost - cny * fee;
            const pct = cny > 0 ? (profit / cny * 100).toFixed(1) : 0;
            document.getElementById('ae_profit_val').textContent = profit.toFixed(2) + ' CNY';
            document.getElementById('ae_profit_pct').textContent = pct + '%';
            document.getElementById('ae_profit_preview').className =
                'w-full p-2 rounded-lg border text-xs ' +
                (profit > 0 ? 'bg-green-50 border-green-200 text-green-700' : 'bg-red-50 border-red-200 text-red-700');
        }}
    }}

    function calcALIPrices() {{
        const fee = (parseFloat(document.getElementById('ali_fee_rate')?.value) || 2.5) / 100;
        const usd = calcSalePrice(costUsdVal(), fee, aliMargin);
        const priceEl = document.getElementById('ali_sale_price_usd');
        if (priceEl) priceEl.value = usd > 0 ? usd.toFixed(2) : '';
        updateALIProfit();
    }}

    function updateALIProfit() {{
        const usd = parseFloat(document.getElementById('ali_sale_price_usd')?.value) || 0;
        const fee = (parseFloat(document.getElementById('ali_fee_rate')?.value) || 2.5) / 100;
        const cUsd = costUsdVal();
        if (usd > 0 && cUsd > 0) {{
            const profit = usd - cUsd - usd * fee;
            const pct = usd > 0 ? (profit / usd * 100).toFixed(1) : 0;
            document.getElementById('ali_profit_val').textContent = '$' + profit.toFixed(2);
            document.getElementById('ali_profit_pct').textContent = pct + '%';
            document.getElementById('ali_profit_preview').className =
                'w-full p-2 rounded-lg border text-xs ' +
                (profit > 0 ? 'bg-green-50 border-green-200 text-green-700' : 'bg-red-50 border-red-200 text-red-700');
        }}
    }}

    function calcWebPrices() {{
        const fee = (parseFloat(document.getElementById('web_fee_rate')?.value) || 0) / 100;
        const cny = calcSalePrice(costCnyVal(), fee, webMargin);
        const cnyEl = document.getElementById('web_sale_price_cny');
        const usdEl = document.getElementById('web_sale_price_usd');
        if (cnyEl) cnyEl.value = cny > 0 ? cny.toFixed(2) : '';
        if (usdEl && cny > 0) usdEl.value = (cny / rate()).toFixed(2);
        else if (usdEl) usdEl.value = '';
        updateWebProfit();
    }}

    function updateWebProfit() {{
        const priceCny = parseFloat(document.getElementById('web_sale_price_cny')?.value) || 0;
        const fee = (parseFloat(document.getElementById('web_fee_rate')?.value) || 0) / 100;
        const cost = costCnyVal();
        if (priceCny > 0 && cost > 0) {{
            const profit = priceCny - cost - priceCny * fee;
            const pct = priceCny > 0 ? (profit / priceCny * 100).toFixed(1) : 0;
            document.getElementById('web_profit_val').textContent = profit.toFixed(2) + ' CNY';
            document.getElementById('web_profit_pct').textContent = pct + '%';
            document.getElementById('web_profit_preview').className =
                'w-full p-2 rounded-lg border text-xs ' +
                (profit > 0 ? 'bg-green-50 border-green-200 text-green-700' : 'bg-red-50 border-red-200 text-red-700');
        }}
    }}

    document.getElementById('ae_sale_price_cny')?.addEventListener('input', calcAEPrices);
    document.getElementById('ae_fee_rate')?.addEventListener('input', calcAEPrices);
    document.getElementById('ali_sale_price_usd')?.addEventListener('input', calcALIPrices);
    document.getElementById('ali_fee_rate')?.addEventListener('input', calcALIPrices);
    document.getElementById('web_fee_rate')?.addEventListener('input', calcWebPrices);
    document.getElementById('price_exchange_rate')?.addEventListener('input', function() {{
        calcCostUsd();
        calcAEPrices();
        calcALIPrices();
        calcWebPrices();
    }});
    document.getElementById('cost_cny')?.addEventListener('input', function() {{
        calcCostUsd();
        calcAEPrices();
        calcALIPrices();
        calcWebPrices();
    }});

    calcAEPrices();
    calcALIPrices();
    calcWebPrices();

    document.getElementById('status')?.addEventListener('change', function() {{
        if (this.value === '1') {{
            const aePrice = parseFloat(document.getElementById('ae_sale_price_cny')?.value) || 0;
            const aliPrice = parseFloat(document.getElementById('ali_sale_price_usd')?.value) || 0;
            const webPrice = parseFloat(document.getElementById('web_sale_price_cny')?.value) || 0;
            if (aePrice <= 0 && aliPrice <= 0 && webPrice <= 0) {{
                alert('⚠️ 请先设置售价，再将产品设为上架状态。');
                this.value = '3';
            }}
        }}
    }});

    document.querySelector('form').addEventListener('submit', function(e) {{
        const costCnyInput = parseFloat(document.getElementById('cost_cny')?.value);
        const rateVal = parseFloat(document.getElementById('price_exchange_rate')?.value);
        if (costCnyInput !== undefined && !isNaN(costCnyInput) && costCnyInput < 0) {{
            e.preventDefault(); alert('成本不能为负数'); return;
        }}
        if (rateVal !== undefined && !isNaN(rateVal) && rateVal <= 0) {{
            e.preventDefault(); alert('汇率必须大于0'); return;
        }}
        const aePrice = parseFloat(document.getElementById('ae_sale_price_cny')?.value) || 0;
        const aliPrice = parseFloat(document.getElementById('ali_sale_price_usd')?.value) || 0;
        const webPrice = parseFloat(document.getElementById('web_sale_price_cny')?.value) || 0;
        if (aePrice <= 0 && aliPrice <= 0 && webPrice <= 0) {{
            if (!confirm('未设置任何平台售价，确定要保存吗？保存后产品将标记为"草稿"状态。')) {{
                e.preventDefault(); return;
            }}
        }}
    }});

    // 品牌 Combobox
    const brandInput    = document.getElementById('brand_name_input');
    const brandIdInput  = document.getElementById('brand_id');
    const brandNameHid  = document.getElementById('brand_name');
    const brandDropdown = document.getElementById('brand_dropdown');
    let brandTimer = null;

    function renderBrandDropdown(items, keyword) {{
        brandDropdown.innerHTML = '';
        if (items.length > 0) {{
            items.forEach(function(b) {{
                const div = document.createElement('div');
                div.className = 'px-4 py-2 cursor-pointer hover:bg-blue-50 text-sm';
                div.textContent = b.name;
                div.addEventListener('mousedown', function(e) {{
                    e.preventDefault();
                    brandInput.value = b.name;
                    brandIdInput.value = b.id;
                    brandNameHid.value = '';
                    brandDropdown.classList.add('hidden');
                }});
                brandDropdown.appendChild(div);
            }});
        }}
        const tip = document.createElement('div');
        tip.className = 'px-4 py-2 text-xs text-gray-400 border-t border-gray-100';
        tip.textContent = items.length > 0
            ? '↑ 选择已有品牌，或直接保存以自动新增「' + keyword + '」'
            : '未找到匹配品牌，保存时将自动新增「' + keyword + '」';
        brandDropdown.appendChild(tip);
        brandDropdown.classList.remove('hidden');
    }}

    if (brandInput) {{
        brandInput.addEventListener('input', function() {{
            const kw = this.value.trim();
            brandIdInput.value = '';
            brandNameHid.value = kw;
            clearTimeout(brandTimer);
            if (!kw) {{ brandDropdown.classList.add('hidden'); return; }}
            brandTimer = setTimeout(function() {{
                fetch('/api/v1/brands?q=' + encodeURIComponent(kw) + '&limit=10')
                    .then(r => r.json())
                    .then(function(d) {{ renderBrandDropdown(d.data || [], kw); }});
            }}, 200);
        }});
        brandInput.addEventListener('blur', function() {{
            setTimeout(function() {{ brandDropdown.classList.add('hidden'); }}, 150);
        }});
    }}

    // 分类 combobox
    const catInput    = document.getElementById('category_name_input');
    const catIdInput  = document.getElementById('category_id');
    const catNameHid  = document.getElementById('category_name');
    const catDropdown = document.getElementById('category_dropdown');
    let catTimer = null;

    function renderCatDropdown(items, keyword) {{
        catDropdown.innerHTML = '';
        if (items.length > 0) {{
            items.forEach(function(c) {{
                const div = document.createElement('div');
                div.className = 'px-4 py-2 cursor-pointer hover:bg-blue-50 text-sm';
                div.textContent = c.name;
                div.addEventListener('mousedown', function(e) {{
                    e.preventDefault();
                    catInput.value = c.name;
                    catIdInput.value = c.id;
                    catNameHid.value = '';
                    catDropdown.classList.add('hidden');
                }});
                catDropdown.appendChild(div);
            }});
        }}
        const tip = document.createElement('div');
        tip.className = 'px-4 py-2 text-xs text-gray-400 border-t border-gray-100';
        tip.textContent = items.length > 0
            ? '↑ 选择已有分类，或直接保存以自动新增「' + keyword + '」'
            : '未找到匹配分类，保存时将自动新增「' + keyword + '」';
        catDropdown.appendChild(tip);
        catDropdown.classList.remove('hidden');
    }}

    if (catInput) {{
        catInput.addEventListener('input', function() {{
            const kw = this.value.trim();
            catIdInput.value = '';
            catNameHid.value = kw;
            clearTimeout(catTimer);
            if (!kw) {{ catDropdown.classList.add('hidden'); return; }}
            catTimer = setTimeout(function() {{
                fetch('/api/v1/categories?q=' + encodeURIComponent(kw) + '&limit=10')
                    .then(r => r.json())
                    .then(function(d) {{ renderCatDropdown(d.data || [], kw); }});
            }}, 200);
        }});
        catInput.addEventListener('blur', function() {{
            setTimeout(function() {{ catDropdown.classList.add('hidden'); }}, 150);
        }});
    }}

    // 供应商 combobox
    const supInput    = document.getElementById('supplier_name_input');
    const supIdInput  = document.getElementById('supplier_id');
    const supDropdown = document.getElementById('supplier_dropdown');
    let supTimer = null;

    function renderSupDropdown(items) {{
        supDropdown.innerHTML = '';
        if (items.length === 0) {{
            const tip = document.createElement('div');
            tip.className = 'px-4 py-2 text-xs text-gray-400';
            tip.textContent = '未找到匹配供应商';
            supDropdown.appendChild(tip);
        }} else {{
            items.forEach(function(s) {{
                const div = document.createElement('div');
                div.className = 'px-4 py-2 cursor-pointer hover:bg-blue-50 text-sm';
                div.textContent = s.name;
                div.addEventListener('mousedown', function(e) {{
                    e.preventDefault();
                    supInput.value = s.name;
                    supIdInput.value = s.id;
                    supDropdown.classList.add('hidden');
                }});
                supDropdown.appendChild(div);
            }});
        }}
        supDropdown.classList.remove('hidden');
    }}

    if (supInput) {{
        supInput.addEventListener('input', function() {{
            const kw = this.value.trim();
            supIdInput.value = '';
            clearTimeout(supTimer);
            if (!kw) {{ supDropdown.classList.add('hidden'); return; }}
            supTimer = setTimeout(function() {{
                fetch('/api/v1/suppliers?keyword=' + encodeURIComponent(kw) + '&page_size=10')
                    .then(r => r.json())
                    .then(function(d) {{
                        const items = (d.data && d.data.items) ? d.data.items : [];
                        renderSupDropdown(items);
                    }});
            }}, 200);
        }});
        supInput.addEventListener('blur', function() {{
            setTimeout(function() {{ supDropdown.classList.add('hidden'); }}, 150);
        }});
    }}
}})();
</script>"#);

    render_layout("新增产品", "products", Some(user), &content)
}

/// 创建产品表单数据
#[derive(Debug, Deserialize)]
pub struct ProductForm {
    // 产品基本信息（product_code 由系统自动生成）
    name: String,
    name_en: Option<String>,
    model: Option<String>,
    unit: Option<String>,
    #[serde(default, deserialize_with = "empty_string_as_none_i64")]
    category_id: Option<i64>,
    category_name: Option<String>,  // 新分类名称，category_id 为空时自动创建
    #[serde(default, deserialize_with = "empty_string_as_none_i64")]
    brand_id: Option<i64>,
    brand_name: Option<String>,  // 新品牌名称，brand_id 为空时自动创建
    #[serde(default, deserialize_with = "empty_string_as_none_i64")]
    supplier_id: Option<i64>,
    supplier_name: Option<String>,  // 新供应商名称，supplier_id 为空时自动创建
    #[serde(default, deserialize_with = "empty_string_as_none_f64")]
    weight: Option<f64>,
    #[serde(default, deserialize_with = "empty_string_as_none_f64")]
    volume: Option<f64>,
    description: Option<String>,
    #[serde(default, deserialize_with = "empty_string_as_none_i64")]
    status: Option<i64>,
    is_featured: Option<String>,
    is_new: Option<String>,
    notes: Option<String>,
    // 参考成本（可选）
    #[serde(default, deserialize_with = "empty_string_as_none_f64")]
    cost_cny: Option<f64>,
    #[serde(default, deserialize_with = "empty_string_as_none_f64")]
    cost_usd: Option<f64>,
    #[serde(default, deserialize_with = "empty_string_as_none_f64")]
    cost_exchange_rate: Option<f64>,
    cost_notes: Option<String>,
    // 参考售价
    #[serde(default, deserialize_with = "empty_string_as_none_f64")]
    ae_fee_rate: Option<f64>,
    #[serde(default, deserialize_with = "empty_string_as_none_f64")]
    ali_fee_rate: Option<f64>,
    #[serde(default, deserialize_with = "empty_string_as_none_f64")]
    web_fee_rate: Option<f64>,
    // 全局汇率
    #[serde(default, deserialize_with = "empty_string_as_none_f64")]
    price_exchange_rate: Option<f64>,
}

#[derive(Debug, Clone)]
struct ProductPricingInput {
    supplier_id: Option<i64>,
    cost_cny: Option<f64>,
    cost_notes: Option<String>,
    ae_fee_rate: Option<f64>,
    ali_fee_rate: Option<f64>,
    web_fee_rate: Option<f64>,
    price_exchange_rate: Option<f64>,
}

impl From<&ProductForm> for ProductPricingInput {
    fn from(form: &ProductForm) -> Self {
        Self {
            supplier_id: form.supplier_id,
            cost_cny: form.cost_cny,
            cost_notes: form.cost_notes.clone(),
            ae_fee_rate: form.ae_fee_rate,
            ali_fee_rate: form.ali_fee_rate,
            web_fee_rate: form.web_fee_rate,
            price_exchange_rate: form.price_exchange_rate,
        }
    }
}

impl From<&ProductEditForm> for ProductPricingInput {
    fn from(form: &ProductEditForm) -> Self {
        Self {
            supplier_id: form.supplier_id,
            cost_cny: form.cost_cny,
            cost_notes: form.cost_notes.clone(),
            ae_fee_rate: form.ae_fee_rate,
            ali_fee_rate: form.ali_fee_rate,
            web_fee_rate: form.web_fee_rate,
            price_exchange_rate: form.price_exchange_rate,
        }
    }
}

const AE_DEFAULT_FEE_RATE: f64 = 0.12;
const ALI_DEFAULT_FEE_RATE: f64 = 0.025;
const WEB_DEFAULT_FEE_RATE: f64 = 0.0;
const AE_FIXED_PROFIT_MARGIN: f64 = 0.40;
const ALI_FIXED_PROFIT_MARGIN: f64 = 0.15;
const WEB_FIXED_PROFIT_MARGIN: f64 = 0.40;

fn compute_sale_price(cost: f64, fee_rate: f64, profit_margin: f64) -> Option<f64> {
    let denominator = 1.0 - fee_rate - profit_margin;
    if cost > 0.0 && denominator > 0.0 {
        Some(cost / denominator)
    } else {
        None
    }
}

fn build_reference_cost_write(
    product_id: i64,
    pricing: &ProductPricingInput,
    exchange_rate: f64,
) -> Option<ReferenceCostWrite> {
    let cost_cny = pricing.cost_cny.filter(|v| *v > 0.0)?;
    Some(ReferenceCostWrite {
        product_id,
        supplier_id: pricing.supplier_id,
        cost_cny,
        cost_usd: Some(cost_cny / exchange_rate),
        exchange_rate,
        notes: pricing.cost_notes.clone(),
    })
}

fn build_aliexpress_price_write(
    product_id: i64,
    pricing: &ProductPricingInput,
    exchange_rate: f64,
) -> Option<ReferencePriceWrite> {
    let cost_cny = pricing.cost_cny.filter(|v| *v > 0.0)?;
    let fee_rate = pricing.ae_fee_rate.map(|v| v / 100.0).unwrap_or(AE_DEFAULT_FEE_RATE);
    let sale_price_cny = compute_sale_price(cost_cny, fee_rate, AE_FIXED_PROFIT_MARGIN)?;
    Some(ReferencePriceWrite {
        product_id,
        platform: "aliexpress".to_string(),
        sale_price_cny,
        sale_price_usd: Some(sale_price_cny / exchange_rate),
        exchange_rate,
        profit_margin: Some(AE_FIXED_PROFIT_MARGIN),
        platform_fee_rate: Some(fee_rate),
        notes: None,
        pricing_mode: Some("margin".to_string()),
        input_currency: Some("CNY".to_string()),
        reference_platform: None,
        adjustment_type: None,
        adjustment_value: None,
    })
}

fn build_alibaba_price_write(
    product_id: i64,
    pricing: &ProductPricingInput,
    exchange_rate: f64,
) -> Option<ReferencePriceWrite> {
    let cost_cny = pricing.cost_cny.filter(|v| *v > 0.0)?;
    let cost_usd = cost_cny / exchange_rate;
    let fee_rate = pricing.ali_fee_rate.map(|v| v / 100.0).unwrap_or(ALI_DEFAULT_FEE_RATE);
    let sale_price_usd = compute_sale_price(cost_usd, fee_rate, ALI_FIXED_PROFIT_MARGIN)?;
    Some(ReferencePriceWrite {
        product_id,
        platform: "alibaba".to_string(),
        sale_price_cny: sale_price_usd * exchange_rate,
        sale_price_usd: Some(sale_price_usd),
        exchange_rate,
        profit_margin: Some(ALI_FIXED_PROFIT_MARGIN),
        platform_fee_rate: Some(fee_rate),
        notes: None,
        pricing_mode: Some("markup".to_string()),
        input_currency: Some("USD".to_string()),
        reference_platform: None,
        adjustment_type: None,
        adjustment_value: None,
    })
}

fn build_website_price_write(
    product_id: i64,
    pricing: &ProductPricingInput,
    exchange_rate: f64,
) -> Option<ReferencePriceWrite> {
    let cost_cny = pricing.cost_cny.filter(|v| *v > 0.0)?;
    let fee_rate = pricing.web_fee_rate.map(|v| v / 100.0).unwrap_or(WEB_DEFAULT_FEE_RATE);
    let sale_price_cny = compute_sale_price(cost_cny, fee_rate, WEB_FIXED_PROFIT_MARGIN)?;

    Some(ReferencePriceWrite {
        product_id,
        platform: "website".to_string(),
        sale_price_cny,
        sale_price_usd: Some(sale_price_cny / exchange_rate),
        exchange_rate,
        profit_margin: Some(WEB_FIXED_PROFIT_MARGIN),
        platform_fee_rate: Some(fee_rate),
        notes: None,
        pricing_mode: Some("margin".to_string()),
        input_currency: Some("CNY".to_string()),
        reference_platform: None,
        adjustment_type: None,
        adjustment_value: None,
    })
}

fn desired_price_count(pricing: &ProductPricingInput, exchange_rate: f64) -> usize {
    [
        build_aliexpress_price_write(0, pricing, exchange_rate).is_some(),
        build_alibaba_price_write(0, pricing, exchange_rate).is_some(),
        build_website_price_write(0, pricing, exchange_rate).is_some(),
    ]
    .into_iter()
    .filter(|exists| *exists)
    .count()
}

fn validate_pricing_configuration(pricing: &ProductPricingInput) -> Option<&'static str> {
    if matches!(pricing.ae_fee_rate, Some(value) if value < 0.0) {
        return Some("AliExpress 平台费率不能为负数");
    }
    if matches!(pricing.ali_fee_rate, Some(value) if value < 0.0) {
        return Some("Alibaba 平台费率不能为负数");
    }
    if matches!(pricing.web_fee_rate, Some(value) if value < 0.0) {
        return Some("Website 平台费率不能为负数");
    }
    if matches!(pricing.cost_cny, Some(value) if value > 0.0) {
        let ae_fee_rate = pricing.ae_fee_rate.map(|v| v / 100.0).unwrap_or(AE_DEFAULT_FEE_RATE);
        let ali_fee_rate = pricing.ali_fee_rate.map(|v| v / 100.0).unwrap_or(ALI_DEFAULT_FEE_RATE);
        let web_fee_rate = pricing.web_fee_rate.map(|v| v / 100.0).unwrap_or(WEB_DEFAULT_FEE_RATE);
        if 1.0 - AE_FIXED_PROFIT_MARGIN - ae_fee_rate <= 0.0 {
            return Some("AliExpress 平台费率过高，无法计算售价");
        }
        if 1.0 - ALI_FIXED_PROFIT_MARGIN - ali_fee_rate <= 0.0 {
            return Some("Alibaba 平台费率过高，无法计算售价");
        }
        if 1.0 - WEB_FIXED_PROFIT_MARGIN - web_fee_rate <= 0.0 {
            return Some("Website 平台费率过高，无法计算售价");
        }
    }
    None
}

async fn sync_reference_pricing(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    product_id: i64,
    pricing: &ProductPricingInput,
    exchange_rate: f64,
) -> anyhow::Result<()> {
    let cost = build_reference_cost_write(product_id, pricing, exchange_rate);
    ProductCostQueries::sync_reference_cost_tx(tx, product_id, cost.as_ref()).await?;

    if let Some(price) = build_aliexpress_price_write(product_id, pricing, exchange_rate) {
        ProductPriceQueries::upsert_reference_price_full_tx(tx, &price).await?;
    } else {
        ProductPriceQueries::delete_reference_price_tx(tx, product_id, "aliexpress").await?;
    }

    if let Some(price) = build_alibaba_price_write(product_id, pricing, exchange_rate) {
        ProductPriceQueries::upsert_reference_price_full_tx(tx, &price).await?;
    } else {
        ProductPriceQueries::delete_reference_price_tx(tx, product_id, "alibaba").await?;
    }

    if let Some(price) = build_website_price_write(product_id, pricing, exchange_rate) {
        ProductPriceQueries::upsert_reference_price_full_tx(tx, &price).await?;
    } else {
        ProductPriceQueries::delete_reference_price_tx(tx, product_id, "website").await?;
    }

    Ok(())
}

/// 创建产品处理
pub async fn product_create_handler(
    State(state): State<AppState>,
    Form(form): Form<ProductForm>,
) -> Result<impl IntoResponse, Html<String>> {
    let queries = ProductQueries::new(state.db.pool());
    let pricing_input = ProductPricingInput::from(&form);

    // 如果 brand_id 为空但填写了 brand_name，自动创建品牌
    let resolved_brand_id = if form.brand_id.is_none() {
        if let Some(ref bname) = form.brand_name {
            let bname = bname.trim();
            if !bname.is_empty() {
                let brand_queries = cicierp_db::queries::brands::BrandQueries::new(state.db.pool());
                brand_queries.find_or_create(bname).await.ok().map(|b| b.id)
            } else { None }
        } else { None }
    } else {
        form.brand_id
    };

    // 如果 category_id 为空但填写了 category_name，自动创建分类
    let resolved_category_id = if form.category_id.is_none() {
        if let Some(ref cname) = form.category_name {
            let cname = cname.trim();
            if !cname.is_empty() {
                let cat_queries = cicierp_db::queries::categories::CategoryQueries::new(state.db.pool());
                cat_queries.find_or_create(cname).await.ok().map(|c| c.id)
            } else { None }
        } else { None }
    } else {
        form.category_id
    };

    // 如果 supplier_id 为空但填写了 supplier_name，创建供应商（仅当名称非空且不重复）
    let resolved_supplier_id = form.supplier_id;

    // 获取缓冲汇率作为默认值
    let buffered_rate = {
        let eq = ExchangeRateQueries::new(state.db.pool());
        let rate = eq.get_latest_rate("USD", "CNY").await
            .ok().flatten().map(|r| r.rate).unwrap_or(7.2);
        ((rate - 0.05) * 100.0).round() / 100.0
    };

    // BUG-027: 数值校验
    if let Some(r) = form.cost_exchange_rate { if r <= 0.0 { return Err(render_product_form_error("汇率必须大于0", &form)); } }
    if let Some(r) = form.price_exchange_rate { if r <= 0.0 { return Err(render_product_form_error("汇率必须大于0", &form)); } }
    if let Some(message) = validate_pricing_configuration(&pricing_input) {
        return Err(render_product_form_error(message, &form));
    }

    // BUG-030/036: 无售价时强制草稿状态
    let effective_status = {
        let exchange_rate = pricing_input.price_exchange_rate.unwrap_or(buffered_rate);
        let has_price = desired_price_count(&pricing_input, exchange_rate) > 0;
        if !has_price {
            Some(3) // 草稿
        } else {
            form.status
        }
    };

    // 产品编码由系统自动生成，不再需要检查

    let req = CreateProductRequest {
        product_code: None,  // 自动生成
        name: form.name.clone(),
        model: form.model.clone(),
        name_en: form.name_en.clone(),
        slug: None,
        category_id: resolved_category_id,
        brand_id: resolved_brand_id,
        supplier_id: resolved_supplier_id,
        weight: form.weight,
        volume: form.volume,
        description: form.description.clone(),
        description_en: None,
        specifications: None,
        main_image: None,
        images: None,
        status: effective_status,
        is_featured: Some(form.is_featured.is_some()),
        is_new: Some(form.is_new.is_some()),
        notes: form.notes.clone(),
        unit: form.unit.clone(),
    };

    match state.db.pool().begin().await {
        Ok(mut tx) => match queries.create_in_tx(&mut tx, &req).await {
            Ok(product) => {
                info!("Product created: id={}, code={}", product.id, product.product_code);

                let exchange_rate = pricing_input.price_exchange_rate.unwrap_or(buffered_rate);
                if let Err(e) = sync_reference_pricing(&mut tx, product.id, &pricing_input, exchange_rate).await {
                    info!("Failed to create product pricing data: {}", e);
                    let _ = tx.rollback().await;
                    return Err(render_product_form_error("创建产品价格/成本失败，请检查输入信息", &form));
                }

                if let Err(e) = tx.commit().await {
                    info!("Failed to commit product create transaction: {}", e);
                    return Err(render_product_form_error("创建产品失败，请稍后重试", &form));
                }

                Ok(Redirect::to(&format!("/products/{}", product.id)))
            }
            Err(e) => {
                info!("Failed to create product: {}", e);
                let _ = tx.rollback().await;
                Err(render_product_form_error("创建产品失败，请检查输入信息", &form))
            }
        },
        Err(e) => {
            info!("Failed to start product create transaction: {}", e);
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

<div class="mb-6 p-4 bg-red-50 border border-red-200 rounded-lg">
    <p class="text-red-600 text-sm">{}</p>
</div>

<div class="bg-white rounded-xl shadow-sm border border-gray-100">
    <form action="/products/new" method="POST" class="p-4 sm:p-6 space-y-6">
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4 sm:gap-6">
            <div>
                <label for="name" class="block text-sm font-medium text-gray-700 mb-2">产品名称 <span class="text-red-500">*</span></label>
                <input type="text" id="name" name="name" value="{}" required
                       class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent">
            </div>
            <div>
                <label for="name_en" class="block text-sm font-medium text-gray-700 mb-2">英文名称</label>
                <input type="text" id="name_en" name="name_en" value="{}"
                       class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent">
            </div>
            <div>
                <label for="cost_cny" class="block text-sm font-medium text-gray-700 mb-2">成本 (CNY)</label>
                <input type="number" id="cost_cny" name="cost_cny" value="{}" step="0.01" min="0"
                       class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent">
            </div>
            <div>
                <label for="price_exchange_rate" class="block text-sm font-medium text-gray-700 mb-2">参考汇率 (USD/CNY)</label>
                <input type="number" id="price_exchange_rate" name="price_exchange_rate" value="{}" step="0.01" min="0"
                       class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent">
            </div>
            <div>
                <label for="ae_fee_rate" class="block text-sm font-medium text-gray-700 mb-2">AliExpress 平台费率 (%)</label>
                <input type="number" id="ae_fee_rate" name="ae_fee_rate" value="{}" step="0.01" min="0"
                       class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent">
            </div>
            <div>
                <label for="ali_fee_rate" class="block text-sm font-medium text-gray-700 mb-2">Alibaba 平台费率 (%)</label>
                <input type="number" id="ali_fee_rate" name="ali_fee_rate" value="{}" step="0.01" min="0"
                       class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent">
            </div>
            <div>
                <label for="web_fee_rate" class="block text-sm font-medium text-gray-700 mb-2">Website 平台费率 (%)</label>
                <input type="number" id="web_fee_rate" name="web_fee_rate" value="{}" step="0.01" min="0"
                       class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent">
            </div>
            <div>
                <label for="status" class="block text-sm font-medium text-gray-700 mb-2">状态</label>
                <select id="status" name="status"
                        class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent">
                    <option value="3" {}>草稿</option>
                    <option value="1" {}>上架 (需已设置售价)</option>
                    <option value="2" {}>下架</option>
                </select>
            </div>
        </div>

        <div>
            <label for="description" class="block text-sm font-medium text-gray-700 mb-2">产品描述</label>
            <textarea id="description" name="description" rows="4"
                      class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent">{}</textarea>
        </div>

        <div class="flex items-center gap-4">
            <button type="submit" class="px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors">保存产品</button>
            <a href="/products" class="px-6 py-2 bg-gray-100 text-gray-700 rounded-lg hover:bg-gray-200 transition-colors">取消</a>
        </div>
    </form>
</div>"#,
        error,
        form.name,
        form.name_en.as_deref().unwrap_or(""),
        form.cost_cny.unwrap_or(0.0),
        form.price_exchange_rate.unwrap_or(0.0),
        form.ae_fee_rate.unwrap_or(12.0),
        form.ali_fee_rate.unwrap_or(2.5),
        form.web_fee_rate.unwrap_or(0.0),
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
    let pool = state.db.pool();

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

    // 获取成本信息
    let cost_queries = ProductCostQueries::new(pool);
    let product_cost = cost_queries.get_reference_cost(id).await.unwrap_or(None);

    // 获取三个平台价格
    let price_queries = ProductPriceQueries::new(pool);
    let price_website = price_queries.get_reference_price(id, "website").await.unwrap_or(None);
    let price_alibaba = price_queries.get_reference_price(id, "alibaba").await.unwrap_or(None);
    let price_aliexpress = price_queries.get_reference_price(id, "aliexpress").await.unwrap_or(None);

    // 获取供应商、品牌、分类名称
    let supplier_name: String = if let Some(sid) = product.supplier_id {
        sqlx::query_scalar("SELECT name FROM suppliers WHERE id = ?")
            .bind(sid).fetch_optional(pool).await.ok().flatten().unwrap_or_default()
    } else { String::new() };

    let brand_name: String = if let Some(bid) = product.brand_id {
        sqlx::query_scalar("SELECT name FROM brands WHERE id = ?")
            .bind(bid).fetch_optional(pool).await.ok().flatten().unwrap_or_default()
    } else { String::new() };

    let category_name: String = if let Some(cid) = product.category_id {
        sqlx::query_scalar("SELECT name FROM categories WHERE id = ?")
            .bind(cid).fetch_optional(pool).await.ok().flatten().unwrap_or_default()
    } else { String::new() };

    // 获取内容信息
    let content_queries = ProductContentQueries::new(pool);
    let product_content = content_queries.get_by_product_id(id).await.unwrap_or(None);

    let status_badge = match product.status {
        1 => r#"<span class="px-3 py-1 text-sm font-medium bg-green-100 text-green-700 rounded-full">上架</span>"#,
        2 => r#"<span class="px-3 py-1 text-sm font-medium bg-gray-100 text-gray-600 rounded-full">下架</span>"#,
        _ => r#"<span class="px-3 py-1 text-sm font-medium bg-yellow-100 text-yellow-700 rounded-full">草稿</span>"#,
    };

    let featured_badge = if product.is_featured {
        r#"<span class="px-2 py-0.5 text-xs bg-amber-100 text-amber-700 rounded-full">精选</span>"#
    } else { "" };

    let new_badge = if product.is_new {
        r#"<span class="px-2 py-0.5 text-xs bg-blue-100 text-blue-700 rounded-full">新品</span>"#
    } else { "" };

    // 帮助函数：格式化平台价格区块（AliExpress 只显示 CNY，Alibaba 只显示 USD，其他只显示 CNY）
    let fmt_platform_price = |platform_label: &str, platform: &str, price: Option<&cicierp_models::product::ProductPrice>| -> String {
        if let Some(p) = price {
            let price_rows = if platform == "ali" {
                // Alibaba: 只显示 USD 主售价
                format!(
                    r#"<div class="col-span-2"><p class="text-xs text-gray-400">售价(USD)</p><p class="font-medium">{}</p></div>"#,
                    p.sale_price_usd.map(|v| format!("${:.2}", v)).unwrap_or("-".to_string()),
                )
            } else {
                // AliExpress / Website: 只显示 CNY
                format!(
                    r#"<div class="col-span-2"><p class="text-xs text-gray-400">售价(CNY)</p><p class="font-medium">¥{:.2}</p></div>"#,
                    p.sale_price_cny,
                )
            };
            format!(
                r#"<div class="bg-gray-50 rounded-lg p-3">
    <p class="text-xs font-semibold text-gray-500 mb-2 uppercase">{}</p>
    <div class="grid grid-cols-2 gap-2 text-sm">
        {}
        <div><p class="text-xs text-gray-400">平台费率</p><p class="font-medium">{:.1}%</p></div>
    </div>
    {}
</div>"#,
                platform_label,
                price_rows,
                p.platform_fee_rate * 100.0,
                p.notes.as_deref().map(|n| format!(r#"<p class="text-xs text-gray-400 mt-1">备注: {}</p>"#, n)).unwrap_or_default()
            )
        } else {
            format!(
                r#"<div class="bg-gray-50 rounded-lg p-3">
    <p class="text-xs font-semibold text-gray-500 mb-2 uppercase">{}</p>
    <p class="text-sm text-gray-400">暂无价格</p>
</div>"#,
                platform_label
            )
        }
    };

    let price_alibaba_html = fmt_platform_price("Alibaba", "ali", price_alibaba.as_ref());
    let price_aliexpress_html = fmt_platform_price("AliExpress", "ae", price_aliexpress.as_ref());
    let price_website_html = fmt_platform_price("Website", "web", price_website.as_ref());

    // 成本信息 HTML
    let cost_section = if let Some(ref cost) = product_cost {
        format!(
            r#"<div class="bg-white rounded-xl shadow-sm border border-gray-100 p-4 sm:p-6">
    <h3 class="text-base font-semibold text-gray-800 mb-4">💰 成本信息</h3>
    <div class="grid grid-cols-2 gap-4">
        <div><p class="text-xs text-gray-500">成本(CNY)</p><p class="text-lg font-semibold">¥{:.2}</p></div>
        <div><p class="text-xs text-gray-500">成本(USD)</p><p class="text-lg font-semibold">{}</p></div>
        <div><p class="text-xs text-gray-500">汇率</p><p class="font-medium">{:.4}</p></div>
    </div>
    {}
</div>"#,
            cost.cost_cny,
            cost.cost_usd.map(|v| format!("${:.2}", v)).unwrap_or("-".to_string()),
            cost.exchange_rate,
            cost.notes.as_deref().map(|n| format!(r#"<div class="mt-3 pt-3 border-t border-gray-100"><p class="text-xs text-gray-500">备注: {}</p></div>"#, n)).unwrap_or_default()
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
    <div class="space-y-3">
        <div><p class="text-xs text-gray-500 mb-1">英文标题</p><p class="text-sm text-gray-800">{}</p></div>
        <div><p class="text-xs text-gray-500 mb-1">描述</p><p class="text-sm text-gray-800">{}</p></div>
        <div><p class="text-xs text-gray-500 mb-1">SEO 标题</p><p class="text-sm text-gray-800">{}</p></div>
    </div>
</div>"#,
            content.title_en.as_deref().unwrap_or("-"),
            content.description.as_deref().map(|d| if d.len() > 300 { &d[..300] } else { d }).unwrap_or("-"),
            content.meta_title.as_deref().unwrap_or("-")
        )
    } else {
        String::new()
    };

    let unit_display = product.unit.as_deref().unwrap_or("-").to_string();

    let content = format!(
        r#"<!-- 页面标题 -->
<div class="mb-6">
    <div class="flex items-center gap-2 text-sm text-gray-500 mb-2">
        <a href="/products" class="hover:text-blue-600">产品列表</a>
        <span>/</span>
        <span class="text-gray-800">{}</span>
    </div>
    <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div class="flex items-center gap-3 flex-wrap">
            <h1 class="text-xl sm:text-2xl font-bold text-gray-800">{}</h1>
            {}
            {}
            {}
        </div>
        <div class="flex items-center gap-2">
            <a href="/products/{}/edit" class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors text-sm">编辑</a>
            <a href="/products" class="px-4 py-2 bg-gray-100 text-gray-700 rounded-lg hover:bg-gray-200 transition-colors text-sm">返回列表</a>
        </div>
    </div>
</div>

<!-- 基本信息 -->
<div class="bg-white rounded-xl shadow-sm border border-gray-100 p-4 sm:p-6 mb-6">
    <h3 class="text-base font-semibold text-gray-800 mb-4">📦 基本信息</h3>
    <div class="grid grid-cols-2 md:grid-cols-4 gap-4 sm:gap-6">
        <div><p class="text-xs text-gray-500 mb-1">产品编码</p><p class="font-mono text-sm text-gray-800">{}</p></div>
        <div><p class="text-xs text-gray-500 mb-1">产品名称</p><p class="text-sm font-medium text-gray-800">{}</p></div>
        <div><p class="text-xs text-gray-500 mb-1">英文名称</p><p class="text-sm text-gray-800">{}</p></div>
        <div><p class="text-xs text-gray-500 mb-1">型号</p><p class="text-sm text-gray-800">{}</p></div>
        <div><p class="text-xs text-gray-500 mb-1">分类</p><p class="text-sm text-gray-800">{}</p></div>
        <div><p class="text-xs text-gray-500 mb-1">品牌</p><p class="text-sm text-gray-800">{}</p></div>
        <div><p class="text-xs text-gray-500 mb-1">供应商</p><p class="text-sm text-gray-800">{}</p></div>
        <div><p class="text-xs text-gray-500 mb-1">创建时间</p><p class="text-sm text-gray-800">{}</p></div>
        <div><p class="text-xs text-gray-500 mb-1">重量</p><p class="text-sm text-gray-800">{}</p></div>
        <div><p class="text-xs text-gray-500 mb-1">体积</p><p class="text-sm text-gray-800">{}</p></div>
        <div><p class="text-xs text-gray-500 mb-1">单位</p><p class="text-sm text-gray-800">{unit_display}</p></div>
    </div>
    <div class="mt-4 pt-4 border-t border-gray-100 grid grid-cols-1 md:grid-cols-2 gap-4">
        <div><p class="text-xs text-gray-500 mb-1">产品描述</p><p class="text-sm text-gray-700">{}</p></div>
        <div><p class="text-xs text-gray-500 mb-1">备注</p><p class="text-sm text-gray-700">{}</p></div>
    </div>
</div>

<!-- 售价信息（三平台） -->
<div class="bg-white rounded-xl shadow-sm border border-gray-100 p-4 sm:p-6 mb-6">
    <h3 class="text-base font-semibold text-gray-800 mb-4">💵 售价信息</h3>
    <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
        {}
        {}
        {}
    </div>
</div>

<!-- 成本 + 内容 -->
<div class="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-6">
    {}
    {}
</div>"#,
        // breadcrumb + title
        product.name,
        product.name,
        status_badge,
        featured_badge,
        new_badge,
        product.id,
        // basic info grid
        product.product_code,
        product.name,
        product.name_en.as_deref().unwrap_or("-"),
        product.model.as_deref().unwrap_or("-"),
        if category_name.is_empty() { "-".to_string() } else { category_name },
        if brand_name.is_empty() { "-".to_string() } else { brand_name },
        if supplier_name.is_empty() { "-".to_string() } else { supplier_name },
        product.created_at.format("%Y-%m-%d %H:%M"),
        product.weight.map(|w| format!("{:.3} kg", w)).unwrap_or("-".to_string()),
        product.volume.map(|v| format!("{:.4} m³", v)).unwrap_or("-".to_string()),
        product.description.as_deref().unwrap_or("无描述"),
        product.notes.as_deref().unwrap_or("无"),
        // price section
        price_alibaba_html,
        price_aliexpress_html,
        price_website_html,
        // cost + content
        cost_section,
        content_section
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

    // 获取供应商、品牌、分类名称（用于显示在编辑表单中）
    let supplier_name: String = if let Some(sid) = product.supplier_id {
        sqlx::query_scalar("SELECT name FROM suppliers WHERE id = ?")
            .bind(sid).fetch_optional(state.db.pool()).await.ok().flatten().unwrap_or_default()
    } else { String::new() };

    let brand_name: String = if let Some(bid) = product.brand_id {
        sqlx::query_scalar("SELECT name FROM brands WHERE id = ?")
            .bind(bid).fetch_optional(state.db.pool()).await.ok().flatten().unwrap_or_default()
    } else { String::new() };

    let category_name: String = if let Some(cid) = product.category_id {
        sqlx::query_scalar("SELECT name FROM categories WHERE id = ?")
            .bind(cid).fetch_optional(state.db.pool()).await.ok().flatten().unwrap_or_default()
    } else { String::new() };

    // 获取价格信息（三个平台）
    let price_queries = ProductPriceQueries::new(state.db.pool());
    let product_price = price_queries.get_reference_price(id, "website").await.unwrap_or(None);
    let price_alibaba = price_queries.get_reference_price(id, "alibaba").await.unwrap_or(None);
    let price_aliexpress = price_queries.get_reference_price(id, "aliexpress").await.unwrap_or(None);

    // 获取缓冲汇率（市场汇率 - 0.05）作为无数据时的默认值
    let buffered_rate_str = {
        let eq = ExchangeRateQueries::new(state.db.pool());
        let rate = eq.get_latest_rate("USD", "CNY").await
            .ok().flatten().map(|r| r.rate).unwrap_or(7.2);
        format!("{:.2}", (rate - 0.05).max(0.0))
    };

    // 状态选中
    let status_options = format!(
        r#"<option value="3" {}>草稿</option>
            <option value="1" {}>上架 (需已设置售价)</option>
            <option value="2" {}>下架</option>"#,
        if product.status == 3 { "selected" } else { "" },
        if product.status == 1 { "selected" } else { "" },
        if product.status == 2 { "selected" } else { "" }
    );

    // 预计算 unit 用于模板隐式捕获
    let unit_options = {
        let u = product.unit.as_deref().unwrap_or("pcs");
        format!(
            r#"<option value="pcs"{s_pcs}>件 (pcs)</option>
                        <option value="set"{s_set}>套 (set)</option>
                        <option value="pair"{s_pair}>对 (pair)</option>
                        <option value="box"{s_box}>箱 (box)</option>
                        <option value="m"{s_m}>米 (m)</option>
                        <option value="kg"{s_kg}>千克 (kg)</option>"#,
            s_pcs = if u == "pcs" { " selected" } else { "" },
            s_set = if u == "set" { " selected" } else { "" },
            s_pair = if u == "pair" { " selected" } else { "" },
            s_box = if u == "box" { " selected" } else { "" },
            s_m = if u == "m" { " selected" } else { "" },
            s_kg = if u == "kg" { " selected" } else { "" },
        )
    };

    // 预计算定价相关变量（用于 format! 模板捕获）
    let ae_price_cny = price_aliexpress.as_ref().map(|p| format!("{:.2}", p.sale_price_cny)).unwrap_or_default();
    let ae_fee = price_aliexpress.as_ref().map(|p| format!("{:.1}", p.platform_fee_rate * 100.0)).unwrap_or_else(|| "12.0".to_string());
    let ali_price_usd = price_alibaba.as_ref().map(|p| p.sale_price_usd.map(|v| format!("{:.2}", v)).unwrap_or_default()).unwrap_or_default();
    let ali_fee = price_alibaba.as_ref().map(|p| format!("{:.1}", p.platform_fee_rate * 100.0)).unwrap_or_else(|| "2.5".to_string());
    let web_price_cny = product_price.as_ref().map(|p| format!("{:.2}", p.sale_price_cny)).unwrap_or_default();
    let web_price_usd = product_price.as_ref().and_then(|p| p.sale_price_usd).map(|v| format!("{:.2}", v)).unwrap_or_default();
    let web_fee = product_price.as_ref().map(|p| format!("{:.1}", p.platform_fee_rate * 100.0)).unwrap_or_else(|| "0.0".to_string());
    let rate_val = product_price.as_ref()
        .or(price_alibaba.as_ref())
        .or(price_aliexpress.as_ref())
        .map(|p| format!("{:.2}", p.exchange_rate))
        .or_else(|| product_cost.as_ref().map(|c| format!("{:.2}", c.exchange_rate)))
        .unwrap_or_else(|| buffered_rate_str.clone());

    let content = format!(
        r#"<!-- 页面标题 -->
<div class="mb-6">
    <div class="flex items-center gap-2 text-sm text-gray-500 mb-2">
        <a href="/products" class="hover:text-blue-600">产品列表</a>
        <span>/</span>
        <a href="/products/{id}" class="hover:text-blue-600">{}</a>
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

                <!-- 型号 -->
                <div>
                    <label for="model" class="block text-sm font-medium text-gray-700 mb-2">
                        型号
                    </label>
                    <input type="text" id="model" name="model" value="{}"
                           class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                           placeholder="对外展示的型号，如 V3.0 / Pro Max">
                    <p class="text-xs text-gray-400 mt-1">对外展示字段，用于报价单和产品资料</p>
                </div>

                <!-- 单位 -->
                <div>
                    <label for="unit" class="block text-sm font-medium text-gray-700 mb-2">单位</label>
                    <select id="unit" name="unit" class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent">
                        {unit_options}
                    </select>
                </div>

                <!-- 分类 -->
                <div>
                    <label for="edit_category_name_input" class="block text-sm font-medium text-gray-700 mb-2">分类</label>
                    <div class="relative">
                        <input type="text" id="edit_category_name_input" autocomplete="off" value="{}"
                               class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                               placeholder="输入分类名称搜索或新增">
                        <input type="hidden" id="edit_category_id" name="category_id" value="{}">
                        <input type="hidden" id="edit_category_name" name="category_name" value="{}">
                        <div id="edit_category_dropdown" class="hidden absolute z-10 w-full bg-white border border-gray-200 rounded-lg shadow-lg mt-1 max-h-48 overflow-y-auto"></div>
                    </div>
                </div>

                <!-- 品牌 -->
                <div>
                    <label for="edit_brand_name_input" class="block text-sm font-medium text-gray-700 mb-2">品牌</label>
                    <div class="relative">
                        <input type="text" id="edit_brand_name_input" autocomplete="off" value="{}"
                               class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                               placeholder="输入品牌名称搜索或新增">
                        <input type="hidden" id="edit_brand_id" name="brand_id" value="{}">
                        <input type="hidden" id="edit_brand_name" name="brand_name" value="{}">
                        <div id="edit_brand_dropdown" class="hidden absolute z-10 w-full bg-white border border-gray-200 rounded-lg shadow-lg mt-1 max-h-48 overflow-y-auto"></div>
                    </div>
                </div>

                <!-- 供应商 -->
                <div>
                    <label for="edit_supplier_name_input" class="block text-sm font-medium text-gray-700 mb-2">供应商</label>
                    <div class="relative">
                        <input type="text" id="edit_supplier_name_input" autocomplete="off" value="{}"
                               class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                               placeholder="输入供应商名称搜索或新增">
                        <input type="hidden" id="edit_supplier_id" name="supplier_id" value="{}">
                        <input type="hidden" id="edit_supplier_name" name="supplier_name" value="{}">
                        <div id="edit_supplier_dropdown" class="hidden absolute z-10 w-full bg-white border border-gray-200 rounded-lg shadow-lg mt-1 max-h-48 overflow-y-auto"></div>
                    </div>
                </div>

                <!-- 重量 -->
                <div>
                    <label for="weight" class="block text-sm font-medium text-gray-700 mb-2">
                        重量 (kg) <span class="text-xs text-gray-400">(仅记录)</span>
                    </label>
                    <input type="number" id="weight" name="weight" step="0.001" min="0" value="{}"
                           class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                           placeholder="0.000">
                </div>

                <!-- 体积 -->
                <div>
                    <label for="volume" class="block text-sm font-medium text-gray-700 mb-2">
                        体积 (m³) <span class="text-xs text-gray-400">(仅记录)</span>
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

                <!-- 标记 -->
                <div class="flex items-center gap-4 pt-6">
                    <label class="flex items-center gap-2 cursor-pointer">
                        <input type="checkbox" name="is_featured" value="true" {} class="w-4 h-4 text-blue-600 rounded focus:ring-blue-500">
                        <span class="text-sm text-gray-700">推荐产品</span>
                    </label>
                    <label class="flex items-center gap-2 cursor-pointer">
                        <input type="checkbox" name="is_new" value="true" {} class="w-4 h-4 text-blue-600 rounded focus:ring-blue-500">
                        <span class="text-sm text-gray-700">新品</span>
                    </label>
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

                <!-- 成本 USD（只读，自动由CNY/当前汇率计算） -->
                <div>
                    <label for="cost_usd" class="block text-sm font-medium text-gray-700 mb-2">
                        成本 (USD) <span class="text-xs text-gray-400">(自动)</span>
                    </label>
                    <input type="number" id="cost_usd" name="cost_usd" step="0.01" min="0" value="{}"
                           class="w-full px-4 py-2 border border-gray-200 rounded-lg bg-gray-50 text-gray-500 cursor-not-allowed"
                           readonly>
                    <p class="text-xs text-gray-400 mt-1" id="cost_formula_hint">公式: 成本(CNY) ÷ 汇率 = 成本(USD)</p>
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

        <!-- 第三部分：定价（三平台独立） -->
        <div class="mb-8">
            <h3 class="text-lg font-semibold text-gray-800 mb-4 pb-2 border-b border-gray-200">
                🏷️ 平台定价 <span class="text-sm font-normal text-gray-500">(按平台分别设置)</span>
            </h3>

            <!-- 全局汇率 -->
            <div class="mb-4 p-3 bg-blue-50 rounded-lg flex items-center gap-4">
                <label class="text-sm font-medium text-blue-800 whitespace-nowrap">💱 参考汇率 (USD/CNY)</label>
                <input type="number" id="price_exchange_rate" name="price_exchange_rate" step="0.01" min="0"
                       value="{rate_val}"
                       class="w-32 px-3 py-1.5 border border-blue-200 rounded-lg text-sm focus:ring-2 focus:ring-blue-500">
                <span class="text-xs text-blue-600">成本和售价的CNY/USD换算共用此汇率</span>
            </div>

            <!-- AliExpress 区块 -->
            <div class="mb-4 p-4 border border-orange-200 rounded-xl bg-orange-50">
                <div class="flex items-center gap-2 mb-3">
                    <span class="text-sm font-semibold text-orange-700">🛒 AliExpress</span>
                    <span class="text-xs text-orange-500 bg-orange-100 px-2 py-0.5 rounded-full">固定利润率 40%</span>
                </div>
                <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
                    <div>
                        <label class="block text-xs font-medium text-gray-600 mb-1">平台费率 (%)</label>
                        <input type="number" id="ae_fee_rate" name="ae_fee_rate" step="0.01" min="0" value="{ae_fee}"
                               class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:ring-2 focus:ring-orange-500"
                               placeholder="5.0">
                    </div>
                    <div>
                        <label class="block text-xs font-medium text-gray-600 mb-1">售价 CNY <span class="text-xs text-gray-400">(自动)</span></label>
                        <input type="number" id="ae_sale_price_cny" step="0.01" min="0" value="{ae_price_cny}"
                               class="w-full px-3 py-2 border border-orange-200 rounded-lg text-sm bg-white text-gray-700 cursor-not-allowed"
                               readonly placeholder="0.00">
                    </div>
                    <div class="flex items-end">
                        <div id="ae_profit_preview" class="w-full p-2 rounded-lg bg-white border border-orange-200 text-xs text-gray-600">
                            <div>利润: <span id="ae_profit_val" class="font-medium">-</span></div>
                            <div>利润率: <span id="ae_profit_pct" class="font-medium">-</span></div>
                        </div>
                    </div>
                </div>
            </div>

            <!-- Alibaba 区块 -->
            <div class="mb-4 p-4 border border-yellow-200 rounded-xl bg-yellow-50">
                <div class="flex items-center gap-2 mb-3">
                    <span class="text-sm font-semibold text-yellow-700">🏪 Alibaba</span>
                    <span class="text-xs text-yellow-600 bg-yellow-100 px-2 py-0.5 rounded-full">固定利润率 15%</span>
                </div>
                <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
                    <div>
                        <label class="block text-xs font-medium text-gray-600 mb-1">平台费率 (%)</label>
                        <input type="number" id="ali_fee_rate" name="ali_fee_rate" step="0.01" min="0" value="{ali_fee}"
                               class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:ring-2 focus:ring-yellow-500"
                               placeholder="3.0">
                    </div>
                    <div>
                        <label class="block text-xs font-medium text-gray-600 mb-1">售价 USD <span class="text-xs text-gray-400">(自动)</span></label>
                        <input type="number" id="ali_sale_price_usd" step="0.01" min="0" value="{ali_price_usd}"
                               class="w-full px-3 py-2 border border-yellow-200 rounded-lg text-sm bg-white text-gray-700 cursor-not-allowed"
                               readonly placeholder="0.00">
                    </div>
                    <div class="flex items-end">
                        <div id="ali_profit_preview" class="w-full p-2 rounded-lg bg-white border border-yellow-200 text-xs text-gray-600">
                            <div>利润: <span id="ali_profit_val" class="font-medium">-</span></div>
                            <div>利润率: <span id="ali_profit_pct" class="font-medium">-</span></div>
                        </div>
                    </div>
                </div>
            </div>

            <!-- Website 区块 -->
            <div class="p-4 border border-blue-200 rounded-xl bg-blue-50">
                <div class="flex items-center gap-2 mb-3">
                    <span class="text-sm font-semibold text-blue-700">🌐 Website</span>
                    <span class="text-xs text-blue-500 bg-blue-100 px-2 py-0.5 rounded-full">固定利润率 40%</span>
                </div>
                <div class="grid grid-cols-1 md:grid-cols-4 gap-3">
                    <div>
                        <label class="block text-xs font-medium text-gray-600 mb-1">平台费率 (%)</label>
                        <input type="number" id="web_fee_rate" name="web_fee_rate" step="0.01" min="0" value="{web_fee}"
                               class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:ring-2 focus:ring-blue-500"
                               placeholder="2.5">
                    </div>
                    <div>
                        <label class="block text-xs font-medium text-gray-600 mb-1">售价 CNY <span class="text-xs text-gray-400">(自动)</span></label>
                        <input type="number" id="web_sale_price_cny" step="0.01" min="0" value="{web_price_cny}"
                               class="w-full px-3 py-2 border border-blue-200 rounded-lg text-sm bg-white text-gray-700 cursor-not-allowed"
                               readonly placeholder="0.00">
                    </div>
                    <div>
                        <label class="block text-xs font-medium text-gray-600 mb-1">售价 USD <span class="text-xs text-gray-400">(自动)</span></label>
                        <input type="number" id="web_sale_price_usd" step="0.01" min="0" value="{web_price_usd}"
                               class="w-full px-3 py-2 border border-blue-200 rounded-lg text-sm bg-white text-gray-700 cursor-not-allowed"
                               readonly placeholder="0.00">
                    </div>
                    <div class="flex items-end">
                        <div id="web_profit_preview" class="w-full p-2 rounded-lg bg-white border border-blue-200 text-xs text-gray-600">
                            <div>利润: <span id="web_profit_val" class="font-medium">-</span></div>
                            <div>利润率: <span id="web_profit_pct" class="font-medium">-</span></div>
                        </div>
                    </div>
                </div>
            </div>

        </div>

        <!-- 提交按钮 -->
        <div class="flex items-center gap-4 pt-4 border-t border-gray-200">
            <button type="submit"
                    class="px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors">
                保存修改
            </button>
            <a href="/products/{id}" class="px-6 py-2 bg-gray-100 text-gray-700 rounded-lg hover:bg-gray-200 transition-colors">
                取消
            </a>
        </div>
    </form>
</div>
<script>
(function() {{
    function makeCombobox(inputId, idFieldId, nameFieldId, dropdownId, searchUrl) {{
        const input = document.getElementById(inputId);
        const idField = document.getElementById(idFieldId);
        const nameField = document.getElementById(nameFieldId);
        const dropdown = document.getElementById(dropdownId);
        if (!input) return;
        let timeout;
        input.addEventListener('input', function() {{
            clearTimeout(timeout);
            const q = this.value.trim();
            idField.value = '';
            nameField.value = q;
            if (q.length < 1) {{ dropdown.classList.add('hidden'); return; }}
            timeout = setTimeout(async () => {{
                try {{
                    const res = await fetch(searchUrl + encodeURIComponent(q));
                    const data = await res.json();
                    const items = data.data?.items || data.data || [];
                    if (items.length === 0) {{ dropdown.classList.add('hidden'); return; }}
                    dropdown.innerHTML = items.map(item =>
                        `<div class="px-4 py-2 hover:bg-blue-50 cursor-pointer text-sm" data-id="${{item.id}}" data-name="${{item.name}}">${{item.name}}</div>`
                    ).join('');
                    dropdown.classList.remove('hidden');
                    dropdown.querySelectorAll('[data-id]').forEach(el => {{
                        el.addEventListener('click', function() {{
                            idField.value = this.dataset.id;
                            nameField.value = this.dataset.name;
                            input.value = this.dataset.name;
                            dropdown.classList.add('hidden');
                        }});
                    }});
                }} catch(e) {{}}
            }}, 200);
        }});
        document.addEventListener('click', e => {{ if (!input.contains(e.target)) dropdown.classList.add('hidden'); }});
    }}
    makeCombobox('edit_category_name_input', 'edit_category_id', 'edit_category_name', 'edit_category_dropdown', '/api/v1/categories?keyword=');
    makeCombobox('edit_brand_name_input', 'edit_brand_id', 'edit_brand_name', 'edit_brand_dropdown', '/api/v1/brands?keyword=');
    makeCombobox('edit_supplier_name_input', 'edit_supplier_id', 'edit_supplier_name', 'edit_supplier_dropdown', '/api/v1/suppliers?keyword=');
}})();
</script>
<script>
(function() {{
    function calcCostUsd() {{
        const cny  = parseFloat(document.getElementById('cost_cny')?.value) || 0;
        const rate = parseFloat(document.getElementById('price_exchange_rate')?.value) || {buffered_rate_str};
        const costUsdEl = document.getElementById('cost_usd');
        if (cny > 0 && costUsdEl) {{
            costUsdEl.value = (cny / rate).toFixed(2);
            const hint = document.getElementById('cost_formula_hint');
            if (hint) hint.textContent = `¥${{cny.toFixed(2)}} ÷ ${{rate.toFixed(2)}} = $${{(cny/rate).toFixed(2)}}`;
        }} else if (costUsdEl) {{
            costUsdEl.value = '';
        }}
    }}

    const aeMargin = 0.40;
    const aliMargin = 0.15;
    const webMargin = 0.40;
    const rate = () => parseFloat(document.getElementById('price_exchange_rate')?.value) || {buffered_rate_str};
    const costCnyVal = () => parseFloat(document.getElementById('cost_cny')?.value) || 0;
    const costUsdVal = () => costCnyVal() / rate();
    function calcSalePrice(cost, feeRate, margin) {{
        const denominator = 1 - feeRate - margin;
        if (cost <= 0 || denominator <= 0) return 0;
        return cost / denominator;
    }}

    function calcAEPrices() {{
        const fee = (parseFloat(document.getElementById('ae_fee_rate')?.value) || 12) / 100;
        const cny = calcSalePrice(costCnyVal(), fee, aeMargin);
        const priceEl = document.getElementById('ae_sale_price_cny');
        if (priceEl) priceEl.value = cny > 0 ? cny.toFixed(2) : '';
        updateAEProfit();
    }}

    function updateAEProfit() {{
        const cny = parseFloat(document.getElementById('ae_sale_price_cny')?.value) || 0;
        const fee = (parseFloat(document.getElementById('ae_fee_rate')?.value) || 12) / 100;
        const cost = costCnyVal();
        if (cny > 0 && cost > 0) {{
            const profit = cny - cost - cny * fee;
            const pct = cny > 0 ? (profit / cny * 100).toFixed(1) : 0;
            document.getElementById('ae_profit_val').textContent = profit.toFixed(2) + ' CNY';
            document.getElementById('ae_profit_pct').textContent = pct + '%';
            document.getElementById('ae_profit_preview').className =
                'w-full p-2 rounded-lg border text-xs ' +
                (profit > 0 ? 'bg-green-50 border-green-200 text-green-700' : 'bg-red-50 border-red-200 text-red-700');
        }}
    }}

    function calcALIPrices() {{
        const fee = (parseFloat(document.getElementById('ali_fee_rate')?.value) || 2.5) / 100;
        const usd = calcSalePrice(costUsdVal(), fee, aliMargin);
        const priceEl = document.getElementById('ali_sale_price_usd');
        if (priceEl) priceEl.value = usd > 0 ? usd.toFixed(2) : '';
        updateALIProfit();
    }}

    function updateALIProfit() {{
        const usd = parseFloat(document.getElementById('ali_sale_price_usd')?.value) || 0;
        const fee = (parseFloat(document.getElementById('ali_fee_rate')?.value) || 2.5) / 100;
        const cUsd = costUsdVal();
        if (usd > 0 && cUsd > 0) {{
            const profit = usd - cUsd - usd * fee;
            const pct = usd > 0 ? (profit / usd * 100).toFixed(1) : 0;
            document.getElementById('ali_profit_val').textContent = '$' + profit.toFixed(2);
            document.getElementById('ali_profit_pct').textContent = pct + '%';
            document.getElementById('ali_profit_preview').className =
                'w-full p-2 rounded-lg border text-xs ' +
                (profit > 0 ? 'bg-green-50 border-green-200 text-green-700' : 'bg-red-50 border-red-200 text-red-700');
        }}
    }}

    function calcWebPrices() {{
        const fee = (parseFloat(document.getElementById('web_fee_rate')?.value) || 0) / 100;
        const cny = calcSalePrice(costCnyVal(), fee, webMargin);
        const cnyEl = document.getElementById('web_sale_price_cny');
        const usdEl = document.getElementById('web_sale_price_usd');
        if (cnyEl) cnyEl.value = cny > 0 ? cny.toFixed(2) : '';
        if (usdEl && cny > 0) usdEl.value = (cny / rate()).toFixed(2);
        else if (usdEl) usdEl.value = '';
        updateWebProfit();
    }}

    function updateWebProfit() {{
        const priceCny = parseFloat(document.getElementById('web_sale_price_cny')?.value) || 0;
        const fee = (parseFloat(document.getElementById('web_fee_rate')?.value) || 0) / 100;
        const cost = costCnyVal();
        if (priceCny > 0 && cost > 0) {{
            const profit = priceCny - cost - priceCny * fee;
            const pct = priceCny > 0 ? (profit / priceCny * 100).toFixed(1) : 0;
            document.getElementById('web_profit_val').textContent = profit.toFixed(2) + ' CNY';
            document.getElementById('web_profit_pct').textContent = pct + '%';
            document.getElementById('web_profit_preview').className =
                'w-full p-2 rounded-lg border text-xs ' +
                (profit > 0 ? 'bg-green-50 border-green-200 text-green-700' : 'bg-red-50 border-red-200 text-red-700');
        }}
    }}

    document.getElementById('ae_sale_price_cny')?.addEventListener('input', calcAEPrices);
    document.getElementById('ae_fee_rate')?.addEventListener('input', calcAEPrices);
    document.getElementById('ali_sale_price_usd')?.addEventListener('input', calcALIPrices);
    document.getElementById('ali_fee_rate')?.addEventListener('input', calcALIPrices);
    document.getElementById('web_fee_rate')?.addEventListener('input', calcWebPrices);
    document.getElementById('price_exchange_rate')?.addEventListener('input', function() {{
        calcCostUsd();
        calcAEPrices();
        calcALIPrices();
        calcWebPrices();
    }});
    document.getElementById('cost_cny')?.addEventListener('input', function() {{
        calcCostUsd();
        calcAEPrices();
        calcALIPrices();
        calcWebPrices();
    }});

    calcAEPrices();
    calcALIPrices();
    calcWebPrices();

    document.getElementById('status')?.addEventListener('change', function() {{
        if (this.value === '1') {{
            const aePrice = parseFloat(document.getElementById('ae_sale_price_cny')?.value) || 0;
            const aliPrice = parseFloat(document.getElementById('ali_sale_price_usd')?.value) || 0;
            const webPrice = parseFloat(document.getElementById('web_sale_price_cny')?.value) || 0;
            if (aePrice <= 0 && aliPrice <= 0 && webPrice <= 0) {{
                alert('⚠️ 请先设置售价，再将产品设为上架状态。');
                this.value = '3';
            }}
        }}
    }});

    document.querySelector('form').addEventListener('submit', function(e) {{
        const costCnyInput = parseFloat(document.getElementById('cost_cny')?.value);
        const rateVal = parseFloat(document.getElementById('price_exchange_rate')?.value);
        if (costCnyInput !== undefined && !isNaN(costCnyInput) && costCnyInput < 0) {{
            e.preventDefault(); alert('成本不能为负数'); return;
        }}
        if (rateVal !== undefined && !isNaN(rateVal) && rateVal <= 0) {{
            e.preventDefault(); alert('汇率必须大于0'); return;
        }}
    }});
}})();
</script>"#,
        product.name,
        product.id,
        product.product_code,
        product.name,
        product.name_en.as_deref().unwrap_or(""),
        product.model.as_deref().unwrap_or(""),
        &category_name,
        &product.category_id.map(|id| id.to_string()).unwrap_or_default(),
        &category_name,
        &brand_name,
        &product.brand_id.map(|id| id.to_string()).unwrap_or_default(),
        &brand_name,
        &supplier_name,
        &product.supplier_id.map(|id| id.to_string()).unwrap_or_default(),
        &supplier_name,
        product.weight.map(|w| format!("{:.3}", w)).unwrap_or_default(),
        product.volume.map(|v| format!("{:.4}", v)).unwrap_or_default(),
        status_options,
        if product.is_featured { "checked" } else { "" },
        if product.is_new { "checked" } else { "" },
        product.description.as_deref().unwrap_or(""),
        product.notes.as_deref().unwrap_or(""),
        product_cost.as_ref().map(|c| format!("{:.2}", c.cost_cny)).unwrap_or_default(),
        product_cost.as_ref().and_then(|c| c.cost_usd).map(|v| format!("{:.2}", v)).unwrap_or_default(),
        product_cost.as_ref().and_then(|c| c.notes.clone()).unwrap_or_default(),
    );

    render_layout("编辑产品", "products", Some(user), &content)
}

/// 编辑产品表单数据
#[derive(Debug, Deserialize)]
pub struct ProductEditForm {
    // 产品基本信息（不含 product_code，不可编辑）
    name: String,
    name_en: Option<String>,
    model: Option<String>,
    unit: Option<String>,
    #[serde(default, deserialize_with = "empty_string_as_none_i64")]
    category_id: Option<i64>,
    category_name: Option<String>,
    #[serde(default, deserialize_with = "empty_string_as_none_i64")]
    brand_id: Option<i64>,
    brand_name: Option<String>,
    #[serde(default, deserialize_with = "empty_string_as_none_i64")]
    supplier_id: Option<i64>,
    supplier_name: Option<String>,
    #[serde(default, deserialize_with = "empty_string_as_none_f64")]
    weight: Option<f64>,
    #[serde(default, deserialize_with = "empty_string_as_none_f64")]
    volume: Option<f64>,
    description: Option<String>,
    #[serde(default, deserialize_with = "empty_string_as_none_i64")]
    status: Option<i64>,
    is_featured: Option<String>,
    is_new: Option<String>,
    notes: Option<String>,
    // 参考成本（可选）
    #[serde(default, deserialize_with = "empty_string_as_none_f64")]
    cost_cny: Option<f64>,
    #[serde(default, deserialize_with = "empty_string_as_none_f64")]
    cost_usd: Option<f64>,
    #[serde(default, deserialize_with = "empty_string_as_none_f64")]
    cost_exchange_rate: Option<f64>,
    cost_notes: Option<String>,
    // 参考售价
    #[serde(default, deserialize_with = "empty_string_as_none_f64")]
    ae_fee_rate: Option<f64>,
    #[serde(default, deserialize_with = "empty_string_as_none_f64")]
    ali_fee_rate: Option<f64>,
    #[serde(default, deserialize_with = "empty_string_as_none_f64")]
    web_fee_rate: Option<f64>,
    // 全局汇率
    #[serde(default, deserialize_with = "empty_string_as_none_f64")]
    price_exchange_rate: Option<f64>,
}
pub async fn product_update_handler(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Form(form): Form<ProductEditForm>,
) -> Result<impl IntoResponse, Html<String>> {
    let queries = ProductQueries::new(state.db.pool());
    let pricing_input = ProductPricingInput::from(&form);

    // 检查产品是否存在
    if queries.get_by_id(id).await.unwrap_or(None).is_none() {
        let error_content = r#"<div class="text-center py-12">
            <p class="text-4xl mb-4">📦</p>
            <p class="text-gray-600 mb-4">产品不存在</p>
            <a href="/products" class="text-blue-600 hover:text-blue-800">返回产品列表</a>
        </div>"#;
        return Err(Html(error_content.to_string()));
    }

    // 如果 brand_id 为空但填写了 brand_name，自动创建品牌
    let resolved_brand_id = if form.brand_id.is_none() {
        if let Some(ref bname) = form.brand_name {
            let bname = bname.trim();
            if !bname.is_empty() {
                let brand_queries = cicierp_db::queries::brands::BrandQueries::new(state.db.pool());
                brand_queries.find_or_create(bname).await.ok().map(|b| b.id)
            } else { None }
        } else { None }
    } else {
        form.brand_id
    };

    // 如果 category_id 为空但填写了 category_name，自动创建分类
    let resolved_category_id = if form.category_id.is_none() {
        if let Some(ref cname) = form.category_name {
            let cname = cname.trim();
            if !cname.is_empty() {
                let cat_queries = cicierp_db::queries::categories::CategoryQueries::new(state.db.pool());
                cat_queries.find_or_create(cname).await.ok().map(|c| c.id)
            } else { None }
        } else { None }
    } else {
        form.category_id
    };

    // BUG-027: 数值校验
    if let Some(r) = form.cost_exchange_rate { if r <= 0.0 {
        let e = Html(format!(r#"<div class="text-center py-12"><p class="text-4xl mb-4">❌</p><p class="text-gray-600 mb-4">汇率必须大于0</p><a href="/products/{}/edit" class="text-blue-600 hover:text-blue-800">返回编辑</a></div>"#, id));
        return Err(e);
    }}
    if let Some(r) = form.price_exchange_rate { if r <= 0.0 {
        let e = Html(format!(r#"<div class="text-center py-12"><p class="text-4xl mb-4">❌</p><p class="text-gray-600 mb-4">汇率必须大于0</p><a href="/products/{}/edit" class="text-blue-600 hover:text-blue-800">返回编辑</a></div>"#, id));
        return Err(e);
    }}
    if let Some(message) = validate_pricing_configuration(&pricing_input) {
        let e = Html(format!(r#"<div class="text-center py-12"><p class="text-4xl mb-4">❌</p><p class="text-gray-600 mb-4">{}</p><a href="/products/{}/edit" class="text-blue-600 hover:text-blue-800">返回编辑</a></div>"#, message, id));
        return Err(e);
    }

    // BUG-036: 上架状态需要有售价（检查新三平台字段）
    let buffered_rate = {
        let eq = ExchangeRateQueries::new(state.db.pool());
        let rate = eq.get_latest_rate("USD", "CNY").await
            .ok().flatten().map(|r| r.rate).unwrap_or(7.2);
        ((rate - 0.05) * 100.0).round() / 100.0
    };

    let effective_status = if form.status == Some(1) {
        let exchange_rate = pricing_input.price_exchange_rate.unwrap_or(buffered_rate);
        let has_form_price = desired_price_count(&pricing_input, exchange_rate) > 0;
        if !has_form_price {
            let price_queries = ProductPriceQueries::new(state.db.pool());
            let has_db_price = price_queries.get_reference_price(id, "aliexpress").await
                .ok().flatten().map(|p| p.sale_price_cny > 0.0).unwrap_or(false)
                || price_queries.get_reference_price(id, "alibaba").await
                    .ok().flatten().map(|p| p.sale_price_cny > 0.0).unwrap_or(false)
                || price_queries.get_reference_price(id, "website").await
                    .ok().flatten().map(|p| p.sale_price_cny > 0.0).unwrap_or(false);
            if !has_db_price {
                Some(3) // 草稿
            } else {
                form.status
            }
        } else {
            form.status
        }
    } else {
        form.status
    };

    // 更新产品基本信息
    let update_req = UpdateProductRequest {
        name: Some(form.name.clone()),
        model: form.model.clone(),
        name_en: form.name_en.clone(),
        slug: None,
        category_id: resolved_category_id,
        brand_id: resolved_brand_id,
        supplier_id: form.supplier_id,
        weight: form.weight,
        volume: form.volume,
        description: form.description.clone(),
        description_en: None,
        specifications: None,
        main_image: None,
        images: None,
        status: effective_status,
        is_featured: Some(form.is_featured.is_some()),
        is_new: Some(form.is_new.is_some()),
        notes: form.notes.clone(),
        unit: form.unit.clone(),
    };

    match state.db.pool().begin().await {
        Ok(mut tx) => {
            let exchange_rate = pricing_input.price_exchange_rate.unwrap_or(buffered_rate);
            if let Err(e) = queries.update_in_tx(&mut tx, id, &update_req).await {
                info!("Failed to update product: {}", e);
                let _ = tx.rollback().await;
                let error_content = format!(r#"<div class="text-center py-12">
                    <p class="text-4xl mb-4">❌</p>
                    <p class="text-gray-600 mb-4">更新产品失败：{}</p>
                    <a href="/products/{}" class="text-blue-600 hover:text-blue-800">返回产品详情</a>
                </div>"#, e, id);
                return Err(Html(error_content));
            }

            if let Err(e) = sync_reference_pricing(&mut tx, id, &pricing_input, exchange_rate).await {
                info!("Failed to update product pricing data: {}", e);
                let _ = tx.rollback().await;
                let error_content = format!(r#"<div class="text-center py-12">
                    <p class="text-4xl mb-4">❌</p>
                    <p class="text-gray-600 mb-4">更新价格/成本失败：{}</p>
                    <a href="/products/{}/edit" class="text-blue-600 hover:text-blue-800">返回编辑</a>
                </div>"#, e, id);
                return Err(Html(error_content));
            }

            if let Err(e) = tx.commit().await {
                info!("Failed to commit product update transaction: {}", e);
                let error_content = format!(r#"<div class="text-center py-12">
                    <p class="text-4xl mb-4">❌</p>
                    <p class="text-gray-600 mb-4">更新产品失败：{}</p>
                    <a href="/products/{}/edit" class="text-blue-600 hover:text-blue-800">返回编辑</a>
                </div>"#, e, id);
                return Err(Html(error_content));
            }

            info!("Product updated: id={}", id);
            Ok(Redirect::to(&format!("/products/{}", id)))
        }
        Err(e) => {
            info!("Failed to start product update transaction: {}", e);
            let error_content = format!(r#"<div class="text-center py-12">
                <p class="text-4xl mb-4">❌</p>
                <p class="text-gray-600 mb-4">更新产品失败：{}</p>
                <a href="/products/{}/edit" class="text-blue-600 hover:text-blue-800">返回编辑</a>
            </div>"#, e, id);
            Err(Html(error_content))
        }
    }
}

// ============================================================================
// 产品导出 / 导入
// ============================================================================

/// 导出所有产品为 Excel
pub async fn product_export_handler(
    State(state): State<AppState>,
    Extension(_auth_user): Extension<AuthUser>,
) -> impl IntoResponse {
    use rust_xlsxwriter::{Format, Workbook};
    use axum::http::{header, StatusCode};

    // 获取缓冲汇率用于 USD 计算
    let rate = {
        let eq = ExchangeRateQueries::new(state.db.pool());
        eq.get_latest_rate("USD", "CNY").await
            .ok().flatten().map(|r| r.rate - 0.05).unwrap_or(6.84)
    };

    let queries = ProductQueries::new(state.db.pool());
    // 最多导出 10000 条
    let products = queries.list(1, 10000, None, None, None, None, None, None, None)
        .await.map(|r| r.items).unwrap_or_default();

    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();

    // 表头
    let headers = [
        "产品编码", "产品名称", "英文名称", "供应商",
        "成本(CNY)", "成本(USD)", "参考汇率",
        "售价(CNY)", "售价(USD)",
        "利润(USD)", "利润率%",
        "库存", "状态", "分类", "品牌",
    ];
    let bold = Format::new().set_bold();
    for (col, h) in headers.iter().enumerate() {
        ws.write_with_format(0, col as u16, *h, &bold).ok();
    }

    // 数据行
    for (row, p) in products.iter().enumerate() {
        let r = (row + 1) as u32;
        let status = match p.status { 1 => "上架", 2 => "下架", _ => "草稿" };
        let cost_usd = p.cost_cny.map(|c| c / rate);

        ws.write(r, 0, p.product_code.as_str()).ok();
        ws.write(r, 1, p.name.as_str()).ok();
        ws.write(r, 2, "").ok();  // name_en not in list item
        ws.write(r, 3, p.supplier_name.as_deref().unwrap_or("")).ok();
        if let Some(v) = p.cost_cny { ws.write(r, 4, v).ok(); }
        if let Some(v) = cost_usd { ws.write(r, 5, (v * 100.0).round() / 100.0).ok(); }
        ws.write(r, 6, (rate * 100.0).round() / 100.0).ok();
        if let Some(v) = p.sale_price_cny { ws.write(r, 7, v).ok(); }
        if let Some(v) = p.sale_price_usd { ws.write(r, 8, v).ok(); }
        if let Some(v) = p.profit_usd { ws.write(r, 9, v).ok(); }
        if let Some(v) = p.profit_margin { ws.write(r, 10, (v * 100.0 * 10.0).round() / 10.0).ok(); }
        if let Some(v) = p.stock_quantity { ws.write(r, 11, v).ok(); }
        ws.write(r, 12, status).ok();
        ws.write(r, 13, p.category_name.as_deref().unwrap_or("")).ok();
        ws.write(r, 14, p.brand_name.as_deref().unwrap_or("")).ok();
    }

    let buf = match wb.save_to_buffer() {
        Ok(b) => b,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR,
                format!("导出失败: {}", e)).into_response();
        }
    };

    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"),
            (header::CONTENT_DISPOSITION, "attachment; filename=\"products_export.xlsx\""),
        ],
        buf,
    ).into_response()
}

/// 下载导入模板
pub async fn product_import_template_handler(
    Extension(_auth_user): Extension<AuthUser>,
) -> impl IntoResponse {
    use rust_xlsxwriter::{Format, Workbook};
    use axum::http::{header, StatusCode};

    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();

    let bold = Format::new().set_bold();
    let headers = [
        "产品名称*", "英文名称", "供应商名称",
        "成本(CNY)*", "售价(CNY)", "售价(USD)",
        "状态(上架/下架/草稿)", "分类", "品牌", "备注",
    ];
    for (col, h) in headers.iter().enumerate() {
        ws.write_with_format(0, col as u16, *h, &bold).ok();
    }
    // 示例行
    let example = ["示例产品", "Example Product", "COMFAST",
        "100", "200", "29.00",
        "上架", "路由器", "COMFAST", "备注信息"];
    for (col, v) in example.iter().enumerate() {
        ws.write(1, col as u16, *v).ok();
    }

    let buf = match wb.save_to_buffer() {
        Ok(b) => b,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("生成失败: {}", e)).into_response(),
    };

    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"),
            (header::CONTENT_DISPOSITION, "attachment; filename=\"products_import_template.xlsx\""),
        ],
        buf,
    ).into_response()
}

/// 批量导入产品
pub async fn product_import_handler(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    mut multipart: axum::extract::Multipart,
) -> impl IntoResponse {
    use calamine::{open_workbook_from_rs, Reader, Xlsx};
    use std::io::Cursor;

    let user = get_user_from_extension(&auth_user);

    // 读取上传文件
    let file_bytes = loop {
        match multipart.next_field().await {
            Ok(Some(field)) if field.name() == Some("file") => {
                match field.bytes().await {
                    Ok(b) => break b,
                    Err(e) => {
                        let content = format!(r#"<div class="p-6 text-red-600">读取文件失败: {}</div>"#, e);
                        return render_layout("导入结果", "products", Some(user), &content);
                    }
                }
            }
            Ok(Some(_)) => continue,
            Ok(None) => {
                let content = r#"<div class="p-6 text-red-600">未收到文件</div>"#;
                return render_layout("导入结果", "products", Some(user), content);
            }
            Err(e) => {
                let content = format!(r#"<div class="p-6 text-red-600">上传错误: {}</div>"#, e);
                return render_layout("导入结果", "products", Some(user), &content);
            }
        }
    };

    // 解析 Excel
    let cursor = Cursor::new(file_bytes.as_ref());
    let mut workbook: Xlsx<_> = match open_workbook_from_rs(cursor) {
        Ok(wb) => wb,
        Err(e) => {
            let content = format!(r#"<div class="p-6 text-red-600">解析 Excel 失败: {}</div>"#, e);
            return render_layout("导入结果", "products", Some(user), &content);
        }
    };

    let sheet_name = workbook.sheet_names().first().cloned().unwrap_or_default();
    let range = match workbook.worksheet_range(&sheet_name) {
        Ok(r) => r,
        Err(e) => {
            let content = format!(r#"<div class="p-6 text-red-600">读取工作表失败: {}</div>"#, e);
            return render_layout("导入结果", "products", Some(user), &content);
        }
    };

    // 获取缓冲汇率
    let buffered_rate = {
        let eq = ExchangeRateQueries::new(state.db.pool());
        let r = eq.get_latest_rate("USD", "CNY").await
            .ok().flatten().map(|r| r.rate).unwrap_or(7.2);
        ((r - 0.05) * 100.0).round() / 100.0
    };

    let queries = ProductQueries::new(state.db.pool());
    let supplier_queries = SupplierQueries::new(state.db.pool());
    let cost_queries = ProductCostQueries::new(state.db.pool());
    let price_queries = ProductPriceQueries::new(state.db.pool());

    let mut ok_count = 0usize;
    let mut skip_count = 0usize;
    let mut errors: Vec<String> = Vec::new();

    // 跳过第一行（表头），从第二行开始
    for (row_idx, row) in range.rows().skip(1).enumerate() {
        let row_num = row_idx + 2;

        let get_str = |col: usize| -> String {
            row.get(col)
                .map(|c| c.to_string().trim().to_string())
                .unwrap_or_default()
        };
        let get_f64 = |col: usize| -> Option<f64> {
            row.get(col).and_then(|c| {
                let s = c.to_string();
                let s = s.trim();
                if s.is_empty() { None } else { s.parse::<f64>().ok() }
            })
        };

        let name = get_str(0);
        if name.is_empty() { skip_count += 1; continue; }

        // 列: 产品名称* | 英文名称 | 供应商名称 | 成本(CNY)* | 售价(CNY) | 售价(USD) | 状态 | 分类 | 品牌 | 备注
        let name_en = get_str(1);
        let supplier_name = get_str(2);
        let cost_cny = get_f64(3);
        let sale_price_cny = get_f64(4);
        let sale_price_usd = get_f64(5);
        let status_str = get_str(6);
        let _category = get_str(7);
        let _brand = get_str(8);
        let notes = get_str(9);

        let status: i64 = match status_str.as_str() {
            "上架" => 1,
            "下架" => 2,
            _ => 3, // 草稿
        };

        // 查找供应商 ID
        let supplier_id: Option<i64> = if !supplier_name.is_empty() {
            supplier_queries.list(1, 100, Some(1), None, None).await
                .ok().map(|r| r.items)
                .unwrap_or_default()
                .into_iter()
                .find(|s| s.name == supplier_name)
                .map(|s| s.id)
        } else {
            None
        };

        let req = CreateProductRequest {
            product_code: None,
            name: name.clone(),
            model: None,
            name_en: if name_en.is_empty() { None } else { Some(name_en) },
            slug: None,
            category_id: None,
            brand_id: None,
            supplier_id,
            weight: None,
            volume: None,
            description: None,
            description_en: None,
            specifications: None,
            main_image: None,
            images: None,
            status: Some(status),
            is_featured: Some(false),
            is_new: Some(false),
            notes: if notes.is_empty() { None } else { Some(notes) },
            unit: None,
        };

        let product = match queries.create(&req).await {
            Ok(p) => p,
            Err(e) => {
                errors.push(format!("第 {} 行「{}」创建失败: {}", row_num, name, e));
                continue;
            }
        };

        // 创建参考成本
        if let Some(cny) = cost_cny {
            if cny > 0.0 {
                let cost_req = CreateProductCostRequest {
                    product_id: product.id,
                    supplier_id,
                    cost_cny: cny,
                    cost_usd: Some(cny / buffered_rate),
                    currency: Some("CNY".to_string()),
                    exchange_rate: Some(buffered_rate),
                    profit_margin: Some(0.15),
                    platform_fee_rate: Some(0.02),
                    platform_fee: None,
                    sale_price_usd: None,
                    quantity: Some(1),
                    purchase_order_id: None,
                    is_reference: Some(true),
                    effective_date: None,
                    notes: None,
                };
                let _ = cost_queries.create(&cost_req).await;
            }
        }

        // 创建参考售价（alibaba 平台）
        if let Some(sprice_cny) = sale_price_cny {
            if sprice_cny > 0.0 {
                use cicierp_models::product::CreateProductPriceRequest;
                let sprice_usd = sale_price_usd.unwrap_or_else(|| sprice_cny / buffered_rate);
                for platform in &["alibaba", "aliexpress", "website"] {
                    let price_req = CreateProductPriceRequest {
                        product_id: product.id,
                        platform: Some(platform.to_string()),
                        sale_price_cny: sprice_cny,
                        sale_price_usd: Some((sprice_usd * 100.0).round() / 100.0),
                        exchange_rate: Some(buffered_rate),
                        profit_margin: Some(0.15),
                        platform_fee_rate: Some(0.02),
                        platform_fee: None,
                        is_reference: Some(*platform == "alibaba"),
                        effective_date: None,
                        notes: None,
                        pricing_mode: None,
                        input_currency: None,
                        reference_platform: None,
                        adjustment_type: None,
                        adjustment_value: None,
                    };
                    let _ = price_queries.create(&price_req).await;
                }
            }
        }

        ok_count += 1;
    }

    let error_html = if errors.is_empty() {
        String::new()
    } else {
        let items: String = errors.iter().map(|e| format!("<li class='text-red-600'>{}</li>", e)).collect();
        format!(r#"<div class="mt-4"><h4 class="font-semibold text-red-700 mb-2">错误详情：</h4><ul class="list-disc list-inside space-y-1 text-sm">{}</ul></div>"#, items)
    };

    let content = format!(r#"
<div class="max-w-lg mx-auto mt-8">
    <div class="bg-white rounded-xl shadow-sm border border-gray-100 p-6">
        <h2 class="text-xl font-bold text-gray-800 mb-4">导入完成</h2>
        <div class="grid grid-cols-3 gap-4 mb-4">
            <div class="text-center p-3 bg-green-50 rounded-lg">
                <div class="text-2xl font-bold text-green-600">{ok_count}</div>
                <div class="text-sm text-gray-600">成功导入</div>
            </div>
            <div class="text-center p-3 bg-yellow-50 rounded-lg">
                <div class="text-2xl font-bold text-yellow-600">{skip_count}</div>
                <div class="text-sm text-gray-600">跳过（空行）</div>
            </div>
            <div class="text-center p-3 bg-red-50 rounded-lg">
                <div class="text-2xl font-bold text-red-600">{err_count}</div>
                <div class="text-sm text-gray-600">失败</div>
            </div>
        </div>
        {error_html}
        <div class="mt-6">
            <a href="/products" class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700">返回产品列表</a>
        </div>
    </div>
</div>"#,
        ok_count = ok_count,
        skip_count = skip_count,
        err_count = errors.len(),
        error_html = error_html,
    );

    render_layout("导入结果", "products", Some(user), &content)
}

// ============================================================================
// 订单管理
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct OrdersQuery {
    pub page: Option<u32>,
    #[serde(default, deserialize_with = "empty_string_as_none_i64")]
    pub status: Option<i64>,
    pub currency: Option<String>,
    pub platform: Option<String>,
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
        query.platform.as_deref(), // platform
        None, // date_from
        None, // date_to
        None, // keyword
        query.currency.as_deref(), // currency
    ).await.unwrap_or_else(|_| PagedResponse::new(vec![], page, page_size, 0));

    // 获取平台统计和当前筛选统计
    let filter_stats = queries.filter_stats(
        query.status,
        query.platform.as_deref(),
        query.currency.as_deref(),
    ).await.unwrap_or_default();
    let platform_counts = queries.platform_counts().await.unwrap_or_default();

    // 当前币种 / 平台
    let current_currency = query.currency.as_deref();
    let current_platform = query.platform.as_deref();

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
        if let Some(p) = current_platform {
            params.push(format!("platform={}", p));
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

    // 生成平台筛选链接
    let platform_filter = |current: Option<&str>, target: Option<&str>, text: &str, count: i64| -> String {
        let is_active = current == target;
        let mut params = Vec::new();
        if let Some(t) = target {
            params.push(format!("platform={}", t));
        }
        if let Some(s) = query.status {
            params.push(format!("status={}", s));
        }
        if let Some(c) = current_currency {
            params.push(format!("currency={}", c));
        }
        let url = if params.is_empty() {
            "/orders".to_string()
        } else {
            format!("/orders?{}", params.join("&"))
        };
        let label = if count > 0 {
            format!("{} <span style='font-size:11px;opacity:0.7;'>({})</span>", text, count)
        } else {
            text.to_string()
        };
        if is_active {
            format!(r#"<a href="{}" class="px-3 py-1 text-sm rounded-full transition-colors bg-purple-100 text-purple-700">{}</a>"#, url, label)
        } else {
            format!(r#"<a href="{}" class="px-3 py-1 text-sm rounded-full transition-colors text-gray-600 hover:bg-gray-100">{}</a>"#, url, label)
        }
    };

    // 平台数量 map
    let mut platform_map: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    let mut total_platform_count = 0i64;
    for ps in &platform_counts {
        platform_map.insert(ps.platform.clone(), ps.order_count);
        total_platform_count += ps.order_count;
    }
    let ali_count = platform_map.get("alibaba").or_else(|| platform_map.get("Alibaba")).copied().unwrap_or(0);
    let ae_count = platform_map.get("aliexpress").or_else(|| platform_map.get("AliExpress")).copied().unwrap_or(0);

    // 统计栏
    let profit_rate_str = if filter_stats.total_amount > 0.0 {
        format!("{:.1}%", filter_stats.total_profit / filter_stats.total_amount * 100.0)
    } else {
        "-".to_string()
    };
    let loss_badge = if filter_stats.loss_count > 0 {
        format!(r#" <span style="color:#dc2626;font-size:12px;">⚠️{}单亏损</span>"#, filter_stats.loss_count)
    } else {
        String::new()
    };
    let stats_bar = format!(
        r#"<div style="display:flex;gap:12px;margin-bottom:16px;flex-wrap:wrap;">
  <div style="flex:1;min-width:120px;background:#f0f9ff;border:1px solid #bae6fd;border-radius:8px;padding:12px;text-align:center;">
    <div style="font-size:22px;font-weight:700;color:#0284c7;">{}</div>
    <div style="font-size:11px;color:#64748b;margin-top:2px;">订单总数</div>
  </div>
  <div style="flex:1;min-width:120px;background:#f0fdf4;border:1px solid #bbf7d0;border-radius:8px;padding:12px;text-align:center;">
    <div style="font-size:22px;font-weight:700;color:#16a34a;">{:.2}</div>
    <div style="font-size:11px;color:#64748b;margin-top:2px;">总销售额</div>
  </div>
  <div style="flex:1;min-width:120px;background:#fefce8;border:1px solid #fde68a;border-radius:8px;padding:12px;text-align:center;">
    <div style="font-size:22px;font-weight:700;color:#ca8a04;">{:.2}</div>
    <div style="font-size:11px;color:#64748b;margin-top:2px;">总毛利{}</div>
  </div>
  <div style="flex:1;min-width:120px;background:#fdf4ff;border:1px solid #e9d5ff;border-radius:8px;padding:12px;text-align:center;">
    <div style="font-size:22px;font-weight:700;color:#9333ea;">{}</div>
    <div style="font-size:11px;color:#64748b;margin-top:2px;">平均毛利率</div>
  </div>
</div>"#,
        filter_stats.total_orders,
        filter_stats.total_amount,
        filter_stats.total_profit,
        loss_badge,
        profit_rate_str
    );

    // 生成状态筛选链接（保留币种/平台参数）
    let status_filter = |current: Option<i64>, target: Option<i64>, text: &str| -> String {
        let is_active = current == target;
        let mut params = Vec::new();
        if let Some(t) = target {
            params.push(format!("status={}", t));
        }
        if let Some(c) = current_currency {
            params.push(format!("currency={}", c));
        }
        if let Some(p) = current_platform {
            params.push(format!("platform={}", p));
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
        let delete_btn = format!(r#"<button onclick="deleteItem('/api/v1/orders/{}', '确认删除订单「{}」？', true)" class="text-red-600 hover:text-red-800 text-sm ml-2">删除</button>"#, order.id, order.order_code);
        let action_buttons = match order.order_status {
            1 => {
                // 未成交 - 可编辑、可下载 PI
                format!(r#"
                    <a href="/orders/{}" class="text-blue-600 hover:text-blue-800 text-sm">详情</a>
                    <a href="/orders/{}/edit" class="text-orange-600 hover:text-orange-800 text-sm ml-2">编辑</a>
                    <a href="/api/v1/orders/{}/download-pi" class="text-green-600 hover:text-green-800 text-sm ml-2" target="_blank">下载PI</a>
                    {}
                "#, order.id, order.id, order.id, delete_btn)
            }
            2 => {
                // 价格锁定 - 可下载 PI
                format!(r#"
                    <a href="/orders/{}" class="text-blue-600 hover:text-blue-800 text-sm">详情</a>
                    <a href="/api/v1/orders/{}/download-pi" class="text-green-600 hover:text-green-800 text-sm ml-2" target="_blank">下载PI</a>
                    {}
                "#, order.id, order.id, delete_btn)
            }
            3 | 4 | 5 => {
                // 已付款/已发货/已收货 - 可下载 CI
                format!(r#"
                    <a href="/orders/{}" class="text-blue-600 hover:text-blue-800 text-sm">详情</a>
                    <a href="/api/v1/orders/{}/download-ci" class="text-green-600 hover:text-green-800 text-sm ml-2" target="_blank">下载CI</a>
                    {}
                "#, order.id, order.id, delete_btn)
            }
            _ => {
                format!(r#"<a href="/orders/{}" class="text-blue-600 hover:text-blue-800 text-sm">详情</a>{}"#, order.id, delete_btn)
            }
        };

        let profit_cell = match order.profit_amount {
            None => r#"<span class="text-gray-400">-</span>"#.to_string(),
            Some(p) if p < 0.0 => {
                let rate_str = order.profit_rate
                    .map(|r| format!(" ({:.1}%)", r))
                    .unwrap_or_default();
                format!(
                    r#"<span class="text-red-600 font-medium">{:.2}{}</span> <span class="ml-1 px-1 py-0.5 text-xs bg-red-100 text-red-600 rounded">⚠️亏损</span>"#,
                    p, rate_str
                )
            }
            Some(p) => {
                let rate_str = order.profit_rate
                    .map(|r| format!(" ({:.1}%)", r))
                    .unwrap_or_default();
                format!(
                    r#"<span class="text-green-600 font-medium">{:.2}{}</span>"#,
                    p, rate_str
                )
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
                    <span class="font-medium text-gray-800">{}{:.2}</span>
                </td>
                <td class="px-4 sm:px-6 py-4 text-right">
                    {}
                </td>
                <td class="px-4 sm:px-6 py-4 text-center">
                    <span class="px-2 py-1 text-xs font-medium rounded-full {}">{}</span>
                </td>
                <td class="px-4 sm:px-6 py-4 text-center">
                    <div class="flex items-center justify-center gap-2">{}</div>
                </td>
            </tr>"#,
            order.order_code,
            order.platform,
            order.customer_name.as_deref().unwrap_or("-"),
            if order.currency.to_uppercase() == "USD" { "$" } else { "¥" },
            order.total_amount,
            profit_cell,
            status_class,
            status_text,
            action_buttons
        )
    }).collect();

    let rows = if rows.is_empty() {
        r#"<tr><td colspan="7" class="px-6 py-12 text-center"><div class="text-gray-500"><p class="text-4xl mb-2">📋</p><p>暂无订单数据</p></div></td></tr>"#.to_string()
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
            if let Some(ref pl) = query.platform {
                params.push(format!("platform={}", pl));
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
        <a href="/orders/import/template" class="inline-flex items-center justify-center gap-2 px-4 py-2 bg-gray-100 text-gray-700 rounded-lg hover:bg-gray-200 transition-colors">
            <span>↓</span><span>订单模板</span>
        </a>
        <button onclick="document.getElementById('orderImportModal').classList.remove('hidden')"
                class="inline-flex items-center justify-center gap-2 px-4 py-2 bg-emerald-600 text-white rounded-lg hover:bg-emerald-700 transition-colors">
            <span>📥</span><span>导入订单</span>
        </button>
        <a href="/api/v1/orders/export" class="inline-flex items-center justify-center gap-2 px-4 py-2 bg-gray-600 text-white rounded-lg hover:bg-gray-700 transition-colors">
            <span>📊</span><span>导出Excel</span>
        </a>
        <a href="/orders/new" class="inline-flex items-center justify-center gap-2 px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors">
            <span>+</span><span>新建订单</span>
        </a>
    </div>
</div>

<!-- 订单导入弹窗 -->
<div id="orderImportModal" class="hidden fixed inset-0 z-50 flex items-center justify-center bg-black/50">
    <div class="bg-white rounded-xl shadow-xl w-full max-w-lg mx-4 p-6">
        <div class="flex items-center justify-between mb-4">
            <h3 class="text-lg font-semibold text-gray-800">📥 批量导入订单</h3>
            <button onclick="document.getElementById('orderImportModal').classList.add('hidden')"
                    class="text-gray-400 hover:text-gray-600 text-2xl leading-none">&times;</button>
        </div>
        <div class="text-sm text-gray-600 mb-4 space-y-2 bg-blue-50 rounded-lg p-3">
            <p class="font-medium text-gray-800">统一模板列顺序（支持 Alibaba / AliExpress 混合导入）：</p>
            <p>日期 / 订单号* / 客户姓名 / 产品名称* / 数量* / 平台*(alibaba/aliexpress) / 单价* / 币种(USD/RMB) / 成本 / 备注</p>
            <p class="text-xs text-gray-500">• Alibaba：单价 USD，自动换算 CNY<br>• AliExpress：单价 RMB，自动换算 USD</p>
            <p><a href="/orders/import/template" class="text-blue-600 hover:underline">↓ 下载统一订单模板</a></p>
        </div>
        <form action="/orders/import" method="POST" enctype="multipart/form-data">
            <div class="mb-4">
                <label class="block text-sm font-medium text-gray-700 mb-2">选择文件 (.xlsx)</label>
                <input type="file" name="file" accept=".xlsx"
                       class="block w-full text-sm text-gray-700 border border-gray-300 rounded-lg px-3 py-2 cursor-pointer" required>
            </div>
            <div class="flex justify-end gap-3">
                <button type="button" onclick="document.getElementById('orderImportModal').classList.add('hidden')"
                        class="px-4 py-2 bg-gray-100 text-gray-700 rounded-lg hover:bg-gray-200">取消</button>
                <button type="submit" class="px-4 py-2 bg-emerald-600 text-white rounded-lg hover:bg-emerald-700">开始导入</button>
            </div>
        </form>
    </div>
</div>

<!-- 聚合统计栏 -->
{}

<!-- 状态说明 -->
<div class="bg-blue-50 rounded-xl border border-blue-100 p-4 mb-6">
    <div class="flex flex-wrap items-center gap-4 text-sm">
        <span class="text-gray-600">状态说明:</span>
        <span class="text-gray-700"><strong>未成交/价格锁定</strong> → 可下载 PI（形式发票）</span>
        <span class="text-gray-700"><strong>已付款/已发货/已收货</strong> → 可下载 CI（商业发票）</span>
    </div>
</div>

<!-- 平台筛选 -->
<div class="bg-white rounded-xl shadow-sm border border-gray-100 p-4 mb-4 overflow-x-auto">
    <div class="flex items-center gap-3 sm:gap-4 min-w-max">
        <span class="text-sm text-gray-500 whitespace-nowrap">平台:</span>
        <div class="flex items-center gap-2">
            {}
            {}
            {}
            {}
        </div>
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
                    <th class="px-4 sm:px-6 py-4 text-right text-sm font-semibold text-gray-700">毛利</th>
                    <th class="px-4 sm:px-6 py-4 text-center text-sm font-semibold text-gray-700">状态</th>
                    <th class="px-4 sm:px-6 py-4 text-center text-sm font-semibold text-gray-700">操作</th>
                </tr>
            </thead>
            <tbody class="divide-y divide-gray-100">{}</tbody>
        </table>
    </div>
    {}
</div>"#,
        stats_bar,
        platform_filter(current_platform, None, "全部", total_platform_count),
        platform_filter(current_platform, Some("alibaba"), "Alibaba", ali_count),
        platform_filter(current_platform, Some("aliexpress"), "AliExpress", ae_count),
        platform_filter(current_platform, Some("other"), "其他",
            total_platform_count.saturating_sub(ali_count).saturating_sub(ae_count)),
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

/// GET /orders/import/template — 下载统一订单导入模板
pub async fn order_import_template_handler(
    Extension(_auth_user): Extension<AuthUser>,
) -> impl axum::response::IntoResponse {
    use rust_xlsxwriter::{Format, FormatAlign, Workbook};
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    let _ = worksheet.set_name("订单导入模板");

    let header_fmt = Format::new()
        .set_bold()
        .set_align(FormatAlign::Center)
        .set_background_color(rust_xlsxwriter::Color::RGB(0x4F81BD))
        .set_font_color(rust_xlsxwriter::Color::White);

    let headers = [
        "日期(YYYY-MM-DD)", "订单号*", "客户姓名", "产品名称*",
        "数量*", "平台*(alibaba/aliexpress)", "单价*", "币种(USD/RMB)", "成本", "备注"
    ];
    for (col, h) in headers.iter().enumerate() {
        let _ = worksheet.write_with_format(0, col as u16, *h, &header_fmt);
        let _ = worksheet.set_column_width(col as u16, 22.0);
    }

    // 示例行 - Alibaba (USD)
    let _ = worksheet.write(1, 0, "2025-08-14");
    let _ = worksheet.write(1, 1, "ORD-20250814-001");
    let _ = worksheet.write(1, 2, "Tokpasoua Haba");
    let _ = worksheet.write(1, 3, "COMFAST CF-EW85");
    let _ = worksheet.write(1, 4, 3u32);
    let _ = worksheet.write(1, 5, "alibaba");
    let _ = worksheet.write(1, 6, 46.0f64);
    let _ = worksheet.write(1, 7, "USD");
    let _ = worksheet.write(1, 8, 45.13f64);
    let _ = worksheet.write(1, 9, "示例-Alibaba订单");

    // 示例行 - AliExpress (RMB)
    let _ = worksheet.write(2, 0, "2025-11-03");
    let _ = worksheet.write(2, 1, "ORD-20251103-001");
    let _ = worksheet.write(2, 2, "Oleksandr Maltsev");
    let _ = worksheet.write(2, 3, "MOES Plug-in Display");
    let _ = worksheet.write(2, 4, 1u32);
    let _ = worksheet.write(2, 5, "aliexpress");
    let _ = worksheet.write(2, 6, 51.62f64);
    let _ = worksheet.write(2, 7, "RMB");
    let _ = worksheet.write(2, 8, 86.0f64);
    let _ = worksheet.write(2, 9, "示例-AliExpress订单");

    let buf = workbook.save_to_buffer().unwrap_or_default();
    (
        [
            (axum::http::header::CONTENT_TYPE, "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"),
            (axum::http::header::CONTENT_DISPOSITION, "attachment; filename=\"orders_import_template.xlsx\""),
        ],
        buf,
    )
}

/// POST /orders/import — 批量导入订单（统一模板，支持 Alibaba/AliExpress）
pub async fn order_import_handler(
    Extension(auth_user): Extension<AuthUser>,
    State(state): State<AppState>,
    mut multipart: axum::extract::Multipart,
) -> Html<String> {
    let user = get_user_from_extension(&auth_user);

    let mut file_data: Option<Vec<u8>> = None;
    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() == Some("file") {
            if let Ok(bytes) = field.bytes().await {
                file_data = Some(bytes.to_vec());
                break;
            }
        }
    }

    let file_bytes = match file_data {
        Some(b) if !b.is_empty() => b,
        _ => {
            let content = r#"<div class="max-w-2xl mx-auto p-6"><div class="bg-red-50 border border-red-200 rounded-lg p-4 text-red-700">错误：未收到文件，请重试。<br><a href="/orders" class="text-blue-600 hover:underline">返回订单列表</a></div></div>"#;
            return render_layout("导入失败", "orders", Some(user), content);
        }
    };

    // 保存上传文件
    let uploads_dir = "/home/wxy/data/ciciERP/data/uploads";
    let _ = std::fs::create_dir_all(uploads_dir);
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs()).unwrap_or(0);
    let upload_path = format!("{}/unified_import_{}.xlsx", uploads_dir, ts);
    if let Err(e) = std::fs::write(&upload_path, &file_bytes) {
        let content = format!(r#"<div class="max-w-2xl mx-auto p-6"><div class="bg-red-50 border border-red-200 rounded-lg p-4 text-red-700">保存文件失败: {}<br><a href="/orders" class="text-blue-600 hover:underline">返回</a></div></div>"#, e);
        return render_layout("导入失败", "orders", Some(user), &content);
    }

    // 用统一脚本处理（分拆到 Ali / AE 脚本）
    let script_path = "/home/wxy/data/ciciERP/scripts/import_orders_unified.py";
    let db_path = "/home/wxy/data/ciciERP/data/cicierp.db";
    let output = std::process::Command::new("python3")
        .arg(script_path)
        .arg(&upload_path)
        .arg(db_path)
        .output();
    let _ = std::fs::remove_file(&upload_path);

    let content = match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            let summary = stdout.lines().rev()
                .find(|l| l.starts_with('{'))
                .and_then(|l| serde_json::from_str::<serde_json::Value>(l).ok());

            if out.status.success() {
                let (orders_added, customers_added, skipped) = if let Some(j) = &summary {
                    (j["orders_added"].as_i64().unwrap_or(0),
                     j["customers_added"].as_i64().unwrap_or(0),
                     j["skipped"].as_i64().unwrap_or(0))
                } else { (0, 0, 0) };
                format!(r#"<div class="max-w-2xl mx-auto p-6">
  <div class="bg-green-50 border border-green-200 rounded-lg p-4 mb-4">
    <h3 class="text-lg font-semibold text-green-700 mb-3">✅ 导入成功</h3>
    <div class="grid grid-cols-3 gap-4 text-center">
      <div><p class="text-2xl font-bold text-green-600">{}</p><p class="text-sm text-gray-600">新增订单</p></div>
      <div><p class="text-2xl font-bold text-blue-600">{}</p><p class="text-sm text-gray-600">新增客户</p></div>
      <div><p class="text-2xl font-bold text-gray-500">{}</p><p class="text-sm text-gray-600">跳过记录</p></div>
    </div>
  </div>
  <details class="mb-4"><summary class="cursor-pointer text-sm text-gray-500">查看完整输出</summary>
    <pre class="bg-gray-50 border rounded p-3 text-xs mt-2 overflow-x-auto">{}</pre></details>
  <a href="/orders" class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 text-sm">查看订单列表</a>
</div>"#, orders_added, customers_added, skipped, html_escape(&stdout))
            } else {
                format!(r#"<div class="max-w-2xl mx-auto p-6">
  <div class="bg-red-50 border border-red-200 rounded-lg p-4 mb-4">
    <h3 class="text-lg font-semibold text-red-700 mb-2">❌ 导入失败</h3>
    <pre class="text-xs whitespace-pre-wrap">{}</pre>
  </div>
  <a href="/orders" class="px-4 py-2 bg-gray-100 text-gray-700 rounded-lg text-sm">返回</a>
</div>"#, html_escape(&format!("STDOUT:\n{}\nSTDERR:\n{}", stdout, stderr)))
            }
        }
        Err(e) => format!(r#"<div class="max-w-2xl mx-auto p-6"><div class="bg-red-50 border border-red-200 rounded-lg p-4 text-red-700">执行脚本失败: {}<br><a href="/orders">返回</a></div></div>"#, e),
    };

    render_layout("导入订单结果", "orders", Some(user), &content)
}

/// GET /orders/import/ae — AliExpress 订单导入页面
pub async fn order_import_ae_page(
    Extension(auth_user): Extension<AuthUser>,
    State(_state): State<AppState>,
) -> Html<String> {
    let user = get_user_from_extension(&auth_user);
    let content = r#"<div style="max-width:600px;margin:0 auto;">
  <h2 style="font-size:1.5rem;font-weight:700;margin-bottom:16px;">导入 AliExpress 订单</h2>
  <div style="background:#f0f9ff;border:1px solid #bae6fd;border-radius:8px;padding:16px;margin-bottom:20px;">
    <h4 style="margin:0 0 8px;font-weight:600;">Excel 文件格式要求</h4>
    <p style="margin:0;font-size:14px;color:#555;">列顺序：Date | Order No. | Client Name | Product | Qty | Order Amount (RMB) | Sales Unit Price (RMB) | Cost per Unit (RMB) | Gross Profit (RMB) | Loss Flag | Shipping Status</p>
  </div>
  <form method="POST" action="/orders/import/ae" enctype="multipart/form-data" style="background:#fff;border:1px solid #e2e8f0;border-radius:8px;padding:24px;">
    <div style="margin-bottom:16px;">
      <label style="display:block;margin-bottom:6px;font-weight:600;">选择 Excel 文件 (.xlsx)</label>
      <input type="file" name="file" accept=".xlsx,.xls" required style="width:100%;padding:8px;border:1px solid #cbd5e1;border-radius:6px;">
    </div>
    <button type="submit" style="background:#3b82f6;color:#fff;padding:10px 24px;border:none;border-radius:6px;cursor:pointer;font-size:14px;">开始导入</button>
    <a href="/orders" style="margin-left:12px;color:#64748b;text-decoration:none;">取消</a>
  </form>
</div>"#;
    render_layout("导入AE订单", "orders", Some(user), content)
}

/// POST /orders/import/ae — 处理 AliExpress 订单文件上传并执行导入
pub async fn order_import_ae_handler(
    Extension(auth_user): Extension<AuthUser>,
    State(_state): State<AppState>,
    mut multipart: axum::extract::Multipart,
) -> Html<String> {
    let user = get_user_from_extension(&auth_user);

    // 读取上传的文件数据
    let mut file_data: Option<Vec<u8>> = None;
    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() == Some("file") {
            if let Ok(bytes) = field.bytes().await {
                file_data = Some(bytes.to_vec());
                break;
            }
        }
    }

    let file_bytes = match file_data {
        Some(b) if !b.is_empty() => b,
        _ => {
            let content = r#"<div style="color:red;padding:20px;">错误：未收到文件，请重试。<br><a href="/orders/import/ae">返回</a></div>"#;
            return render_layout("导入失败", "orders", Some(user), content);
        }
    };

    // 保存上传文件到项目 data/uploads 目录
    let uploads_dir = "/home/wxy/data/ciciERP/data/uploads";
    if let Err(e) = std::fs::create_dir_all(uploads_dir) {
        let content = format!(r#"<div style="color:red;padding:20px;">创建目录失败: {}<br><a href="/orders/import/ae">返回</a></div>"#, e);
        return render_layout("导入失败", "orders", Some(user), &content);
    }

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let upload_path = format!("{}/ae_import_{}.xlsx", uploads_dir, ts);

    if let Err(e) = std::fs::write(&upload_path, &file_bytes) {
        let content = format!(r#"<div style="color:red;padding:20px;">保存文件失败: {}<br><a href="/orders/import/ae">返回</a></div>"#, e);
        return render_layout("导入失败", "orders", Some(user), &content);
    }

    // 运行 Python 导入脚本
    let script_path = "/home/wxy/data/ciciERP/scripts/import_orders_ae.py";
    let output = std::process::Command::new("python3")
        .arg(script_path)
        .arg(&upload_path)
        .output();

    // 清理上传的临时文件
    let _ = std::fs::remove_file(&upload_path);

    let content = match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);

            // 尝试从输出中解析 JSON 摘要
            let summary = stdout
                .lines()
                .rev()
                .find(|l| l.starts_with('{'))
                .and_then(|l| serde_json::from_str::<serde_json::Value>(l).ok());

            if out.status.success() {
                let (orders_added, customers_added, skipped) = if let Some(j) = &summary {
                    (
                        j["orders_added"].as_i64().unwrap_or(0),
                        j["customers_added"].as_i64().unwrap_or(0),
                        j["skipped"].as_i64().unwrap_or(0),
                    )
                } else {
                    (0, 0, 0)
                };
                format!(
                    r#"<div style="max-width:600px;margin:0 auto;">
  <div style="background:#f0fdf4;border:1px solid #bbf7d0;border-radius:8px;padding:20px;margin-bottom:20px;">
    <h3 style="color:#16a34a;margin:0 0 12px;">✅ 导入成功</h3>
    <table style="width:100%;border-collapse:collapse;">
      <tr><td style="padding:4px 0;color:#555;">新增订单</td><td style="font-weight:700;">{} 条</td></tr>
      <tr><td style="padding:4px 0;color:#555;">新增客户</td><td style="font-weight:700;">{} 个</td></tr>
      <tr><td style="padding:4px 0;color:#555;">跳过记录</td><td style="font-weight:700;">{} 条</td></tr>
    </table>
  </div>
  <details style="margin-bottom:16px;">
    <summary style="cursor:pointer;color:#64748b;font-size:13px;">查看完整输出</summary>
    <pre style="background:#f8fafc;border:1px solid #e2e8f0;border-radius:6px;padding:12px;font-size:12px;overflow-x:auto;margin-top:8px;">{}</pre>
  </details>
  <a href="/orders" style="background:#3b82f6;color:#fff;padding:10px 20px;border-radius:6px;text-decoration:none;font-size:14px;">查看订单列表</a>
  <a href="/orders/import/ae" style="margin-left:12px;color:#64748b;text-decoration:none;font-size:14px;">再次导入</a>
</div>"#,
                    orders_added, customers_added, skipped,
                    html_escape(&stdout)
                )
            } else {
                format!(
                    r#"<div style="max-width:600px;margin:0 auto;">
  <div style="background:#fef2f2;border:1px solid #fecaca;border-radius:8px;padding:20px;margin-bottom:20px;">
    <h3 style="color:#dc2626;margin:0 0 12px;">❌ 导入失败</h3>
    <pre style="font-size:12px;white-space:pre-wrap;margin:0;">{}</pre>
  </div>
  <a href="/orders/import/ae" style="background:#3b82f6;color:#fff;padding:10px 20px;border-radius:6px;text-decoration:none;font-size:14px;">重试</a>
</div>"#,
                    html_escape(&format!("STDOUT:\n{}\nSTDERR:\n{}", stdout, stderr))
                )
            }
        }
        Err(e) => {
            format!(
                r#"<div style="color:red;padding:20px;">执行脚本失败: {}<br><a href="/orders/import/ae">返回</a></div>"#,
                e
            )
        }
    };

    render_layout("导入AE订单结果", "orders", Some(user), &content)
}

/// 对 HTML 特殊字符转义
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
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
                <td class="px-4 sm:px-6 py-4 text-right">{}</td>
                <td class="px-4 sm:px-6 py-4 text-right font-medium">{}</td>
                <td class="px-4 sm:px-6 py-4 text-right text-gray-500">{}</td>
                <td class="px-4 sm:px-6 py-4 text-right text-gray-500">{}</td>
                <td class="px-4 sm:px-6 py-4 text-center">{}</td>
                <td class="px-4 sm:px-6 py-4 text-center">
                    <a href="/inventory/{}/adjust" class="text-blue-600 hover:text-blue-800 text-sm mr-2">调整</a>
                    <a href="/inventory/{}" class="text-gray-600 hover:text-gray-800 text-sm mr-2">详情</a>
                    <a href="/inventory/{}/movements" class="text-green-600 hover:text-green-800 text-sm mr-2">流水</a>
                    <button onclick="deleteItem('/api/v1/inventory/{}/delete', '确认删除该库存记录？', true)" class="text-red-600 hover:text-red-800 text-sm">删除</button>
                </td>
            </tr>"#,
            row_class,
            item.product_code,
            item.product_name,
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
            item.product_id,
            item.product_id,
            item.product_id,
            item.id
        )
    }).collect();

    let rows = if rows.is_empty() {
        r#"<tr><td colspan="8" class="px-6 py-12 text-center"><div class="text-gray-500"><p class="text-4xl mb-2">📊</p><p>暂无库存数据</p></div></td></tr>"#.to_string()
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
            <div><p class="text-xs sm:text-sm text-gray-500">产品数</p><p class="text-lg sm:text-xl font-bold text-gray-800">{}</p></div>
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
                    <th class="px-4 sm:px-6 py-4 text-left text-sm font-semibold text-gray-700">产品编码</th>
                    <th class="px-4 sm:px-6 py-4 text-left text-sm font-semibold text-gray-700">产品名称</th>
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
    #[serde(default, deserialize_with = "empty_string_as_none_i64")]
    status: Option<i64>,
    #[serde(default, deserialize_with = "empty_string_as_none_i64")]
    lead_status: Option<i64>,
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
        query.lead_status,
        None,
        query.keyword.as_deref(),
    ).await.unwrap_or_else(|_| PagedResponse::new(vec![], page, page_size, 0));

    let rows: String = result.items.iter().map(|c| {
        let status_badge = match c.status {
            1 => r#"<span class="px-2 py-1 text-xs bg-green-100 text-green-700 rounded-full">正常</span>"#,
            2 => r#"<span class="px-2 py-1 text-xs bg-yellow-100 text-yellow-700 rounded-full">冻结</span>"#,
            _ => r#"<span class="px-2 py-1 text-xs bg-red-100 text-red-700 rounded-full">黑名单</span>"#,
        };
        let lead_badge = match c.lead_status {
            2 => r#"<span class="px-2 py-1 text-xs bg-blue-100 text-blue-700 rounded-full">跟进中</span>"#,
            3 => r#"<span class="px-2 py-1 text-xs bg-green-100 text-green-700 rounded-full">已成交</span>"#,
            4 => r#"<span class="px-2 py-1 text-xs bg-red-100 text-red-700 rounded-full">已流失</span>"#,
            _ => r#"<span class="px-2 py-1 text-xs bg-gray-100 text-gray-600 rounded-full">潜在客户</span>"#,
        };
        format!(
            r#"<tr class="hover:bg-gray-50">
                <td class="px-4 sm:px-6 py-4"><span class="font-mono text-sm">{}</span></td>
                <td class="px-4 sm:px-6 py-4 font-medium">{}</td>
                <td class="px-4 sm:px-6 py-4">{}</td>
                <td class="px-4 sm:px-6 py-4">{}</td>
                <td class="px-4 sm:px-6 py-4 text-center">{}</td>
                <td class="px-4 sm:px-6 py-4 text-center">{}</td>
                <td class="px-4 sm:px-6 py-4 text-center">
                    <a href="/customers/{}" class="text-blue-600 hover:text-blue-800 text-sm mr-2">查看</a>
                    <a href="/customers/{}/edit" class="text-green-600 hover:text-green-800 text-sm mr-2">编辑</a>
                    <button onclick="deleteItem('/api/v1/customers/{}', '确认删除客户「{}」？', true)" class="text-red-600 hover:text-red-800 text-sm">删除</button>
                </td>
            </tr>"#,
            c.customer_code,
            c.name,
            c.mobile.as_deref().unwrap_or("-"),
            c.email.as_deref().unwrap_or("-"),
            status_badge,
            lead_badge,
            c.id,
            c.id,
            c.id,
            c.name
        )
    }).collect();

    let rows = if rows.is_empty() {
        r#"<tr><td colspan="7" class="px-6 py-12 text-center"><div class="text-gray-500"><p class="text-4xl mb-2">👥</p><p>暂无客户数据</p><a href="/customers/new" class="text-blue-500 hover:text-blue-600 mt-2 inline-block">添加第一个客户</a></div></td></tr>"#.to_string()
    } else {
        rows
    };

    let total_pages = ((result.pagination.total as f64) / (page_size as f64)).ceil() as u32;
    let pagination = if total_pages > 1 {
        let build_cust_url = |p: u32| -> String {
            let mut params = vec![format!("page={}", p)];
            if let Some(ref kw) = query.keyword { params.push(format!("keyword={}", kw)); }
            if let Some(ls) = query.lead_status { params.push(format!("lead_status={}", ls)); }
            if let Some(st) = query.status { params.push(format!("status={}", st)); }
            format!("/customers?{}", params.join("&"))
        };
        let prev_btn = if page > 1 {
            format!(r#"<a href="{}" class="px-4 py-2 text-sm text-gray-600 hover:bg-gray-100 rounded-lg border border-gray-300">上一页</a>"#, build_cust_url(page - 1))
        } else { String::new() };
        let next_btn = if page < total_pages {
            format!(r#"<a href="{}" class="px-4 py-2 text-sm text-gray-600 hover:bg-gray-100 rounded-lg border border-gray-300">下一页</a>"#, build_cust_url(page + 1))
        } else { String::new() };
        format!(
            r#"<div class="flex items-center justify-between px-4 sm:px-6 py-4 border-t border-gray-100">
                <p class="text-sm text-gray-600">共 {} 条记录，第 {}/{} 页</p>
                <div class="flex items-center gap-2">{}{}</div>
            </div>"#,
            result.pagination.total, page, total_pages, prev_btn, next_btn
        )
    } else {
        String::new()
    };

    let lead_active = |val: Option<i64>| -> &'static str {
        if query.lead_status == val { "bg-blue-600 text-white" } else { "bg-white text-gray-600 hover:bg-gray-50" }
    };
    let lead_tabs = format!(
        r#"<div class="flex flex-wrap gap-2 mb-6">
    <a href="/customers{}" class="px-4 py-2 text-sm rounded-lg border border-gray-200 {}">全部</a>
    <a href="/customers?lead_status=1{}" class="px-4 py-2 text-sm rounded-lg border border-gray-200 {}">潜在客户</a>
    <a href="/customers?lead_status=2{}" class="px-4 py-2 text-sm rounded-lg border border-gray-200 {}">跟进中</a>
    <a href="/customers?lead_status=3{}" class="px-4 py-2 text-sm rounded-lg border border-gray-200 {}">已成交</a>
    <a href="/customers?lead_status=4{}" class="px-4 py-2 text-sm rounded-lg border border-gray-200 {}">已流失</a>
</div>"#,
        if query.keyword.is_some() { format!("?keyword={}", query.keyword.as_deref().unwrap_or("")) } else { String::new() },
        lead_active(None),
        if query.keyword.is_some() { format!("&keyword={}", query.keyword.as_deref().unwrap_or("")) } else { String::new() },
        lead_active(Some(1)),
        if query.keyword.is_some() { format!("&keyword={}", query.keyword.as_deref().unwrap_or("")) } else { String::new() },
        lead_active(Some(2)),
        if query.keyword.is_some() { format!("&keyword={}", query.keyword.as_deref().unwrap_or("")) } else { String::new() },
        lead_active(Some(3)),
        if query.keyword.is_some() { format!("&keyword={}", query.keyword.as_deref().unwrap_or("")) } else { String::new() },
        lead_active(Some(4)),
    );
    let lead_status_hidden = query.lead_status.map(|ls| format!(r#"<input type="hidden" name="lead_status" value="{}">"#, ls)).unwrap_or_default();

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
<div class="bg-white rounded-xl shadow-sm border border-gray-100 p-4 mb-4">
    <form action="/customers" method="GET" class="flex flex-col sm:flex-row gap-3 sm:gap-4">
        {}
        <div class="flex-1">
            <input type="text" name="keyword" value="{}" placeholder="搜索客户名称、手机号..."
                   class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent">
        </div>
        <button type="submit" class="px-6 py-2 bg-gray-100 text-gray-700 rounded-lg hover:bg-gray-200 transition-colors w-full sm:w-auto">搜索</button>
    </form>
</div>

{}

<!-- 客户表格 -->
<div class="bg-white rounded-xl shadow-sm border border-gray-100 overflow-hidden">
    <div class="overflow-x-auto">
        <table class="w-full min-w-[800px]">
            <thead class="bg-gray-50 border-b border-gray-200">
                <tr>
                    <th class="px-4 sm:px-6 py-4 text-left text-sm font-semibold text-gray-700">客户编码</th>
                    <th class="px-4 sm:px-6 py-4 text-left text-sm font-semibold text-gray-700">姓名</th>
                    <th class="px-4 sm:px-6 py-4 text-left text-sm font-semibold text-gray-700">手机号</th>
                    <th class="px-4 sm:px-6 py-4 text-left text-sm font-semibold text-gray-700">邮箱</th>
                    <th class="px-4 sm:px-6 py-4 text-center text-sm font-semibold text-gray-700">状态</th>
                    <th class="px-4 sm:px-6 py-4 text-center text-sm font-semibold text-gray-700">阶段</th>
                    <th class="px-4 sm:px-6 py-4 text-center text-sm font-semibold text-gray-700">操作</th>
                </tr>
            </thead>
            <tbody class="divide-y divide-gray-100">{}</tbody>
        </table>
    </div>
    {}
</div>"#,
        lead_status_hidden,
        query.keyword.as_deref().unwrap_or(""),
        lead_tabs,
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
                    <a href="/suppliers/{}/edit" class="text-green-600 hover:text-green-800 text-sm mr-2">编辑</a>
                    <button onclick="deleteItem('/api/v1/suppliers/{}', '确认删除供应商「{}」？', true)" class="text-red-600 hover:text-red-800 text-sm">删除</button>
                </td>
            </tr>"#,
            s.supplier_code,
            s.name,
            s.contact_person.as_deref().unwrap_or("-"),
            s.contact_phone.as_deref().unwrap_or("-"),
            status_badge,
            s.id,
            s.id,
            s.id,
            s.name
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
    let customers_result = customer_queries.list(1, 500, None, None, None, None, None).await
        .unwrap_or_else(|_| PagedResponse::new(vec![], 1, 500, 0));
    let customers = customers_result.items;

    // 获取产品列表用于选择（全量，避免截断）
    let product_queries = ProductQueries::new(state.db.pool());
    let products = product_queries.list(1, 500, None, None, None, None, None, None, None).await
        .map(|r| r.items).unwrap_or_default();

    // 生成客户选项
    let customer_options: String = customers.iter().map(|c| {
        format!(r#"<option value="{}" data-name="{}" data-email="{}" data-mobile="{}">{}</option>"#,
            c.id, c.name, c.email.as_deref().unwrap_or(""), c.mobile.as_deref().unwrap_or(""), c.name)
    }).collect();

    // 生成产品选项（显示产品编码方便搜索）
    let product_options: String = products.iter().map(|p| {
        let price = p.sale_price_cny.unwrap_or(0.0);
        format!(r#"<option value="{}" data-name="{}" data-price="{}">[{}] {}</option>"#,
            p.id, p.name, price, p.product_code, p.name)
    }).collect();

    // 获取缓冲汇率（市场汇率 - 0.05）用于订单页面JS计算
    let order_buffered_rate = {
        let eq = ExchangeRateQueries::new(state.db.pool());
        let rate = eq.get_latest_rate("USD", "CNY").await
            .ok().flatten().map(|r| r.rate).unwrap_or(7.2);
        ((rate - 0.05) * 100.0).round() / 100.0
    };

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
                        <input type="text" id="customerSearch" class="w-full px-4 py-2 border border-gray-200 rounded-lg text-sm mb-1 focus:ring-2 focus:ring-blue-300" placeholder="🔍 输入姓名搜索客户..." oninput="filterCustomers(this.value)">
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
                            <input type="text" class="product-search w-full px-3 py-1 border border-gray-200 rounded-lg text-sm mb-1 focus:ring-2 focus:ring-blue-300" placeholder="🔍 编码或名称搜索..." oninput="filterProductSelect(this)">
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
// 缓冲汇率（market - 0.05）
const EXCHANGE_RATE = {order_buffered_rate};
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
                    // 使用缓冲汇率转换为USD
                    const refPriceUsd = (refPriceCny / EXCHANGE_RATE).toFixed(2);
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
    // 从第一行获取完整选项列表（_allOpts 保证包含所有产品）
    const firstSel = container.querySelector('.item-row select');
    const allOpts = (firstSel._allOpts || Array.from(firstSel.options)).map(o => o.cloneNode(true));

    const row = container.querySelector('.item-row').cloneNode(true);
    const searchInput = row.querySelector('.product-search');
    if (searchInput) {{ searchInput.value = ''; }}
    const sel = row.querySelector('select');
    sel.innerHTML = '';
    allOpts.forEach(o => sel.appendChild(o.cloneNode(true)));
    sel.value = '';
    sel._allOpts = allOpts.map(o => o.cloneNode(true));
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

// 客户搜索过滤（重建 DOM，兼容 Chrome）
let _custAllOpts = null;
function filterCustomers(q) {{
    const sel = document.getElementById('customerSelect');
    if (!_custAllOpts) {{
        _custAllOpts = Array.from(sel.options).map(o => o.cloneNode(true));
    }}
    const lower = q.toLowerCase().trim();
    const cur = sel.value;
    sel.innerHTML = '';
    _custAllOpts.forEach(opt => {{
        if (!lower || opt.text.toLowerCase().includes(lower)) {{
            sel.appendChild(opt.cloneNode(true));
        }}
    }});
    sel.value = cur;
}}

// 商品搜索过滤（重建 DOM，兼容 Chrome；每行独立）
function filterProductSelect(input) {{
    const q = input.value.toLowerCase().trim();
    const select = input.nextElementSibling;
    if (!select._allOpts) {{
        select._allOpts = Array.from(select.options).map(o => o.cloneNode(true));
    }}
    const cur = select.value;
    select.innerHTML = '';
    select._allOpts.forEach(opt => {{
        if (!q || opt.text.toLowerCase().includes(q)) {{
            select.appendChild(opt.cloneNode(true));
        }}
    }});
    select.value = cur;
}}
</script>"#,
        customer_options,
        product_options,
        order_buffered_rate = order_buffered_rate
    );

    render_layout("新建订单", "orders", Some(user), &content)
}

/// 创建订单表单
#[derive(Debug, Deserialize)]
pub struct OrderForm {
    #[serde(default, deserialize_with = "empty_string_as_none_i64")]
    customer_id: Option<i64>,
    customer_name: Option<String>,
    customer_mobile: Option<String>,
    customer_email: Option<String>,
    receiver_name: String,
    receiver_phone: String,
    country: String,
    address: String,
    save_address: Option<String>, // 保存到客户地址列表
    // 多商品支持 - HTML 表单发送 item_product[]=... 格式
    #[serde(rename = "item_product[]", deserialize_with = "str_or_vec_string")]
    item_product: Vec<String>,
    #[serde(rename = "item_quantity[]", deserialize_with = "str_or_vec_i64")]
    item_quantity: Vec<i64>,
    #[serde(rename = "item_price[]", deserialize_with = "str_or_vec_f64")]
    item_price: Vec<f64>,
    #[serde(default, deserialize_with = "empty_string_as_none_f64")]
    shipping_fee: Option<f64>,
    #[serde(default, deserialize_with = "empty_string_as_none_f64")]
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

    // 获取成交汇率快照（缓冲汇率）
    let snapshot_rate = {
        let eq = ExchangeRateQueries::new(state.db.pool());
        let rate = eq.get_latest_rate("USD", "CNY").await
            .ok().flatten().map(|r| r.rate).unwrap_or(7.2);
        ((rate - 0.05) * 100.0).round() / 100.0
    };

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
                        lead_status: None,
                        notes: Some("Auto-created from order".to_string()),
                        next_followup_date: None,
                        followup_notes: None,
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
        exchange_rate: Some(snapshot_rate),  // 成交时汇率快照
        currency: Some("USD".to_string()),   // 手动订单默认 USD
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
    let currency_sym = if order.order.currency.to_uppercase() == "USD" { "$" } else { "¥" };

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
                <div class="flex justify-between"><span class="text-gray-600">商品总额</span><span>{}{:.2}</span></div>
                <div class="flex justify-between"><span class="text-gray-600">运费</span><span>{}{:.2}</span></div>
                <div class="flex justify-between"><span class="text-gray-600">优惠</span><span>-{}{:.2}</span></div>
                <div class="flex justify-between pt-3 border-t border-gray-100 font-semibold">
                    <span>订单总额</span><span class="text-lg text-blue-600">{}{:.2}</span>
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
        currency_sym,
        order.order.subtotal,
        currency_sym,
        order.order.shipping_fee,
        currency_sym,
        order.order.discount_amount,
        currency_sym,
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
    let customers = customer_queries.list(1, 500, None, None, None, None, None).await
        .map(|r| r.items).unwrap_or_default();

    // 获取产品列表（全量）
    let product_queries = ProductQueries::new(state.db.pool());
    let products = product_queries.list(1, 500, None, None, None, None, None, None, None).await
        .map(|r| r.items).unwrap_or_default();

    let customer_options: String = customers.iter().map(|c| {
        let selected = order.order.customer_id.map(|id| id == c.id).unwrap_or(false);
        format!(r#"<option value="{}" data-name="{}" data-email="{}" data-mobile="{}" {}>{}</option>"#,
            c.id, c.name, c.email.as_deref().unwrap_or(""), c.mobile.as_deref().unwrap_or(""),
            if selected { "selected" } else { "" }, c.name)
    }).collect();

    let product_options: String = products.iter().map(|p| {
        let price = p.sale_price_cny.unwrap_or(0.0);
        format!(r#"<option value="{}" data-name="{}" data-price="{}"> [{}] {}</option>"#,
            p.id, p.name, price, p.product_code, p.name)
    }).collect();

    // 商品行
    let items_html: String = order.items.iter().enumerate().map(|(i, item)| {
        format!(r#"<div class="item-row grid grid-cols-12 gap-2 mb-2 items-end">
            <div class="col-span-5">
                <label class="block text-sm font-medium text-gray-700 mb-1">商品</label>
                <input type="text" class="product-search w-full px-3 py-1 border border-gray-200 rounded-lg text-sm mb-1 focus:ring-2 focus:ring-blue-300" placeholder="🔍 编码或名称搜索..." oninput="filterProductSelect(this)">
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
                        <input type="text" id="customerSearch" class="w-full px-4 py-2 border border-gray-200 rounded-lg text-sm mb-1 focus:ring-2 focus:ring-blue-300" placeholder="🔍 输入姓名搜索客户..." oninput="filterCustomers(this.value)">
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
    const firstSel = container.querySelector('.item-row select');
    const allOpts = (firstSel._allOpts || Array.from(firstSel.options)).map(o => o.cloneNode(true));

    const row = container.querySelector('.item-row').cloneNode(true);
    const searchInput = row.querySelector('.product-search');
    if (searchInput) {{ searchInput.value = ''; }}
    const sel = row.querySelector('select');
    sel.innerHTML = '';
    allOpts.forEach(o => sel.appendChild(o.cloneNode(true)));
    sel.value = '';
    sel._allOpts = allOpts.map(o => o.cloneNode(true));
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

function filterCustomers(q) {{
    const sel = document.getElementById('customerSelect');
    if (!sel._allOpts) {{
        sel._allOpts = Array.from(sel.options).map(o => o.cloneNode(true));
    }}
    const lower = q.toLowerCase().trim();
    const cur = sel.value;
    sel.innerHTML = '';
    sel._allOpts.forEach(opt => {{
        if (!lower || opt.text.toLowerCase().includes(lower)) {{
            sel.appendChild(opt.cloneNode(true));
        }}
    }});
    sel.value = cur;
}}

function filterProductSelect(input) {{
    const q = input.value.toLowerCase().trim();
    const select = input.nextElementSibling;
    if (!select._allOpts) {{
        select._allOpts = Array.from(select.options).map(o => o.cloneNode(true));
    }}
    const cur = select.value;
    select.innerHTML = '';
    select._allOpts.forEach(opt => {{
        if (!q || opt.text.toLowerCase().includes(q)) {{
            select.appendChild(opt.cloneNode(true));
        }}
    }});
    select.value = cur;
}}
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
    #[serde(default, deserialize_with = "empty_string_as_none_i64")]
    customer_id: Option<i64>,
    customer_name: Option<String>,
    customer_mobile: Option<String>,
    customer_email: Option<String>,
    receiver_name: String,
    receiver_phone: String,
    country: String,
    address: String,
    // HTML 表单发送 item_product[]=... 格式
    #[serde(rename = "item_product[]", deserialize_with = "str_or_vec_string")]
    item_product: Vec<String>,
    #[serde(rename = "item_quantity[]", deserialize_with = "str_or_vec_i64")]
    item_quantity: Vec<i64>,
    #[serde(rename = "item_price[]", deserialize_with = "str_or_vec_f64")]
    item_price: Vec<f64>,
    #[serde(default, deserialize_with = "empty_string_as_none_f64")]
    shipping_fee: Option<f64>,
    #[serde(default, deserialize_with = "empty_string_as_none_f64")]
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
                <label class="block text-sm font-medium text-gray-700 mb-2">产品 ID <span class="text-red-500">*</span></label>
                <input type="number" name="product_id" required min="1" class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500" placeholder="输入产品 ID">
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
    product_id: i64,
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

    match queries.update(form.product_id, &req, None).await {
        Ok(Some(inventory)) => {
            info!("Inventory updated: product_id={}, qty={}", form.product_id, form.quantity);
            Ok(Redirect::to(&format!("/inventory/{}", inventory.product_id)))
        }
        Ok(None) => {
            Err(Html(r#"<!DOCTYPE html><html><body><script>alert('产品不存在');history.back();</script></body></html>"#.to_string()))
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
    let inventory = match queries.get_by_product(id).await {
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
        <h3 class="text-base font-semibold text-gray-800 mb-4">📦 产品信息</h3>
        <div class="space-y-3">
            <div class="flex justify-between"><span class="text-gray-500">产品 ID</span><span class="font-mono">{}</span></div>
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
        inventory.product_id,
        inventory.total_quantity,
        inventory.available_quantity,
        inventory.locked_quantity,
        inventory.safety_stock,
        inventory.product_id,
        inventory.damaged_quantity,
        inventory.warehouse_id.map(|id| id.to_string()).unwrap_or("-".to_string()),
        inventory.updated_at.format("%Y-%m-%d %H:%M"),
        inventory.product_id,
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
    let inventory = match queries.get_by_product(id).await {
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
        inventory.product_id,
        inventory.total_quantity,
        inventory.available_quantity,
        inventory.locked_quantity,
        inventory.damaged_quantity,
        inventory.product_id,
        inventory.product_id
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
    let current = match queries.get_by_product(id).await {
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
            info!("Inventory adjusted: product_id={}, type={}, qty={}", id, form.adjust_type, form.quantity);
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
    let inventory = queries.get_by_product(id).await.ok().flatten();

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
                        <span class="text-gray-500">产品 ID: </span>
                        <span class="font-mono">{}</span>
                    </div>
                    <div class="flex items-center gap-4 text-sm">
                        <span><span class="text-gray-500">总库存:</span> <span class="font-medium">{}</span></span>
                        <span><span class="text-gray-500">可用:</span> <span class="font-medium text-green-600">{}</span></span>
                        <span><span class="text-gray-500">锁定:</span> <span class="font-medium text-yellow-600">{}</span></span>
                    </div>
                </div>
            </div>"#,
            inv.product_id,
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
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-2">销售阶段</label>
                <select name="lead_status" class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500">
                    <option value="1">潜在客户</option>
                    <option value="2">跟进中</option>
                    <option value="3">已成交</option>
                    <option value="4">已流失</option>
                </select>
            </div>
            <div class="md:col-span-2">
                <label class="block text-sm font-medium text-gray-700 mb-2">备注</label>
                <textarea name="notes" rows="2" class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500" placeholder="备注信息..."></textarea>
            </div>
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-2">下次跟进日期</label>
                <input type="date" name="next_followup_date" value=""
                       class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500">
            </div>
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-2">跟进备注</label>
                <textarea name="followup_notes" rows="2" placeholder="跟进情况记录..."
                          class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"></textarea>
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
    mobile: String,
    email: Option<String>,
    status: Option<i64>,
    lead_status: Option<i64>,
    notes: Option<String>,
    next_followup_date: Option<String>,
    followup_notes: Option<String>,
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
        lead_status: form.lead_status,
        notes: form.notes.clone(),
        next_followup_date: form.next_followup_date.clone(),
        followup_notes: form.followup_notes.clone(),
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
    let lead_selected_1 = if customer.lead_status == 1 { "selected" } else { "" };
    let lead_selected_2 = if customer.lead_status == 2 { "selected" } else { "" };
    let lead_selected_3 = if customer.lead_status == 3 { "selected" } else { "" };
    let lead_selected_4 = if customer.lead_status == 4 { "selected" } else { "" };

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
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-2">销售阶段</label>
                <select name="lead_status" class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500">
                    <option value="1" {}>潜在客户</option>
                    <option value="2" {}>跟进中</option>
                    <option value="3" {}>已成交</option>
                    <option value="4" {}>已流失</option>
                </select>
            </div>
            <div class="md:col-span-2">
                <label class="block text-sm font-medium text-gray-700 mb-2">备注</label>
                <textarea name="notes" rows="2" class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500" placeholder="备注信息...">{}</textarea>
            </div>
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-2">下次跟进日期</label>
                <input type="date" name="next_followup_date" value="{}"
                       class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500">
            </div>
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-2">跟进备注</label>
                <textarea name="followup_notes" rows="2" placeholder="跟进情况记录..."
                          class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500">{}</textarea>
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
        lead_selected_1,
        lead_selected_2,
        lead_selected_3,
        lead_selected_4,
        customer.notes.unwrap_or_default(),
        customer.next_followup_date.unwrap_or_default(),
        customer.followup_notes.unwrap_or_default(),
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
        lead_status: form.lead_status,
        notes: form.notes.clone(),
        next_followup_date: form.next_followup_date.clone(),
        followup_notes: form.followup_notes.clone(),
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
                    <a href="/purchase/{}" class="text-blue-600 hover:text-blue-800 text-sm mr-2">查看</a>
                    <button onclick="deleteItem('/api/v1/purchases/{}', '确认删除采购单「{}」？', true)" class="text-red-600 hover:text-red-800 text-sm">删除</button>
                </td>
            </tr>"#,
            p.order_code,
            supplier_display,
            p.item_count,
            p.total_amount,
            status_badge,
            p.id,
            p.id,
            p.order_code
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
    let products = match product_queries.list(1, 1000, None, None, None, None, None, None, None).await {
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
        r#"<button onclick="submitForReview()" class="px-4 py-2 bg-yellow-500 text-white rounded-lg hover:bg-yellow-600 transition-colors">
                提交审核
            </button>"#
    } else if detail.order.status == 2 {
        r#"<button onclick="confirmApproval()" class="px-4 py-2 bg-green-500 text-white rounded-lg hover:bg-green-600 transition-colors">
                审批通过
            </button>"#
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
</div>

<script>
function submitForReview() {{
    if (!confirm('确认提交审核？')) return;
    fetch('/api/v1/purchases/{}/approve', {{
        method: 'POST',
        headers: {{'Content-Type': 'application/json'}},
        body: JSON.stringify({{approval_note: ''}})
    }}).then(r => r.json()).then(d => {{
        if (d.code === 200) {{ location.reload(); }}
        else {{ alert('提交失败: ' + d.message); }}
    }}).catch(e => alert('请求失败: ' + e));
}}
function confirmApproval() {{
    if (!confirm('确认审批通过？')) return;
    fetch('/api/v1/purchases/{}/confirm', {{
        method: 'POST',
        headers: {{'Content-Type': 'application/json'}}
    }}).then(r => r.json()).then(d => {{
        if (d.code === 200) {{ location.reload(); }}
        else {{ alert('审批失败: ' + d.message); }}
    }}).catch(e => alert('请求失败: ' + e));
}}
</script>"#,
        detail.order.order_code,
        detail.items.len(),
        status_badge,
        action_buttons,
        detail.order.order_code,
        detail.order.total_amount,
        detail.order.expected_date.as_deref().unwrap_or("-"),
        detail.order.created_at,
        if detail.order.internal_note.is_some() { format!(r#"<div class="mt-4 pt-4 border-t border-gray-200"><span class="text-sm text-gray-500">备注:</span> {}</div>"#, detail.order.internal_note.as_deref().unwrap_or("")) } else { String::new() },
        supplier_sections,
        detail.order.id,
        detail.order.id
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
    #[serde(default, deserialize_with = "empty_string_as_none_i32")]
    pub year: Option<i32>,
    #[serde(default, deserialize_with = "empty_string_as_none_i32")]
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
        let (platform_name, currency_symbol): (&str, &str) = match p.platform.as_str() {
            "ali" => ("阿里国际站", "$"),
            "ae" => ("速卖通", "¥"),
            "manual" => ("手动创建", "$"),
            other => (other, "$"),
        };
        format!(
            r#"<tr class="hover:bg-gray-50">
                <td class="px-4 py-3">{}</td>
                <td class="px-4 py-3 text-center">{}</td>
                <td class="px-4 py-3 text-right">{}{:.2}</td>
            </tr>"#,
            platform_name,
            p.order_count,
            currency_symbol,
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
                <li>平台：阿里国际站(ali) = 阿里巴巴国际站订单，速卖通(ae) = 速卖通平台订单</li>
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
