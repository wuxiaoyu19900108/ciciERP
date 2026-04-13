//! PI/CI Web 页面路由
//!
//! 提供 Proforma Invoice 和 Commercial Invoice 的 HTML 页面

use axum::{
    extract::{Extension, Path, Query, State},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Router,
    body::Body,
};
use rust_xlsxwriter::{Workbook, Format, FormatBorder};
use serde::Deserialize;
use tracing::info;

use crate::middleware::auth::AuthUser;
use crate::state::AppState;
use crate::templates::base::{get_menus, UserInfo};
use cicierp_db::queries::proforma_invoices::ProformaInvoiceQueries;
use cicierp_db::queries::commercial_invoices::CommercialInvoiceQueries;
use cicierp_db::queries::exchange_rates::ExchangeRateQueries;
use cicierp_models::proforma_invoice::PIQuery;
use cicierp_models::commercial_invoice::CIQuery;

// ==================== PI 状态辅助函数 ====================

/// PI 状态文本
fn pi_status_text(status: i64) -> &'static str {
    match status {
        1 => "草稿",
        2 => "已发送",
        3 => "已确认",
        4 => "已转订单",
        5 => "已取消",
        _ => "未知",
    }
}

/// PI 状态样式
fn pi_status_class(status: i64) -> &'static str {
    match status {
        1 => "bg-yellow-100 text-yellow-700",
        2 => "bg-blue-100 text-blue-700",
        3 => "bg-green-100 text-green-700",
        4 => "bg-indigo-100 text-indigo-700",
        5 => "bg-gray-100 text-gray-600",
        _ => "bg-gray-100 text-gray-500",
    }
}

/// PI 状态徽章
fn pi_status_badge(status: i64) -> String {
    let text = pi_status_text(status);
    let class = pi_status_class(status);
    format!(r#"<span class="{} px-2 py-1 text-xs rounded-full">{}</span>"#, class, text)
}

// ==================== CI 状态辅助函数 ====================

/// CI 状态文本
fn ci_status_text(status: i64) -> &'static str {
    match status {
        1 => "草稿",
        2 => "已发送",
        3 => "已付款",
        _ => "未知",
    }
}

/// CI 状态样式
fn ci_status_class(status: i64) -> &'static str {
    match status {
        1 => "bg-yellow-100 text-yellow-700",
        2 => "bg-blue-100 text-blue-700",
        3 => "bg-green-100 text-green-700",
        _ => "bg-gray-100 text-gray-500",
    }
}

/// CI 状态徽章
fn ci_status_badge(status: i64) -> String {
    let text = ci_status_text(status);
    let class = ci_status_class(status);
    format!(r#"<span class="{} px-2 py-1 text-xs rounded-full">{}</span>"#, class, text)
}

// ==================== 辅助函数 ====================

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
    let menus = get_menus();

    let menu_html: String = menus
        .iter()
        .map(|menu| {
            let active_class = if menu.code == active_menu {
                "bg-blue-50 text-blue-600"
            } else {
                "text-gray-600 hover:bg-gray-50"
            };
            format!(
                r#"<a href="{}" class="flex items-center px-4 py-2 {}">
                    <span class="mr-2">{}</span>{}
                </a>"#,
                menu.href, active_class, menu.icon, menu.label
            )
        })
        .collect();

    let user_display = user
        .as_ref()
        .map(|u| u.display_name().to_string())
        .unwrap_or_else(|| "用户".to_string());

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{} - ciciERP</title>
    <script src="https://unpkg.com/htmx.org@1.9.10"></script>
    <script src="https://cdn.tailwindcss.com"></script>
</head>
<body class="bg-gray-100">
    <div class="flex h-screen">
        <!-- 侧边栏 -->
        <aside class="w-64 bg-white shadow-lg">
            <div class="p-4 border-b">
                <h1 class="text-xl font-bold text-gray-800">ciciERP</h1>
            </div>
            <nav class="mt-4">{}</nav>
        </aside>
        <!-- 主内容 -->
        <main class="flex-1 overflow-auto">
            <header class="bg-white shadow px-6 py-4 flex justify-between items-center">
                <h2 class="text-xl font-semibold">{}</h2>
                <div class="text-gray-600">欢迎，{}</div>
            </header>
            <div class="p-6">{}</div>
        </main>
    </div>
</body>
</html>"#,
        title, menu_html, title, user_display, content
    );

    Html(html)
}

// ==================== PI 列表页面 ====================

/// PI 列表页面
pub async fn pi_list_page(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Query(query): Query<PIQuery>,
) -> impl IntoResponse {
    let user = get_user_from_extension(&auth_user);

    let result = ProformaInvoiceQueries::new(state.db.pool())
        .list(&query)
        .await;

    let response = match result {
        Ok(data) => data,
        Err(e) => {
            let error_content = format!(
                r#"<div class="bg-red-100 text-red-700 p-4 rounded">加载失败: {}</div>"#,
                e
            );
            return render_layout("PI 管理", "pi", Some(user), &error_content);
        }
    };

    let pis = response.items;
    let total = response.pagination.total;
    let page = query.page();
    let page_size = query.page_size();

    let rows_html: String = pis
        .iter()
        .map(|pi| {
            let badge = pi_status_badge(pi.status);
            format!(
                r#"<tr class="hover:bg-gray-50">
                    <td class="px-6 py-4 text-sm">{}</td>
                    <td class="px-6 py-4 text-sm">{}</td>
                    <td class="px-6 py-4 text-sm">{:.2}</td>
                    <td class="px-6 py-4 text-sm">{}</td>
                    <td class="px-6 py-4">{}</td>
                    <td class="px-6 py-4 text-sm">{}</td>
                    <td class="px-6 py-4 text-sm">
                        <a href="/orders/pi/{}" class="text-blue-600 hover:text-blue-800 mr-3">查看</a>
                    </td>
                </tr>"#,
                pi.pi_code,
                pi.customer_name,
                pi.total_amount,
                pi.currency,
                badge,
                pi.pi_date,
                pi.id
            )
        })
        .collect();

    let total_pages = ((total as f64) / (page_size as f64)).ceil() as i64;

    let content = format!(
        r#"<div class="bg-white rounded-lg shadow">
            <div class="px-6 py-4 border-b flex justify-between items-center">
                <h3 class="text-lg font-semibold">PI 列表</h3>
                <a href="/orders/pi/new" class="bg-blue-500 text-white px-4 py-2 rounded hover:bg-blue-600">新建 PI</a>
            </div>
            <div class="overflow-x-auto">
                <table class="w-full">
                    <thead class="bg-gray-50">
                        <tr>
                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">PI 编号</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">客户</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">总金额</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">币种</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">状态</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">创建日期</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">操作</th>
                        </tr>
                    </thead>
                    <tbody class="divide-y divide-gray-200">{}</tbody>
                </table>
            </div>
            <div class="px-6 py-4 border-t text-sm text-gray-600">
                共 {} 条，第 {}/{} 页
            </div>
        </div>"#,
        rows_html, total, page, total_pages
    );

    render_layout("PI 管理", "pi", Some(user), &content)
}

