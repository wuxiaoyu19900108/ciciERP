//! 物流管理模板数据结构

use cicierp_models::logistics::{LogisticsCompany, Shipment, ShipmentTracking};

use super::base::{MenuItem, PageInfo};

/// 物流公司列表项
#[derive(Debug, Clone)]
pub struct LogisticsCompanyItem {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub name_en: Option<String>,
    pub service_type: String,
    pub contact_phone: Option<String>,
    pub status: i64,
}

impl From<&LogisticsCompany> for LogisticsCompanyItem {
    fn from(company: &LogisticsCompany) -> Self {
        Self {
            id: company.id,
            code: company.code.clone(),
            name: company.name.clone(),
            name_en: company.name_en.clone(),
            service_type: company.service_type.clone(),
            contact_phone: company.contact_phone.clone(),
            status: company.status,
        }
    }
}

/// 发货单列表项
#[derive(Debug, Clone)]
pub struct ShipmentItem {
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

impl From<&Shipment> for ShipmentItem {
    fn from(shipment: &Shipment) -> Self {
        Self {
            id: shipment.id,
            shipment_code: shipment.shipment_code.clone(),
            order_id: shipment.order_id,
            logistics_name: shipment.logistics_name.clone(),
            tracking_number: shipment.tracking_number.clone(),
            receiver_name: shipment.receiver_name.clone(),
            receiver_phone: shipment.receiver_phone.clone(),
            status: shipment.status,
            ship_time: shipment.ship_time.clone(),
            created_at: shipment.created_at.clone(),
        }
    }
}

/// 物流轨迹项
#[derive(Debug, Clone)]
pub struct TrackingItem {
    pub id: i64,
    pub tracking_time: String,
    pub tracking_status: String,
    pub tracking_description: String,
    pub location: Option<String>,
}

impl From<&ShipmentTracking> for TrackingItem {
    fn from(tracking: &ShipmentTracking) -> Self {
        Self {
            id: tracking.id,
            tracking_time: tracking.tracking_time.clone(),
            tracking_status: tracking.tracking_status.clone(),
            tracking_description: tracking.tracking_description.clone(),
            location: tracking.location.clone(),
        }
    }
}

/// 物流公司列表页面数据
#[derive(Debug, Clone)]
pub struct LogisticsCompaniesPage {
    pub page_info: PageInfo,
    pub menu_items: Vec<MenuItem>,
    pub current_path: &'static str,
    pub companies: Vec<LogisticsCompanyItem>,
}

/// 发货单列表页面数据
#[derive(Debug, Clone)]
pub struct ShipmentsPage {
    pub page_info: PageInfo,
    pub menu_items: Vec<MenuItem>,
    pub current_path: &'static str,
    pub shipments: Vec<ShipmentItem>,
    pub page: u32,
    pub page_size: u32,
    pub total: u64,
    pub total_pages: u32,
}

/// 发货单详情页面数据
#[derive(Debug, Clone)]
pub struct ShipmentDetailPage {
    pub page_info: PageInfo,
    pub menu_items: Vec<MenuItem>,
    pub current_path: &'static str,
    pub shipment: Shipment,
    pub tracking: Vec<TrackingItem>,
}

/// 发货单状态文本
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

/// 发货单状态样式类
pub fn shipment_status_class(status: i64) -> &'static str {
    match status {
        1 => "bg-blue-100 text-blue-700",
        2 => "bg-purple-100 text-purple-700",
        3 => "bg-green-100 text-green-700",
        4 => "bg-red-100 text-red-700",
        5 => "bg-gray-100 text-gray-700",
        _ => "bg-gray-100 text-gray-600",
    }
}

/// 服务类型文本
pub fn service_type_text(service_type: &str) -> &'static str {
    match service_type {
        "express" => "快递",
        "air" => "空运",
        "sea" => "海运",
        "land" => "陆运",
        _ => "其他",
    }
}
