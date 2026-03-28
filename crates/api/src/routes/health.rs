//! 健康检查

use axum::{extract::State, Json};
use serde_json::json;

use crate::state::AppState;
use cicierp_utils::AppResult;

/// @api GET /health
/// @desc 健康检查接口
/// @response 200 {"status": "ok", "database": "ok"}
/// @example curl -X GET "http://localhost:3000/health"
pub async fn health_check(State(state): State<AppState>) -> AppResult<Json<serde_json::Value>> {
    let db_ok = state.db.health_check().await.unwrap_or(false);

    Ok(Json(json!({
        "status": "ok",
        "database": if db_ok { "ok" } else { "error" },
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}