// ==================== PI 详情页面 ====================

/// PI 详情页面
pub async fn pi_detail_page(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let user = get_user_from_extension(&auth_user);

    let result = ProformaInvoiceQueries::new(state.db.pool())
        .get_detail(id)
        .await;

    let detail = match result {
        Ok(Some(d)) => d,
        Ok(None) => {
            let error_content = r#"<div class="bg-red-100 text-red-700 p-4 rounded">PI 不存在</div>"#;
            return render_layout("PI 详情", "pi", Some(user), error_content);
        }
        Err(e) => {
            let error_content = format!(
                r#"<div class="bg-red-100 text-red-700 p-4 rounded">加载失败: {}</div>"#,
                e
            );
            return render_layout("PI 详情", "pi", Some(user), &error_content);
        }
    };

    let pi = &detail.pi;
    let badge = pi_status_badge(pi.status);

    // 明细表格
    let items_html: String = detail.items
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            format!(
                r#"<tr class="hover:bg-gray-50">
                    <td class="px-4 py-3 text-sm text-center">{}</td>
                    <td class="px-4 py-3 text-sm">{}</td>
                    <td class="px-4 py-3 text-sm">{}</td>
                    <td class="px-4 py-3 text-sm text-right">{}</td>
                    <td class="px-4 py-3 text-sm text-right">{:.2}</td>
                    <td class="px-4 py-3 text-sm text-right">{:.2}</td>
                </tr>"#,
                idx + 1,
                item.product_name,
                item.model.as_deref().unwrap_or("-"),
                item.quantity,
                item.unit_price,
                item.total_price
            )
        })
        .collect();

    // 操作按钮
    let actions_html = match pi.status {
        1 => format!(r#"
            <form method="POST" action="/orders/pi/{}/send" style="display:inline">
                <button type="submit" class="bg-blue-500 text-white px-4 py-2 rounded hover:bg-blue-600 mr-2">发送</button>
            </form>
            <form method="POST" action="/orders/pi/{}/cancel" style="display:inline">
                <button type="submit" class="bg-red-500 text-white px-4 py-2 rounded hover:bg-red-600 mr-2" onclick="return confirm('确认取消此 PI?')">取消</button>
            </form>
        "#, pi.id, pi.id),
        2 => format!(r#"
            <form method="POST" action="/orders/pi/{}/confirm" style="display:inline">
                <button type="submit" class="bg-green-500 text-white px-4 py-2 rounded hover:bg-green-600 mr-2" onclick="return confirm('确认此 PI?')">确认</button>
            </form>
            <form method="POST" action="/orders/pi/{}/cancel" style="display:inline">
                <button type="submit" class="bg-red-500 text-white px-4 py-2 rounded hover:bg-red-600 mr-2" onclick="return confirm('确认取消此 PI?')">取消</button>
            </form>
        "#, pi.id, pi.id),
        3 => format!(r#"
            <form method="POST" action="/orders/pi/{}/convert" style="display:inline">
                <button type="submit" class="bg-indigo-500 text-white px-4 py-2 rounded hover:bg-indigo-600 mr-2" onclick="return confirm('确认将此 PI 转为订单?')">转订单</button>
            </form>
        "#, pi.id),
        _ => String::new(),
    };

    let content = format!(
        r#"<div class="bg-white rounded-lg shadow">
            <div class="px-6 py-4 border-b flex justify-between items-center">
                <div class="flex items-center gap-4">
                    <h3 class="text-lg font-semibold">PI - {}</h3>
                    {}
                </div>
                <div class="flex gap-2">
                    <a href="/orders/pi/{}/download" class="bg-green-500 text-white px-4 py-2 rounded hover:bg-green-600">下载 Excel</a>
                    <a href="/orders/pi" class="bg-gray-500 text-white px-4 py-2 rounded hover:bg-gray-600">返回列表</a>
                </div>
            </div>
            <div class="p-6">
                <!-- 基本信息 -->
                <div class="grid grid-cols-2 gap-6 mb-6">
                    <div>
                        <h4 class="font-semibold mb-3 text-gray-700">客户信息</h4>
                        <p class="text-sm mb-1"><span class="text-gray-500">客户名称:</span> {}</p>
                        <p class="text-sm mb-1"><span class="text-gray-500">邮箱:</span> {}</p>
                        <p class="text-sm mb-1"><span class="text-gray-500">电话:</span> {}</p>
                        <p class="text-sm"><span class="text-gray-500">地址:</span> {}</p>
                    </div>
                    <div>
                        <h4 class="font-semibold mb-3 text-gray-700">PI 信息</h4>
                        <p class="text-sm mb-1"><span class="text-gray-500">PI 日期:</span> {}</p>
                        <p class="text-sm mb-1"><span class="text-gray-500">有效期至:</span> {}</p>
                        <p class="text-sm mb-1"><span class="text-gray-500">付款条款:</span> {}</p>
                        <p class="text-sm"><span class="text-gray-500">交货条款:</span> {}</p>
                    </div>
                </div>
                <!-- 产品明细 -->
                <div class="mb-6">
                    <h4 class="font-semibold mb-3 text-gray-700">产品明细</h4>
                    <table class="w-full border">
                        <thead class="bg-gray-50">
                            <tr>
                                <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 w-12">#</th>
                                <th class="px-4 py-2 text-left text-xs font-medium text-gray-500">产品名称</th>
                                <th class="px-4 py-2 text-left text-xs font-medium text-gray-500">型号</th>
                                <th class="px-4 py-2 text-right text-xs font-medium text-gray-500">数量</th>
                                <th class="px-4 py-2 text-right text-xs font-medium text-gray-500">单价</th>
                                <th class="px-4 py-2 text-right text-xs font-medium text-gray-500">总价</th>
                            </tr>
                        </thead>
                        <tbody class="divide-y">{}</tbody>
                    </table>
                </div>
                <!-- 金额汇总 -->
                <div class="flex justify-end">
                    <div class="w-64">
                        <div class="flex justify-between py-2 text-sm">
                            <span>小计:</span>
                            <span>{:.2} {}</span>
                        </div>
                        <div class="flex justify-between py-2 text-sm">
                            <span>折扣:</span>
                            <span>{:.2} {}</span>
                        </div>
                        <div class="flex justify-between py-2 font-semibold border-t">
                            <span>总计:</span>
                            <span>{:.2} {}</span>
                        </div>
                    </div>
                </div>
                <!-- 操作按钮 -->
                <div class="mt-6 flex justify-end">
                    {}
                </div>
            </div>
        </div>"#,
        pi.pi_code,
        badge,
        pi.id,
        pi.customer_name,
        pi.customer_email.as_deref().unwrap_or("-"),
        pi.customer_phone.as_deref().unwrap_or("-"),
        pi.customer_address.as_deref().unwrap_or("-"),
        pi.pi_date,
        pi.valid_until.as_deref().unwrap_or("-"),
        pi.payment_terms,
        pi.delivery_terms,
        items_html,
        pi.subtotal, pi.currency,
        pi.discount, pi.currency,
        pi.total_amount, pi.currency,
        actions_html
    );

    render_layout("PI 详情", "pi", Some(user), &content)
}

// ==================== CI 列表页面 ====================

/// CI 列表页面
pub async fn ci_list_page(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Query(query): Query<CIQuery>,
) -> impl IntoResponse {
    let user = get_user_from_extension(&auth_user);

    let result = CommercialInvoiceQueries::new(state.db.pool())
        .list(&query)
        .await;

    let response = match result {
        Ok(data) => data,
        Err(e) => {
            let error_content = format!(
                r#"<div class="bg-red-100 text-red-700 p-4 rounded">加载失败: {}</div>"#,
                e
            );
            return render_layout("CI 管理", "ci", Some(user), &error_content);
        }
    };

    let cis = response.items;
    let total = response.pagination.total;
    let page = query.page();
    let page_size = query.page_size();

    let rows_html: String = cis
        .iter()
        .map(|ci| {
            let badge = ci_status_badge(ci.status);
            format!(
                r#"<tr class="hover:bg-gray-50">
                    <td class="px-6 py-4 text-sm">{}</td>
                    <td class="px-6 py-4 text-sm">{}</td>
                    <td class="px-6 py-4 text-sm">{:.2}</td>
                    <td class="px-6 py-4 text-sm">{}</td>
                    <td class="px-6 py-4">{}</td>
                    <td class="px-6 py-4 text-sm">{}</td>
                    <td class="px-6 py-4 text-sm">
                        <a href="/orders/ci/{}" class="text-blue-600 hover:text-blue-800 mr-3">查看</a>
                    </td>
                </tr>"#,
                ci.ci_code,
                ci.customer_name,
                ci.total_amount,
                ci.currency,
                badge,
                ci.ci_date,
                ci.id
            )
        })
        .collect();

    let total_pages = ((total as f64) / (page_size as f64)).ceil() as i64;

    let content = format!(
        r#"<div class="bg-white rounded-lg shadow">
            <div class="px-6 py-4 border-b">
                <h3 class="text-lg font-semibold">CI 列表</h3>
            </div>
            <div class="overflow-x-auto">
                <table class="w-full">
                    <thead class="bg-gray-50">
                        <tr>
                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">CI 编号</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">客户</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">总金额</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">币种</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">状态</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">创建日期</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">操作</th>
                        </tr>
                    </thead>
                    <tbody class="divide-y divide-gray-200">{}</tbody>
                </table>
            </div>
            <div class="px-6 py-4 border-t text-sm text-gray-600">
                共 {} 条，第 {}/{} 页
            </div>
        </div>"#,
        rows_html, total, page, total_pages
    );

    render_layout("CI 管理", "ci", Some(user), &content)
}

// ==================== CI 详情页面 ====================

/// CI 详情页面
pub async fn ci_detail_page(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let user = get_user_from_extension(&auth_user);

    let result = CommercialInvoiceQueries::new(state.db.pool())
        .get_detail(id)
        .await;

    let detail = match result {
        Ok(Some(d)) => d,
        Ok(None) => {
            let error_content = r#"<div class="bg-red-100 text-red-700 p-4 rounded">CI 不存在</div>"#;
            return render_layout("CI 详情", "ci", Some(user), error_content);
        }
        Err(e) => {
            let error_content = format!(
                r#"<div class="bg-red-100 text-red-700 p-4 rounded">加载失败: {}</div>"#,
                e
            );
            return render_layout("CI 详情", "ci", Some(user), &error_content);
        }
    };

    let ci = &detail.ci;
    let badge = ci_status_badge(ci.status);

    // 明细表格
    let items_html: String = detail.items
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            format!(
                r#"<tr class="hover:bg-gray-50">
                    <td class="px-4 py-3 text-sm text-center">{}</td>
                    <td class="px-4 py-3 text-sm">{}</td>
                    <td class="px-4 py-3 text-sm">{}</td>
                    <td class="px-4 py-3 text-sm text-right">{}</td>
                    <td class="px-4 py-3 text-sm text-right">{:.2}</td>
                    <td class="px-4 py-3 text-sm text-right">{:.2}</td>
                </tr>"#,
                idx + 1,
                item.product_name,
                item.model.as_deref().unwrap_or("-"),
                item.quantity,
                item.unit_price,
                item.total_price
            )
        })
        .collect();

    // 操作按钮
    let actions_html = match ci.status {
        1 | 2 => format!(r#"
            <form method="POST" action="/orders/ci/{}/mark-paid" style="display:inline" onsubmit="return confirm('确认标记为已付款?')">
                <input type="hidden" name="paid_amount" value="{:.2}">
                <button type="submit" class="bg-green-500 text-white px-4 py-2 rounded hover:bg-green-600 mr-2">标记已付款</button>
            </form>
        "#, ci.id, ci.total_amount),
        _ => String::new(),
    };

    let content = format!(
        r#"<div class="bg-white rounded-lg shadow">
            <div class="px-6 py-4 border-b flex justify-between items-center">
                <div class="flex items-center gap-4">
                    <h3 class="text-lg font-semibold">CI - {}</h3>
                    {}
                </div>
                <div class="flex gap-2">
                    <a href="/orders/ci/{}/download" class="bg-green-500 text-white px-4 py-2 rounded hover:bg-green-600">下载 Excel</a>
                    <a href="/orders/ci" class="bg-gray-500 text-white px-4 py-2 rounded hover:bg-gray-600">返回列表</a>
                </div>
            </div>
            <div class="p-6">
                <!-- 基本信息 -->
                <div class="grid grid-cols-2 gap-6 mb-6">
                    <div>
                        <h4 class="font-semibold mb-3 text-gray-700">客户信息</h4>
                        <p class="text-sm mb-1"><span class="text-gray-500">客户名称:</span> {}</p>
                        <p class="text-sm mb-1"><span class="text-gray-500">邮箱:</span> {}</p>
                        <p class="text-sm mb-1"><span class="text-gray-500">电话:</span> {}</p>
                        <p class="text-sm"><span class="text-gray-500">地址:</span> {}</p>
                    </div>
                    <div>
                        <h4 class="font-semibold mb-3 text-gray-700">CI 信息</h4>
                        <p class="text-sm mb-1"><span class="text-gray-500">CI 日期:</span> {}</p>
                        <p class="text-sm mb-1"><span class="text-gray-500">已付金额:</span> {:.2}</p>
                        <p class="text-sm"><span class="text-gray-500">关联订单:</span> 订单 #{}</p>
                    </div>
                </div>
                <!-- 产品明细 -->
                <div class="mb-6">
                    <h4 class="font-semibold mb-3 text-gray-700">产品明细</h4>
                    <table class="w-full border">
                        <thead class="bg-gray-50">
                            <tr>
                                <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 w-12">#</th>
                                <th class="px-4 py-2 text-left text-xs font-medium text-gray-500">产品名称</th>
                                <th class="px-4 py-2 text-left text-xs font-medium text-gray-500">型号</th>
                                <th class="px-4 py-2 text-right text-xs font-medium text-gray-500">数量</th>
                                <th class="px-4 py-2 text-right text-xs font-medium text-gray-500">单价</th>
                                <th class="px-4 py-2 text-right text-xs font-medium text-gray-500">总价</th>
                            </tr>
                        </thead>
                        <tbody class="divide-y">{}</tbody>
                    </table>
                </div>
                <!-- 金额汇总 -->
                <div class="flex justify-end">
                    <div class="w-64">
                        <div class="flex justify-between py-2 font-semibold border-t">
                            <span>总计:</span>
                            <span>{:.2} {}</span>
                        </div>
                    </div>
                </div>
                <!-- 操作按钮 -->
                <div class="mt-6 flex justify-end">
                    {}
                </div>
            </div>
        </div>"#,
        ci.ci_code,
        badge,
        ci.id,
        ci.customer_name,
        ci.customer_email.as_deref().unwrap_or("-"),
        ci.customer_phone.as_deref().unwrap_or("-"),
        ci.customer_address.as_deref().unwrap_or("-"),
        ci.ci_date,
        ci.paid_amount,
        ci.sales_order_id,
        items_html,
        ci.total_amount, ci.currency,
        actions_html
    );

    render_layout("CI 详情", "ci", Some(user), &content)
}

// ==================== Excel 导出 ====================

/// PI 下载 Excel
pub async fn pi_download_handler(
    State(state): State<AppState>,
    Extension(_auth_user): Extension<AuthUser>,
    Path(id): Path<i64>,
) -> Result<Response<Body>, Response<Body>> {
    let result = ProformaInvoiceQueries::new(state.db.pool())
        .get_detail(id)
        .await
        .map_err(|e| {
            Response::builder()
                .status(500)
                .body(Body::from(format!("获取 PI 失败: {}", e)))
                .unwrap()
        })?;

    let detail = result.ok_or_else(|| {
        Response::builder()
            .status(404)
            .body(Body::from("PI 不存在"))
            .unwrap()
    })?;

    let pi = &detail.pi;

    // 创建 Excel 工作簿
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet().set_name("PI").unwrap();

    // 标题格式
    let title_format = Format::new()
        .set_bold()
        .set_font_size(16)
        .set_align(rust_xlsxwriter::FormatAlign::Center);

    // 表头格式
    let header_format = Format::new()
        .set_bold()
        .set_background_color("#4472C4")
        .set_font_color("#FFFFFF")
        .set_border(FormatBorder::Thin)
        .set_align(rust_xlsxwriter::FormatAlign::Center);

    // 普通单元格格式
    let cell_format = Format::new()
        .set_border(FormatBorder::Thin);

    // 金额格式
    let money_format = Format::new()
        .set_border(FormatBorder::Thin)
        .set_num_format("#,##0.00");

    // 标题
    worksheet.write_string_with_format(0, 2, "PROFORMA INVOICE", &title_format).unwrap();

    // PI 信息
    worksheet.write_string(2, 0, "PI Number:").unwrap();
    worksheet.write_string(2, 1, &pi.pi_code).unwrap();
    worksheet.write_string(2, 3, "Date:").unwrap();
    worksheet.write_string(2, 4, &pi.pi_date).unwrap();

    // 卖家信息
    worksheet.write_string(4, 0, "Seller:").unwrap();
    worksheet.write_string(4, 1, &pi.seller_name).unwrap();
    if let Some(addr) = &pi.seller_address {
        worksheet.write_string(5, 1, addr).unwrap();
    }

    // 客户信息
    worksheet.write_string(7, 0, "Buyer:").unwrap();
    worksheet.write_string(7, 1, &pi.customer_name).unwrap();
    if let Some(addr) = &pi.customer_address {
        worksheet.write_string(8, 1, addr).unwrap();
    }

    // 产品明细表头
    let headers = ["#", "Product Name", "Model", "Qty", "Unit Price", "Total"];
    for (col, header) in headers.iter().enumerate() {
        worksheet.write_string_with_format(10, col as u16, *header, &header_format).unwrap();
    }

    // 产品明细
    for (row, item) in detail.items.iter().enumerate() {
        let row_num = 11 + row as u32;
        worksheet.write_string_with_format(row_num, 0, &(row + 1).to_string(), &cell_format).unwrap();
        worksheet.write_string_with_format(row_num, 1, &item.product_name, &cell_format).unwrap();
        worksheet.write_string_with_format(row_num, 2, item.model.as_deref().unwrap_or("-"), &cell_format).unwrap();
        worksheet.write_number_with_format(row_num, 3, item.quantity as f64, &cell_format).unwrap();
        worksheet.write_number_with_format(row_num, 4, item.unit_price, &money_format).unwrap();
        worksheet.write_number_with_format(row_num, 5, item.total_price, &money_format).unwrap();
    }

    // 汇总
    let summary_row = 11 + detail.items.len() as u32 + 1;
    worksheet.write_string_with_format(summary_row, 4, "Subtotal:", &cell_format).unwrap();
    worksheet.write_number_with_format(summary_row, 5, pi.subtotal, &money_format).unwrap();

    if pi.discount > 0.0 {
        worksheet.write_string_with_format(summary_row + 1, 4, "Discount:", &cell_format).unwrap();
        worksheet.write_number_with_format(summary_row + 1, 5, pi.discount, &money_format).unwrap();
    }

    let total_row = summary_row + 2;
    worksheet.write_string_with_format(total_row, 4, "Total:", &header_format).unwrap();
    worksheet.write_number_with_format(total_row, 5, pi.total_amount, &money_format).unwrap();

    // 条款
    let terms_row = total_row + 2;
    worksheet.write_string(terms_row, 0, "Payment Terms:").unwrap();
    worksheet.write_string(terms_row, 1, &pi.payment_terms).unwrap();
    worksheet.write_string(terms_row + 1, 0, "Delivery Terms:").unwrap();
    worksheet.write_string(terms_row + 1, 1, &pi.delivery_terms).unwrap();

    // 设置列宽
    worksheet.set_column_width(0, 5).unwrap();   // #
    worksheet.set_column_width(1, 30).unwrap();  // Product Name
    worksheet.set_column_width(2, 15).unwrap();  // Model
    worksheet.set_column_width(3, 8).unwrap();   // Qty
    worksheet.set_column_width(4, 12).unwrap();  // Unit Price
    worksheet.set_column_width(5, 12).unwrap();  // Total

    // 生成 Excel 文件
    let data = workbook.save_to_buffer().unwrap();

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet")
        .header("Content-Disposition", format!("attachment; filename=\"{}.xlsx\"", pi.pi_code))
        .body(Body::from(data))
        .unwrap())
}

