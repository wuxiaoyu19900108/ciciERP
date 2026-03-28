//! 汇率 API 路由

use axum::{
    extract::{Query, State},
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use reqwest::Client;
use serde::Deserialize;
use tracing::{info, error, instrument};

use crate::state::AppState;
use cicierp_db::queries::exchange_rates::ExchangeRateQueries;
use cicierp_models::exchange_rate::{
    ExchangeRate, ExchangeRateApiResponse, ExchangeRateHistoryQuery,
    CreateExchangeRateRequest, ManualUpdateRateRequest,
};
use cicierp_utils::{AppError, AppResult, ApiResponse};

/// 外部 API 汇率响应
#[derive(Debug, Deserialize)]
struct ExternalRateResponse {
    rates: std::collections::HashMap<String, f64>,
}

/// 创建汇率路由
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/exchange-rates/current", get(get_current_rate))
        .route("/exchange-rates/update", post(manual_update_rate))
        .route("/exchange-rates/history", get(get_rate_history))
        .route("/exchange-rates/fetch", post(fetch_and_save_rate))
}

/// @api GET /api/v1/exchange-rates/current
/// @desc 获取当前汇率
/// @query from_currency: 源货币(默认USD)
/// @query to_currency: 目标货币(默认CNY)
/// @response 200 ExchangeRateApiResponse
#[instrument(skip(state))]
pub async fn get_current_rate(
    State(state): State<AppState>,
    Query(params): Query<CurrentRateQuery>,
) -> AppResult<Json<ApiResponse<ExchangeRateApiResponse>>> {
    let from = params.from_currency.unwrap_or_else(|| "USD".to_string());
    let to = params.to_currency.unwrap_or_else(|| "CNY".to_string());

    info!("Getting current exchange rate: {} -> {}", from, to);

    let queries = ExchangeRateQueries::new(state.db.pool());
    let today = Utc::now().format("%Y-%m-%d").to_string();

    match queries.get_current_rate(&from, &to).await? {
        Some(rate) => {
            let response = ExchangeRateApiResponse {
                from_currency: rate.from_currency,
                to_currency: rate.to_currency,
                rate: rate.rate,
                source: rate.source,
                effective_date: rate.effective_date.clone(),
                is_today: rate.effective_date == today,
            };
            Ok(Json(ApiResponse::success(response)))
        }
        None => Err(AppError::NotFound),
    }
}

/// @api POST /api/v1/exchange-rates/update
/// @desc 手动更新汇率
/// @body ManualUpdateRateRequest
/// @response 200 ExchangeRate
#[instrument(skip(state))]
pub async fn manual_update_rate(
    State(state): State<AppState>,
    Json(req): Json<ManualUpdateRateRequest>,
) -> AppResult<Json<ApiResponse<ExchangeRate>>> {
    info!("Manual updating exchange rate: {:?}", req);

    let from = req.from_currency.unwrap_or_else(|| "USD".to_string());
    let to = req.to_currency.unwrap_or_else(|| "CNY".to_string());

    let queries = ExchangeRateQueries::new(state.db.pool());
    let create_req = CreateExchangeRateRequest {
        from_currency: from,
        to_currency: to,
        rate: req.rate,
        source: Some("manual".to_string()),
        effective_date: Some(Utc::now().format("%Y-%m-%d").to_string()),
    };

    let rate = queries.create(&create_req).await?;
    Ok(Json(ApiResponse::success(rate)))
}

/// @api GET /api/v1/exchange-rates/history
/// @desc 获取汇率历史
/// @query ExchangeRateHistoryQuery
/// @response 200 Vec<ExchangeRate>
#[instrument(skip(state))]
pub async fn get_rate_history(
    State(state): State<AppState>,
    Query(query): Query<ExchangeRateHistoryQuery>,
) -> AppResult<Json<ApiResponse<Vec<ExchangeRate>>>> {
    info!("Getting exchange rate history: {:?}", query);

    let queries = ExchangeRateQueries::new(state.db.pool());
    let rates = queries.list_history(&query).await?;

    Ok(Json(ApiResponse::success(rates)))
}

/// @api POST /api/v1/exchange-rates/fetch
/// @desc 从外部 API 获取并保存汇率
/// @response 200 ExchangeRate
#[instrument(skip(state))]
pub async fn fetch_and_save_rate(
    State(state): State<AppState>,
) -> AppResult<Json<ApiResponse<ExchangeRate>>> {
    info!("Fetching exchange rate from external API");

    // 调用外部 API
    let client = Client::new();
    let response = client
        .get("https://open.er-api.com/v6/latest/USD")
        .send()
        .await
        .map_err(|e| {
            error!("Failed to fetch exchange rate: {}", e);
            anyhow::anyhow!("Failed to fetch exchange rate: {}", e)
        })?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "External API returned status: {}",
            response.status()
        ).into());
    }

    let data: ExternalRateResponse = response.json().await.map_err(|e| {
        error!("Failed to parse exchange rate response: {}", e);
        anyhow::anyhow!("Failed to parse response: {}", e)
    })?;

    // 获取 USD -> CNY 汇率
    let cny_rate = data.rates.get("CNY").ok_or_else(|| {
        anyhow::anyhow!("CNY rate not found in response")
    })?;

    info!("Fetched USD -> CNY rate: {}", cny_rate);

    // 保存到数据库
    let queries = ExchangeRateQueries::new(state.db.pool());
    let create_req = CreateExchangeRateRequest {
        from_currency: "USD".to_string(),
        to_currency: "CNY".to_string(),
        rate: *cny_rate,
        source: Some("api".to_string()),
        effective_date: Some(Utc::now().format("%Y-%m-%d").to_string()),
    };

    let rate = queries.create(&create_req).await?;
    Ok(Json(ApiResponse::success(rate)))
}

/// 从外部 API 获取汇率（内部使用，供定时任务调用）
pub async fn fetch_exchange_rate_from_api(pool: &sqlx::SqlitePool) -> anyhow::Result<ExchangeRate> {
    let client = Client::new();
    let response = client
        .get("https://open.er-api.com/v6/latest/USD")
        .send()
        .await?;

    if !response.status().is_success() {
        anyhow::bail!("External API returned status: {}", response.status());
    }

    let data: ExternalRateResponse = response.json().await?;
    let cny_rate = data.rates.get("CNY")
        .ok_or_else(|| anyhow::anyhow!("CNY rate not found in response"))?;

    info!("Auto-fetched USD -> CNY rate: {}", cny_rate);

    let queries = ExchangeRateQueries::new(pool);
    let create_req = CreateExchangeRateRequest {
        from_currency: "USD".to_string(),
        to_currency: "CNY".to_string(),
        rate: *cny_rate,
        source: Some("api".to_string()),
        effective_date: Some(Utc::now().format("%Y-%m-%d").to_string()),
    };

    queries.create(&create_req).await
}

// 查询参数结构
#[derive(Debug, Deserialize)]
struct CurrentRateQuery {
    from_currency: Option<String>,
    to_currency: Option<String>,
}
