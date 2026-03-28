//! ciciERP API 服务
//!
//! 基于 Axum 的 RESTful API 服务

mod middleware;
mod routes;
mod state;
mod templates;

use std::net::SocketAddr;
use std::time::Duration;

use axum::http::{self, HeaderValue};
use tokio::time::interval;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::{info, warn, error, Level};
use tracing_subscriber::FmtSubscriber;

use cicierp_db::{Database, DatabaseConfig};
use routes::create_router;
use routes::exchange_rates::fetch_exchange_rate_from_api;
use state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .with_thread_ids(false)
        .pretty()
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting ciciERP API server...");

    // 加载配置
    let db_config = DatabaseConfig::default();
    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());

    // 连接数据库
    let db = Database::new(&db_config).await?;
    db.run_migrations().await?;

    // 创建应用状态
    let state = AppState::new(db.clone());

    // 启动汇率定时更新任务
    let pool = db.pool().clone();
    tokio::spawn(async move {
        // 启动时立即获取一次汇率
        info!("Fetching initial exchange rate...");
        if let Err(e) = fetch_exchange_rate_from_api(&pool).await {
            error!("Failed to fetch initial exchange rate: {}", e);
        }

        // 每 24 小时更新一次（在早上 6:00 左右）
        // 这里使用简单的 interval，实际生产环境应该使用 cron 调度
        let mut interval = interval(Duration::from_secs(24 * 60 * 60));

        loop {
            interval.tick().await;
            info!("Auto-updating exchange rate...");
            if let Err(e) = fetch_exchange_rate_from_api(&pool).await {
                error!("Failed to auto-update exchange rate: {}", e);
            }
        }
    });

    // 配置 CORS
    let cors = configure_cors();

    // 创建路由（添加请求追踪中间件）
    // 注意：create_router 现在需要 state 参数来支持认证中间件
    let app = create_router(state.clone())
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().include_headers(true))
                .on_response(DefaultOnResponse::new().include_headers(true))
                .on_failure(tower_http::trace::DefaultOnFailure::new())
        )
        .layer(cors)
        .with_state(state);

    // 启动服务器
    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;
    info!("Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// 配置 CORS
///
/// 从环境变量 CORS_ORIGINS 读取允许的域名，多个域名用逗号分隔。
/// 如果未设置，开发环境允许所有来源，生产环境只允许同源请求。
fn configure_cors() -> CorsLayer {
    let cors_origins = std::env::var("CORS_ORIGINS").ok();

    match cors_origins {
        Some(origins) if !origins.is_empty() => {
            // 解析允许的域名列表
            let allowed_origins: Vec<String> = origins
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            if allowed_origins.is_empty() {
                warn!("CORS_ORIGINS is set but empty, using restrictive CORS");
                CorsLayer::new()
                    .allow_origin(tower_http::cors::AllowOrigin::exact("http://localhost".parse().unwrap()))
            } else {
                info!("CORS allowed origins: {:?}", allowed_origins);
                // 解析为 HeaderValue
                let origins: Vec<HeaderValue> = allowed_origins
                    .iter()
                    .filter_map(|o| o.parse().ok())
                    .collect();

                CorsLayer::new()
                    .allow_origin(tower_http::cors::AllowOrigin::list(origins))
                    .allow_methods([http::Method::GET, http::Method::POST, http::Method::PUT, http::Method::DELETE, http::Method::OPTIONS])
                    .allow_headers([http::header::AUTHORIZATION, http::header::CONTENT_TYPE, http::header::ACCEPT])
            }
        }
        _ => {
            // 开发环境：允许所有来源（仅当 NODE_ENV 不是 production 时）
            let is_production = std::env::var("NODE_ENV").map(|v| v == "production").unwrap_or(false);

            if is_production {
                warn!("Running in production mode without CORS_ORIGINS, CORS is restrictive");
                CorsLayer::new()
                    .allow_origin(tower_http::cors::AllowOrigin::exact("http://localhost".parse().unwrap()))
            } else {
                warn!("Running in development mode, CORS allows all origins");
                CorsLayer::new()
                    .allow_origin(Any)
                    .allow_methods(Any)
                    .allow_headers(Any)
            }
        }
    }
}