/// CI 下载 Excel
pub async fn ci_download_handler(
    State(state): State<AppState>,
    Extension(_auth_user): Extension<AuthUser>,
    Path(id): Path<i64>,
) -> Result<Response<Body>, Response<Body>> {
    let result = CommercialInvoiceQueries::new(state.db.pool())
        .get_detail(id)
        .await
        .map_err(|e| {
            Response::builder()
                .status(500)
                .body(Body::from(format!("获取 CI 失败: {}", e)))
                .unwrap()
        })?;

    let detail = result.ok_or_else(|| {
        Response::builder()
            .status(404)
            .body(Body::from("CI 不存在"))
            .unwrap()
    })?;

    let ci = &detail.ci;

    // 创建 Excel 工作簿
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet().set_name("CI").unwrap();

    // 标题格式
    let title_format = Format::new()
        .set_bold()
        .set_font_size(16)
        .set_align(rust_xlsxwriter::FormatAlign::Center);

    // 表头格式
    let header_format = Format::new()
        .set_bold()
        .set_background_color("#4472C4")
        .set_font_color("#FFFFFF")
        .set_border(FormatBorder::Thin)
        .set_align(rust_xlsxwriter::FormatAlign::Center);

    // 普通单元格格式
    let cell_format = Format::new()
        .set_border(FormatBorder::Thin);

    // 金额格式
    let money_format = Format::new()
        .set_border(FormatBorder::Thin)
        .set_num_format("#,##0.00");

    // 标题
    worksheet.write_string_with_format(0, 2, "COMMERCIAL INVOICE", &title_format).unwrap();

    // CI 信息
    worksheet.write_string(2, 0, "CI Number:").unwrap();
    worksheet.write_string(2, 1, &ci.ci_code).unwrap();
    worksheet.write_string(2, 3, "Date:").unwrap();
    worksheet.write_string(2, 4, &ci.ci_date).unwrap();

    // 客户信息
    worksheet.write_string(4, 0, "Buyer:").unwrap();
    worksheet.write_string(4, 1, &ci.customer_name).unwrap();
    if let Some(addr) = &ci.customer_address {
        worksheet.write_string(5, 1, addr).unwrap();
    }

    // 产品明细表头
    let headers = ["#", "Product Name", "Model", "Qty", "Unit Price", "Total"];
    for (col, header) in headers.iter().enumerate() {
        worksheet.write_string_with_format(7, col as u16, *header, &header_format).unwrap();
    }

    // 产品明细
    for (row, item) in detail.items.iter().enumerate() {
        let row_num = 8 + row as u32;
        worksheet.write_string_with_format(row_num, 0, &(row + 1).to_string(), &cell_format).unwrap();
        worksheet.write_string_with_format(row_num, 1, &item.product_name, &cell_format).unwrap();
        worksheet.write_string_with_format(row_num, 2, item.model.as_deref().unwrap_or("-"), &cell_format).unwrap();
        worksheet.write_number_with_format(row_num, 3, item.quantity as f64, &cell_format).unwrap();
        worksheet.write_number_with_format(row_num, 4, item.unit_price, &money_format).unwrap();
        worksheet.write_number_with_format(row_num, 5, item.total_price, &money_format).unwrap();
    }

    // 汇总
    let summary_row = 8 + detail.items.len() as u32 + 1;
    worksheet.write_string_with_format(summary_row, 4, "Total:", &header_format).unwrap();
    worksheet.write_number_with_format(summary_row, 5, ci.total_amount, &money_format).unwrap();

    // 设置列宽
    worksheet.set_column_width(0, 5).unwrap();
    worksheet.set_column_width(1, 30).unwrap();
    worksheet.set_column_width(2, 15).unwrap();
    worksheet.set_column_width(3, 8).unwrap();
    worksheet.set_column_width(4, 12).unwrap();
    worksheet.set_column_width(5, 12).unwrap();

    // 生成 Excel 文件
    let data = workbook.save_to_buffer().unwrap();

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet")
        .header("Content-Disposition", format!("attachment; filename=\"{}.xlsx\"", ci.ci_code))
        .body(Body::from(data))
        .unwrap())
}

