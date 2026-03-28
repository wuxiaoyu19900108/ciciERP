//! 物流模块数据模型

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

/// 发货单状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShipmentStatus {
    #[serde(rename = "1")]
    Shipped = 1,      // 已发货
    #[serde(rename = "2")]
    InTransit = 2,    // 运输中
    #[serde(rename = "3")]
    Delivered = 3,    // 已签收
    #[serde(rename = "4")]
    Exception = 4,    // 异常
    #[serde(rename = "5")]
    Returned = 5,     // 已退货
}

impl Default for ShipmentStatus {
    fn default() -> Self {
        ShipmentStatus::Shipped
    }
}

/// 物流服务类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceType {
    #[serde(rename = "express")]
    Express, // 快递
    #[serde(rename = "air")]
    Air,     // 空运
    #[serde(rename = "sea")]
    Sea,     // 海运
    #[serde(rename = "land")]
    Land,    // 陆运
}

impl Default for ServiceType {
    fn default() -> Self {
        ServiceType::Express
    }
}

/// 物流公司实体
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct LogisticsCompany {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub name_en: Option<String>,
    pub service_type: String,
    pub api_code: Option<String>,
    pub api_config: Option<String>,
    pub contact_phone: Option<String>,
    pub contact_email: Option<String>,
    pub website: Option<String>,
    pub tracking_url_template: Option<String>,
    pub status: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// 发货单实体
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Shipment {
    pub id: i64,
    pub shipment_code: String,
    pub order_id: i64,
    pub logistics_id: Option<i64>,
    pub logistics_name: Option<String>,
    pub tracking_number: Option<String>,
    pub receiver_name: String,
    pub receiver_phone: String,
    pub receiver_address: String,
    pub package_weight: Option<f64>,
    pub package_volume: Option<f64>,
    pub package_items: String,
    pub package_count: i64,
    pub shipping_fee: f64,
    pub actual_shipping_fee: Option<f64>,
    pub estimated_arrival: Option<String>,
    pub actual_arrival: Option<String>,
    pub status: i64,
    pub shipping_note: Option<String>,
    pub ship_time: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 物流轨迹实体
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ShipmentTracking {
    pub id: i64,
    pub shipment_id: i64,
    pub tracking_time: String,
    pub tracking_status: String,
    pub tracking_description: String,
    pub location: Option<String>,
    pub raw_data: Option<String>,
    pub created_at: String,
}

// ============================================================================
// 请求/响应 DTOs
// ============================================================================

/// 创建物流公司请求
#[derive(Debug, Deserialize, Validate)]
pub struct CreateLogisticsCompanyRequest {
    #[validate(length(min = 1, max = 50, message = "编码长度1-50"))]
    pub code: String,
    #[validate(length(min = 1, max = 100, message = "名称长度1-100"))]
    pub name: String,
    pub name_en: Option<String>,
    #[validate(length(min = 1, message = "服务类型不能为空"))]
    pub service_type: String,
    pub api_code: Option<String>,
    pub api_config: Option<String>,
    pub contact_phone: Option<String>,
    pub contact_email: Option<String>,
    pub website: Option<String>,
    pub tracking_url_template: Option<String>,
}

/// 更新物流公司请求
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateLogisticsCompanyRequest {
    pub name: Option<String>,
    pub name_en: Option<String>,
    pub service_type: Option<String>,
    pub api_code: Option<String>,
    pub api_config: Option<String>,
    pub contact_phone: Option<String>,
    pub contact_email: Option<String>,
    pub website: Option<String>,
    pub tracking_url_template: Option<String>,
    pub status: Option<i64>,
}

/// 包裹商品项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageItem {
    pub sku_id: Option<i64>,
    pub sku_code: Option<String>,
    pub product_name: String,
    pub quantity: i64,
}

/// 创建发货单请求
#[derive(Debug, Deserialize, Validate)]
pub struct CreateShipmentRequest {
    #[validate(range(min = 1, message = "订单ID无效"))]
    pub order_id: i64,
    pub logistics_id: Option<i64>,
    pub logistics_name: Option<String>,
    #[validate(length(min = 1, message = "物流单号不能为空"))]
    pub tracking_number: String,
    #[validate(length(min = 1, message = "包裹商品不能为空"))]
    pub package_items: Vec<PackageItem>,
    pub package_weight: Option<f64>,
    pub package_volume: Option<f64>,
    pub package_count: Option<i64>,
    pub shipping_fee: Option<f64>,
    pub estimated_arrival: Option<String>,
    pub shipping_note: Option<String>,
}

/// 更新发货单请求
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateShipmentRequest {
    pub logistics_id: Option<i64>,
    pub logistics_name: Option<String>,
    pub tracking_number: Option<String>,
    pub package_weight: Option<f64>,
    pub package_volume: Option<f64>,
    pub shipping_fee: Option<f64>,
    pub actual_shipping_fee: Option<f64>,
    pub estimated_arrival: Option<String>,
    pub actual_arrival: Option<String>,
    pub status: Option<i64>,
    pub shipping_note: Option<String>,
}

/// 添加物流轨迹请求
#[derive(Debug, Deserialize, Validate)]
pub struct AddTrackingRequest {
    #[validate(length(min = 1, message = "轨迹时间不能为空"))]
    pub tracking_time: String,
    #[validate(length(min = 1, message = "轨迹状态不能为空"))]
    pub tracking_status: String,
    #[validate(length(min = 1, message = "轨迹描述不能为空"))]
    pub tracking_description: String,
    pub location: Option<String>,
    pub raw_data: Option<String>,
}

/// 发货单查询参数
#[derive(Debug, Deserialize)]
pub struct ShipmentQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub order_id: Option<i64>,
    pub logistics_id: Option<i64>,
    pub status: Option<i64>,
    pub tracking_number: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
}

impl ShipmentQuery {
    pub fn page(&self) -> u32 {
        self.page.unwrap_or(1).max(1)
    }

    pub fn page_size(&self) -> u32 {
        self.page_size.unwrap_or(20).min(100).max(1)
    }
}

/// 发货单详情（包含轨迹）
#[derive(Debug, Serialize)]
pub struct ShipmentDetail {
    #[serde(flatten)]
    pub shipment: Shipment,
    pub tracking: Vec<ShipmentTracking>,
}

/// 发货单列表项
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ShipmentListItem {
    pub id: i64,
    pub shipment_code: String,
    pub order_id: i64,
    pub logistics_name: Option<String>,
    pub tracking_number: Option<String>,
    pub receiver_name: String,
    pub receiver_phone: String,
    pub status: i64,
    pub ship_time: Option<String>,
    pub created_at: String,
}

/// 获取发货单状态文本
pub fn shipment_status_text(status: i64) -> &'static str {
    match status {
        1 => "已发货",
        2 => "运输中",
        3 => "已签收",
        4 => "异常",
        5 => "已退货",
        _ => "未知",
    }
}

/// 获取发货单状态样式类
pub fn shipment_status_class(status: i64) -> &'static str {
    match status {
        1 => "bg-blue-100 text-blue-700",
        2 => "bg-purple-100 text-purple-700",
        3 => "bg-green-100 text-green-700",
        4 => "bg-red-100 text-red-700",
        5 => "bg-gray-100 text-gray-600",
        _ => "bg-gray-100 text-gray-600",
    }
}
