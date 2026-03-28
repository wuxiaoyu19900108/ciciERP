//! 汇率相关数据模型

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;

/// 汇率记录
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ExchangeRate {
    pub id: i64,
    pub from_currency: String,  // 源货币：USD
    pub to_currency: String,    // 目标货币：CNY
    pub rate: f64,              // 汇率值
    pub source: String,         // 来源：api/manual
    pub effective_date: String, // 生效日期 YYYY-MM-DD
    pub created_at: String,
}

/// 创建汇率请求
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateExchangeRateRequest {
    #[validate(length(min = 3, max = 3))]
    pub from_currency: String,
    #[validate(length(min = 3, max = 3))]
    pub to_currency: String,
    #[validate(range(min = 0.0))]
    pub rate: f64,
    pub source: Option<String>,
    pub effective_date: Option<String>,
}

/// 手动更新汇率请求
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ManualUpdateRateRequest {
    #[validate(range(min = 0.0))]
    pub rate: f64,
    pub from_currency: Option<String>,
    pub to_currency: Option<String>,
}

/// 汇率历史查询参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeRateHistoryQuery {
    pub from_currency: Option<String>,
    pub to_currency: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub limit: Option<i64>,
}

/// 汇率 API 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeRateApiResponse {
    pub from_currency: String,
    pub to_currency: String,
    pub rate: f64,
    pub source: String,
    pub effective_date: String,
    pub is_today: bool,
}