// ==================== PI 创建页面 ====================

use axum::extract::Form;

/// PI 创建表单
#[derive(Debug, Deserialize)]
pub struct PICreateForm {
    pub customer_name: String,
    pub customer_email: Option<String>,
    pub customer_phone: Option<String>,
    pub customer_address: Option<String>,
    pub pi_date: String,
    pub valid_until: Option<String>,
    pub currency: Option<String>,
    pub discount: Option<String>,
    pub payment_terms: Option<String>,
    pub delivery_terms: Option<String>,
    pub lead_time: Option<String>,
    pub notes: Option<String>,
    // 产品明细
    pub product_names: String,
    pub models: String,
    pub quantities: String,
    pub unit_prices: String,
}

/// PI 创建页面
pub async fn pi_new_page(
    Extension(auth_user): Extension<AuthUser>,
) -> Html<String> {
    let user = get_user_from_extension(&auth_user);

    let content = r#"<div class="bg-white rounded-lg shadow">
        <div class="px-6 py-4 border-b">
            <h3 class="text-lg font-semibold">新建 PI</h3>
        </div>
        <form method="POST" action="/orders/pi/new" class="p-6">
            <div class="grid grid-cols-2 gap-6 mb-6">
                <!-- 客户信息 -->
                <div>
                    <h4 class="font-semibold mb-3 text-gray-700">客户信息</h4>
                    <div class="mb-3">
                        <label class="block text-sm text-gray-600 mb-1">客户名称 *</label>
                        <input type="text" name="customer_name" required class="w-full border rounded px-3 py-2">
                    </div>
                    <div class="mb-3">
                        <label class="block text-sm text-gray-600 mb-1">邮箱</label>
                        <input type="email" name="customer_email" class="w-full border rounded px-3 py-2">
                    </div>
                    <div class="mb-3">
                        <label class="block text-sm text-gray-600 mb-1">电话</label>
                        <input type="text" name="customer_phone" class="w-full border rounded px-3 py-2">
                    </div>
                    <div class="mb-3">
                        <label class="block text-sm text-gray-600 mb-1">地址</label>
                        <textarea name="customer_address" rows="2" class="w-full border rounded px-3 py-2"></textarea>
                    </div>
                </div>
                <!-- PI 信息 -->
                <div>
                    <h4 class="font-semibold mb-3 text-gray-700">PI 信息</h4>
                    <div class="mb-3">
                        <label class="block text-sm text-gray-600 mb-1">PI 日期 *</label>
                        <input type="date" name="pi_date" required class="w-full border rounded px-3 py-2">
                    </div>
                    <div class="mb-3">
                        <label class="block text-sm text-gray-600 mb-1">有效期至</label>
                        <input type="date" name="valid_until" class="w-full border rounded px-3 py-2">
                    </div>
                    <div class="mb-3">
                        <label class="block text-sm text-gray-600 mb-1">币种</label>
                        <select name="currency" class="w-full border rounded px-3 py-2">
                            <option value="USD" selected>USD</option>
                            <option value="CNY">CNY</option>
                            <option value="EUR">EUR</option>
                        </select>
                    </div>
                    <div class="mb-3">
                        <label class="block text-sm text-gray-600 mb-1">折扣</label>
                        <input type="number" name="discount" step="0.01" value="0" class="w-full border rounded px-3 py-2">
                    </div>
                </div>
            </div>
            <!-- 条款信息 -->
            <div class="grid grid-cols-2 gap-6 mb-6">
                <div class="mb-3">
                    <label class="block text-sm text-gray-600 mb-1">付款条款</label>
                    <input type="text" name="payment_terms" value="100% before shipment" class="w-full border rounded px-3 py-2">
                </div>
                <div class="mb-3">
                    <label class="block text-sm text-gray-600 mb-1">交货条款</label>
                    <input type="text" name="delivery_terms" value="EXW" class="w-full border rounded px-3 py-2">
                </div>
                <div class="mb-3">
                    <label class="block text-sm text-gray-600 mb-1">交货周期</label>
                    <input type="text" name="lead_time" value="3-7 working days" class="w-full border rounded px-3 py-2">
                </div>
                <div class="mb-3">
                    <label class="block text-sm text-gray-600 mb-1">备注</label>
                    <input type="text" name="notes" class="w-full border rounded px-3 py-2">
                </div>
            </div>
            <!-- 产品明细 -->
            <div class="mb-6">
                <h4 class="font-semibold mb-3 text-gray-700">产品明细</h4>
                <div class="mb-2 text-sm text-gray-500">每行一个产品，用换行分隔</div>
                <div class="grid grid-cols-4 gap-3">
                    <div>
                        <label class="block text-sm text-gray-600 mb-1">产品名称 *</label>
                        <textarea name="product_names" rows="5" required class="w-full border rounded px-3 py-2" placeholder="产品A&#10;产品B&#10;产品C"></textarea>
                    </div>
                    <div>
                        <label class="block text-sm text-gray-600 mb-1">型号</label>
                        <textarea name="models" rows="5" class="w-full border rounded px-3 py-2" placeholder="Model-A&#10;Model-B&#10;Model-C"></textarea>
                    </div>
                    <div>
                        <label class="block text-sm text-gray-600 mb-1">数量 *</label>
                        <textarea name="quantities" rows="5" required class="w-full border rounded px-3 py-2" placeholder="100&#10;200&#10;50"></textarea>
                    </div>
                    <div>
                        <label class="block text-sm text-gray-600 mb-1">单价 *</label>
                        <textarea name="unit_prices" rows="5" required class="w-full border rounded px-3 py-2" placeholder="10.50&#10;25.00&#10;100.00"></textarea>
                    </div>
                </div>
            </div>
            <!-- 提交按钮 -->
            <div class="flex justify-end gap-3">
                <a href="/orders/pi" class="bg-gray-500 text-white px-4 py-2 rounded hover:bg-gray-600">取消</a>
                <button type="submit" class="bg-blue-500 text-white px-4 py-2 rounded hover:bg-blue-600">创建 PI</button>
            </div>
        </form>
    </div>"#;

    render_layout("新建 PI", "pi", Some(user), content)
}

