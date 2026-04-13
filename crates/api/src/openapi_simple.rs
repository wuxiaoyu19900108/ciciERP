//! OpenAPI 文档配置 - 简化版本
//!
//! 使用 utoipa 自动生成 API 文档

use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

/// OpenAPI 文档结构
#[derive(OpenApi)]
#[openapi(
    info(
        title = "ciciERP API",
        version = "1.0.0",
        description = "ciciERP 企业资源规划系统 API 文档",
    ),
    tags(
        (name = "products", description = "产品管理 API"),
        (name = "orders", description = "订单管理 API"),
        (name = "customers", description = "客户管理 API"),
    )
)]
pub struct ApiDoc;

/// 创建 Swagger UI 路由
pub fn create_swagger_router() -> SwaggerUi {
    SwaggerUi::new("/swagger-ui")
        .url("/api-docs/openapi.json", ApiDoc::openapi())
}
