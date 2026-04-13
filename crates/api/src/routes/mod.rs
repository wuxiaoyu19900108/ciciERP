//! API 路由模块

pub mod brands;
pub mod categories;
pub mod products;
pub mod product_costs;
pub mod product_prices;
pub mod product_content;
pub mod suppliers;
pub mod customers;
pub mod orders;
pub mod inventory;
pub mod health;
pub mod auth;
pub mod users;
pub mod web;
pub mod purchases;
pub mod logistics;
pub mod exchange_rates;
pub mod integration;
pub mod proforma_invoices;
pub mod commercial_invoices;
pub mod web_invoice;

use axum::{
    middleware::from_fn_with_state,
    routing::get,
    Router,
};

use crate::middleware::auth::auth_middleware;
use crate::middleware::integration_auth::integration_auth_middleware;
use crate::state::AppState;

/// 创建 API 路由
pub fn create_router(state: AppState) -> Router<AppState> {
    // Web 页面公开路由（无需认证）
    let web_public = web::public_router();

    // Web 页面受保护路由（需要认证，支持 Cookie）
    let web_protected = web::protected_router()
        .route_layer(from_fn_with_state(state.clone(), auth_middleware));

    Router::new()
        // 健康检查（无需认证）
        .route("/health", get(health::health_check))
        // Web 页面路由
        .merge(web_public)
        .merge(web_protected)
        // API v1（传入状态用于认证中间件）
        .nest("/api/v1", api_v1_router(state))
}

fn api_v1_router(state: AppState) -> Router<AppState> {
    // 无需认证的路由
    let public_routes = Router::new()
        .merge(auth::router());

    // 需要认证的路由
    let protected_routes = Router::new()
        // 认证相关（需要登录）
        .merge(auth::protected_router())
        // 用户管理模块
        .merge(users::router())
        // 产品模块
        .merge(products::router())
        // 品牌模块
        .merge(brands::router())
        // 分类模块
        .merge(categories::router())
        // 产品成本模块
        .merge(product_costs::router())
        // 产品价格模块
        .merge(product_prices::router())
        // 产品内容模块
        .merge(product_content::router())
        // 供应商模块
        .merge(suppliers::router())
        // 客户模块
        .merge(customers::router())
        // 订单模块
        .merge(orders::router())
        // 库存模块
        .merge(inventory::router())
        // 采购模块
        .merge(purchases::router())
        // 物流模块
        .merge(logistics::router())
        // 汇率模块
        .merge(exchange_rates::router())
        // PI 模块
        .merge(proforma_invoices::router())
        // CI 模块
        .merge(commercial_invoices::router())
        // 应用认证中间件（使用 from_fn_with_state 传入状态）
        .route_layer(from_fn_with_state(state.clone(), auth_middleware));

    // 对接 API 路由（使用集成认证中间件）
    let integration_routes = Router::new()
        .nest("/integration", integration::router())
        .route_layer(from_fn_with_state(state.clone(), integration_auth_middleware));

    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .merge(integration_routes)
}