/// PI 创建处理
pub async fn pi_create_handler(
    State(state): State<AppState>,
    Extension(_auth_user): Extension<AuthUser>,
    Form(form): Form<PICreateForm>,
) -> impl IntoResponse {
    // 获取成交时汇率快照
    let snapshot_rate = {
        let eq = ExchangeRateQueries::new(state.db.pool());
        let rate = eq.get_latest_rate("USD", "CNY").await
            .ok().flatten().map(|r| r.rate).unwrap_or(7.2);
        ((rate - 0.05) * 100.0).round() / 100.0
    };

    // 解析产品明细
    let names: Vec<&str> = form.product_names.lines().collect();
    let models: Vec<&str> = form.models.lines().collect();
    let quantities: Vec<&str> = form.quantities.lines().collect();
    let prices: Vec<&str> = form.unit_prices.lines().collect();

    let mut items = Vec::new();
    for (i, name) in names.iter().enumerate() {
        if name.trim().is_empty() {
            continue;
        }
        let qty = quantities.get(i).and_then(|s| s.trim().parse::<i64>().ok()).unwrap_or(0);
        let price = prices.get(i).and_then(|s| s.trim().parse::<f64>().ok()).unwrap_or(0.0);
        items.push(cicierp_models::proforma_invoice::PIItemRequest {
            product_id: None,
            product_name: name.trim().to_string(),
            model: models.get(i).map(|s| s.trim().to_string()),
            quantity: qty,
            unit_price: price,
            notes: None,
            sort_order: Some(i as i64),
        });
    }

    if items.is_empty() {
        return Redirect::to("/orders/pi/new");
    }

    let request = cicierp_models::proforma_invoice::CreatePIRequest {
        customer_id: None,
        customer_name: form.customer_name,
        customer_email: form.customer_email,
        customer_phone: form.customer_phone,
        customer_address: form.customer_address,
        seller_name: None,
        seller_address: None,
        seller_phone: None,
        seller_email: None,
        currency: form.currency,
        discount: form.discount.and_then(|s| s.parse().ok()),
        pi_date: form.pi_date,
        valid_until: form.valid_until,
        payment_terms: form.payment_terms,
        delivery_terms: form.delivery_terms,
        lead_time: form.lead_time,
        notes: form.notes,
        items,
        exchange_rate: Some(snapshot_rate),  // 成交时汇率快照
    };

    match ProformaInvoiceQueries::new(state.db.pool()).create(&request).await {
        Ok(pi) => Redirect::to(&format!("/orders/pi/{}", pi.id)),
        Err(e) => {
            tracing::error!("创建 PI 失败: {}", e);
            Redirect::to("/orders/pi")
        }
    }
}

// ==================== PI 操作 Handler ====================

/// PI 发送处理
pub async fn pi_send_handler(
    State(state): State<AppState>,
    Extension(_auth_user): Extension<AuthUser>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    match ProformaInvoiceQueries::new(state.db.pool()).send(id).await {
        Ok(true) => Redirect::to(&format!("/orders/pi/{}", id)),
        Ok(false) => {
            tracing::warn!("PI {} 发送失败：状态不允许", id);
            Redirect::to(&format!("/orders/pi/{}", id))
        }
        Err(e) => {
            tracing::error!("PI {} 发送失败: {}", id, e);
            Redirect::to(&format!("/orders/pi/{}", id))
        }
    }
}

/// PI 确认处理
pub async fn pi_confirm_handler(
    State(state): State<AppState>,
    Extension(_auth_user): Extension<AuthUser>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    match ProformaInvoiceQueries::new(state.db.pool()).confirm(id).await {
        Ok(true) => Redirect::to(&format!("/orders/pi/{}", id)),
        Ok(false) => {
            tracing::warn!("PI {} 确认失败：状态不允许", id);
            Redirect::to(&format!("/orders/pi/{}", id))
        }
        Err(e) => {
            tracing::error!("PI {} 确认失败: {}", id, e);
            Redirect::to(&format!("/orders/pi/{}", id))
        }
    }
}

/// PI 取消处理
pub async fn pi_cancel_handler(
    State(state): State<AppState>,
    Extension(_auth_user): Extension<AuthUser>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    match ProformaInvoiceQueries::new(state.db.pool()).cancel(id).await {
        Ok(true) => Redirect::to(&format!("/orders/pi/{}", id)),
        Ok(false) => {
            tracing::warn!("PI {} 取消失败：状态不允许", id);
            Redirect::to(&format!("/orders/pi/{}", id))
        }
        Err(e) => {
            tracing::error!("PI {} 取消失败: {}", id, e);
            Redirect::to(&format!("/orders/pi/{}", id))
        }
    }
}

/// PI 转订单处理
pub async fn pi_convert_handler(
    State(state): State<AppState>,
    Extension(_auth_user): Extension<AuthUser>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    // 获取 PI 详情
    let pi_detail = match ProformaInvoiceQueries::new(state.db.pool()).get_detail(id).await {
        Ok(Some(d)) => d,
        Ok(None) => {
            tracing::error!("PI {} 不存在", id);
            return Redirect::to("/orders/pi");
        }
        Err(e) => {
            tracing::error!("获取 PI {} 失败: {}", id, e);
            return Redirect::to("/orders/pi");
        }
    };

    let pi = &pi_detail.pi;

    // 检查状态是否为已确认
    if pi.status != 3 {
        tracing::warn!("PI {} 状态不是已确认，无法转订单", id);
        return Redirect::to(&format!("/orders/pi/{}", id));
    }

    // 创建订单请求
    let order_items: Vec<cicierp_models::order::OrderItemRequest> = pi_detail.items
        .iter()
        .map(|item| cicierp_models::order::OrderItemRequest {
            product_id: item.product_id,
            sku_id: None,
            product_name: item.product_name.clone(),
            product_code: None,
            sku_code: item.model.clone(),
            sku_spec: None,
            product_image: None,
            quantity: item.quantity,
            unit_price: item.unit_price,
        })
        .collect();

    // 解析客户地址
    let address_parts: Vec<&str> = pi.customer_address.as_deref().unwrap_or("").split(',').collect();
    let country = address_parts.get(0).unwrap_or(&"CN").to_string();
    let province = address_parts.get(1).map(|s| s.to_string());
    let city = address_parts.get(2).map(|s| s.to_string());
    let address = address_parts.get(3).unwrap_or(&"").to_string();

    let order_request = cicierp_models::order::CreateOrderRequest {
        platform: "PI".to_string(),
        platform_order_id: Some(pi.pi_code.clone()),
        customer_id: pi.customer_id,
        customer_name: Some(pi.customer_name.clone()),
        customer_mobile: pi.customer_phone.clone(),
        customer_email: pi.customer_email.clone(),
        order_type: Some(1), // 普通订单
        items: order_items,
        shipping_fee: Some(0.0),
        discount_amount: Some(pi.discount),
        customer_note: pi.notes.clone(),
        receiver_name: pi.customer_name.clone(),
        receiver_phone: pi.customer_phone.clone().unwrap_or_default(),
        country,
        province,
        city,
        district: None,
        address,
        postal_code: None,
        payment_terms: Some(pi.payment_terms.clone()),
        delivery_terms: Some(pi.delivery_terms.clone()),
        lead_time: Some(pi.lead_time.clone()),
        exchange_rate: pi.exchange_rate,  // 沿用 PI 的汇率快照
        currency: Some(pi.currency.clone()),   // 沿用 PI 的货币
    };

    // 使用 OrderQueries 创建订单
    use cicierp_db::queries::orders::OrderQueries;

    match OrderQueries::new(state.db.pool()).create(&order_request).await {
        Ok(order) => {
            // 更新 PI 状态为已转订单
            match ProformaInvoiceQueries::new(state.db.pool()).mark_converted(id, order.id).await {
                Ok(_) => Redirect::to(&format!("/orders/{}", order.id)),
                Err(e) => {
                    tracing::error!("更新 PI {} 状态失败: {}", id, e);
                    Redirect::to(&format!("/orders/{}", order.id))
                }
            }
        }
        Err(e) => {
            tracing::error!("从 PI {} 创建订单失败: {}", id, e);
            Redirect::to(&format!("/orders/pi/{}", id))
        }
    }
}

// ==================== CI 操作 Handler ====================

/// CI 标记已付款表单
#[derive(Debug, Deserialize)]
pub struct CIMarkPaidForm {
    pub paid_amount: String,
}

/// CI 标记已付款处理
pub async fn ci_mark_paid_handler(
    State(state): State<AppState>,
    Extension(_auth_user): Extension<AuthUser>,
    Path(id): Path<i64>,
    Form(form): Form<CIMarkPaidForm>,
) -> impl IntoResponse {
    let paid_amount: f64 = form.paid_amount.parse().unwrap_or(0.0);

    let request = cicierp_models::commercial_invoice::MarkPaidRequest {
        paid_amount,
        paid_at: None,
    };

    match CommercialInvoiceQueries::new(state.db.pool()).mark_paid(id, &request).await {
        Ok(true) => Redirect::to(&format!("/orders/ci/{}", id)),
        Ok(false) => {
            tracing::warn!("CI {} 标记已付款失败：状态不允许", id);
            Redirect::to(&format!("/orders/ci/{}", id))
        }
        Err(e) => {
            tracing::error!("CI {} 标记已付款失败: {}", id, e);
            Redirect::to(&format!("/orders/ci/{}", id))
        }
    }
}

// ==================== 路由注册 ====================

/// PI/CI Web 路由（整合到订单管理模块）
pub fn router() -> Router<AppState> {
    Router::new()
        // PI 管理（形式发票）- 订单模块子功能
        .route("/orders/pi", get(pi_list_page))
        .route("/orders/pi/new", get(pi_new_page).post(pi_create_handler))
        .route("/orders/pi/:id", get(pi_detail_page))
        .route("/orders/pi/:id/download", get(pi_download_handler))
        .route("/orders/pi/:id/send", post(pi_send_handler))
        .route("/orders/pi/:id/confirm", post(pi_confirm_handler))
        .route("/orders/pi/:id/convert", post(pi_convert_handler))
        .route("/orders/pi/:id/cancel", post(pi_cancel_handler))
        // CI 管理（商业发票）- 订单模块子功能
        .route("/orders/ci", get(ci_list_page))
        .route("/orders/ci/:id", get(ci_detail_page))
        .route("/orders/ci/:id/download", get(ci_download_handler))
        .route("/orders/ci/:id/mark-paid", post(ci_mark_paid_handler))
}
