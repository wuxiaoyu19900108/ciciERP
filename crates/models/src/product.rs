//! 产品相关数据模型

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use validator::Validate;

/// 产品状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "lowercase")]
pub enum ProductStatus {
    #[serde(rename = "1")]
    Active = 1,    // 上架
    #[serde(rename = "2")]
    Inactive = 2,  // 下架
    #[serde(rename = "3")]
    Draft = 3,     // 草稿
}

impl Default for ProductStatus {
    fn default() -> Self {
        ProductStatus::Active
    }
}

/// 产品实体（不包含价格字段，价格独立管理）
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Product {
    pub id: i64,
    pub product_code: String,
    pub name: String,
    pub name_en: Option<String>,
    pub slug: Option<String>,
    pub category_id: Option<i64>,
    pub brand_id: Option<i64>,
    pub supplier_id: Option<i64>,
    pub weight: Option<f64>,
    pub volume: Option<f64>,
    pub description: Option<String>,
    pub description_en: Option<String>,
    pub specifications: JsonValue,
    pub main_image: Option<String>,
    pub images: JsonValue,
    pub status: i64,
    pub is_featured: bool,
    pub is_new: bool,
    pub view_count: i64,
    pub sales_count: i64,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

/// SKU 状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkuStatus {
    #[serde(rename = "1")]
    Active = 1,
    #[serde(rename = "2")]
    Inactive = 2,
}

impl Default for SkuStatus {
    fn default() -> Self {
        SkuStatus::Active
    }
}

/// SKU 实体
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProductSku {
    pub id: i64,
    pub product_id: i64,
    pub sku_code: String,
    pub spec_values: JsonValue,
    pub sale_price: f64,
    pub cost_price: f64,
    pub compare_price: Option<f64>,
    pub stock_quantity: i64,
    pub available_quantity: i64,
    pub locked_quantity: i64,
    pub safety_stock: i64,
    pub sku_image: Option<String>,
    pub barcode: Option<String>,
    pub qr_code: Option<String>,
    pub status: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 分类实体
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub name_en: Option<String>,
    pub slug: String,
    pub parent_id: Option<i64>,
    pub level: i64,
    pub path: Option<String>,
    pub icon: Option<String>,
    pub image: Option<String>,
    pub sort_order: i64,
    pub is_visible: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

/// 品牌实体
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Brand {
    pub id: i64,
    pub name: String,
    pub name_en: Option<String>,
    pub slug: String,
    pub logo: Option<String>,
    pub description: Option<String>,
    pub sort_order: i64,
    pub status: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ============================================================================
// 请求/响应 DTOs
// ============================================================================

/// 创建产品请求（不包含价格，价格独立管理）
#[derive(Debug, Deserialize, Validate)]
pub struct CreateProductRequest {
    /// 产品编码（可选，不提供时自动生成）
    #[validate(length(min = 1, max = 50))]
    pub product_code: Option<String>,
    #[validate(length(min = 1, max = 200))]
    pub name: String,
    pub name_en: Option<String>,
    pub slug: Option<String>,
    pub category_id: Option<i64>,
    pub brand_id: Option<i64>,
    pub supplier_id: Option<i64>,
    pub weight: Option<f64>,
    pub volume: Option<f64>,
    pub description: Option<String>,
    pub description_en: Option<String>,
    pub specifications: Option<JsonValue>,
    pub main_image: Option<String>,
    pub images: Option<JsonValue>,
    pub status: Option<i64>,
    pub is_featured: Option<bool>,
    pub is_new: Option<bool>,
    pub notes: Option<String>,
}

/// 更新产品请求（不包含价格，价格独立管理）
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateProductRequest {
    #[validate(length(min = 1, max = 200))]
    pub name: Option<String>,
    pub name_en: Option<String>,
    pub slug: Option<String>,
    pub category_id: Option<i64>,
    pub brand_id: Option<i64>,
    pub supplier_id: Option<i64>,
    pub weight: Option<f64>,
    pub volume: Option<f64>,
    pub description: Option<String>,
    pub description_en: Option<String>,
    pub specifications: Option<JsonValue>,
    pub main_image: Option<String>,
    pub images: Option<JsonValue>,
    pub status: Option<i64>,
    pub is_featured: Option<bool>,
    pub is_new: Option<bool>,
    pub notes: Option<String>,
}

/// 产品查询参数
#[derive(Debug, Deserialize)]
pub struct ProductQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub category_id: Option<i64>,
    pub brand_id: Option<i64>,
    pub status: Option<i64>,
    pub keyword: Option<String>,
    pub sort: Option<String>,
}

impl ProductQuery {
    pub fn page(&self) -> u32 {
        self.page.unwrap_or(1).max(1)
    }

    pub fn page_size(&self) -> u32 {
        self.page_size.unwrap_or(20).min(100).max(1)
    }
}

/// 产品详情（包含 SKU 列表）
#[derive(Debug, Serialize)]
pub struct ProductDetail {
    #[serde(flatten)]
    pub product: Product,
    pub skus: Vec<ProductSku>,
    pub category: Option<Category>,
    pub brand: Option<Brand>,
}

/// 产品列表项（精简版，价格从 product_prices 获取，成本从 product_costs 获取）
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ProductListItem {
    pub id: i64,
    pub product_code: String,
    pub name: String,
    pub main_image: Option<String>,
    pub cost_cny: Option<f64>,        // 从 product_costs 获取的成本（人民币）
    pub sale_price_cny: Option<f64>,  // 从 product_prices 获取的参考售价
    pub status: i64,
    pub stock_quantity: Option<i64>,
    pub category_name: Option<String>,
    pub brand_name: Option<String>,
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// 产品成本 (ProductCost)
// ============================================================================

/// 产品成本实体
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProductCost {
    pub id: i64,
    pub product_id: i64,
    pub supplier_id: Option<i64>,
    pub cost_cny: f64,
    pub cost_usd: Option<f64>,
    pub currency: String,
    pub exchange_rate: f64,
    pub profit_margin: f64,
    pub platform_fee_rate: f64,
    pub platform_fee: Option<f64>,
    pub sale_price_usd: Option<f64>,
    pub quantity: i64,  // 采购数量
    pub purchase_order_id: Option<i64>,  // 关联采购单
    pub is_reference: bool,  // 是否为参考价
    pub effective_date: Option<String>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 创建产品成本请求
#[derive(Debug, Deserialize, Validate)]
pub struct CreateProductCostRequest {
    pub product_id: i64,
    pub supplier_id: Option<i64>,
    #[validate(range(min = 0.0))]
    pub cost_cny: f64,
    pub cost_usd: Option<f64>,
    pub currency: Option<String>,
    pub exchange_rate: Option<f64>,
    pub profit_margin: Option<f64>,
    pub platform_fee_rate: Option<f64>,
    pub platform_fee: Option<f64>,
    pub sale_price_usd: Option<f64>,
    pub quantity: Option<i64>,  // 采购数量，默认 1
    pub purchase_order_id: Option<i64>,  // 关联采购单
    pub is_reference: Option<bool>,  // 是否为参考价，默认 false
    pub effective_date: Option<String>,
    pub notes: Option<String>,
}

/// 更新产品成本请求
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateProductCostRequest {
    pub supplier_id: Option<i64>,
    #[validate(range(min = 0.0))]
    pub cost_cny: Option<f64>,
    pub cost_usd: Option<f64>,
    pub currency: Option<String>,
    pub exchange_rate: Option<f64>,
    pub profit_margin: Option<f64>,
    pub platform_fee_rate: Option<f64>,
    pub platform_fee: Option<f64>,
    pub sale_price_usd: Option<f64>,
    pub quantity: Option<i64>,
    pub purchase_order_id: Option<i64>,
    pub is_reference: Option<bool>,
    pub effective_date: Option<String>,
    pub notes: Option<String>,
}

// ============================================================================
// 产品内容 (ProductContent)
// ============================================================================

/// 产品内容实体（用于独立站上架）
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProductContent {
    pub id: i64,
    pub product_id: i64,
    pub title_en: Option<String>,
    pub description: Option<String>,
    pub description_en: Option<String>,
    pub main_image: Option<String>,
    pub images: Option<String>,
    pub specifications: Option<String>,
    pub meta_title: Option<String>,
    pub meta_description: Option<String>,
    pub meta_keywords: Option<String>,
    pub content_html: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 创建产品内容请求
#[derive(Debug, Deserialize, Validate)]
pub struct CreateProductContentRequest {
    pub product_id: i64,
    pub title_en: Option<String>,
    pub description: Option<String>,
    pub description_en: Option<String>,
    pub main_image: Option<String>,
    pub images: Option<String>,
    pub specifications: Option<String>,
    pub meta_title: Option<String>,
    pub meta_description: Option<String>,
    pub meta_keywords: Option<String>,
    pub content_html: Option<String>,
}

/// 更新产品内容请求
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateProductContentRequest {
    pub title_en: Option<String>,
    pub description: Option<String>,
    pub description_en: Option<String>,
    pub main_image: Option<String>,
    pub images: Option<String>,
    pub specifications: Option<String>,
    pub meta_title: Option<String>,
    pub meta_description: Option<String>,
    pub meta_keywords: Option<String>,
    pub content_html: Option<String>,
}

// ============================================================================
// 产品销售价格 (ProductPrice)
// ============================================================================

/// 销售平台枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SalesPlatform {
    Website,   // 独立站
    Alibaba,   // 阿里巴巴
    Amazon,    // 亚马逊
    Manual,    // 手动报价
}

impl Default for SalesPlatform {
    fn default() -> Self {
        SalesPlatform::Website
    }
}

impl std::fmt::Display for SalesPlatform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SalesPlatform::Website => write!(f, "website"),
            SalesPlatform::Alibaba => write!(f, "alibaba"),
            SalesPlatform::Amazon => write!(f, "amazon"),
            SalesPlatform::Manual => write!(f, "manual"),
        }
    }
}

/// 产品销售价格实体
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProductPrice {
    pub id: i64,
    pub product_id: i64,
    pub platform: String,
    pub sale_price_cny: f64,
    pub sale_price_usd: Option<f64>,
    pub exchange_rate: f64,
    pub profit_margin: f64,
    pub platform_fee_rate: f64,
    pub platform_fee: Option<f64>,
    pub is_reference: bool,  // 是否为参考售价
    pub effective_date: Option<String>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 创建产品销售价格请求
#[derive(Debug, Deserialize, Validate)]
pub struct CreateProductPriceRequest {
    pub product_id: i64,
    pub platform: Option<String>,  // 默认 website
    #[validate(range(min = 0.0))]
    pub sale_price_cny: f64,
    pub sale_price_usd: Option<f64>,
    pub exchange_rate: Option<f64>,
    pub profit_margin: Option<f64>,
    pub platform_fee_rate: Option<f64>,
    pub platform_fee: Option<f64>,
    pub is_reference: Option<bool>,  // 默认 false
    pub effective_date: Option<String>,
    pub notes: Option<String>,
}

/// 更新产品销售价格请求
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateProductPriceRequest {
    pub platform: Option<String>,
    #[validate(range(min = 0.0))]
    pub sale_price_cny: Option<f64>,
    pub sale_price_usd: Option<f64>,
    pub exchange_rate: Option<f64>,
    pub profit_margin: Option<f64>,
    pub platform_fee_rate: Option<f64>,
    pub platform_fee: Option<f64>,
    pub is_reference: Option<bool>,
    pub effective_date: Option<String>,
    pub notes: Option<String>,
}

/// 价格查询参数
#[derive(Debug, Deserialize)]
pub struct PriceQuery {
    pub platform: Option<String>,
    pub is_reference: Option<bool>,
}

/// 产品价格统计（用于列表显示）
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ProductPriceSummary {
    pub product_id: i64,
    pub reference_cost_cny: Option<f64>,  // 参考成本
    pub avg_cost_cny: Option<f64>,  // 平均成本
    pub reference_price_cny: Option<f64>,  // 参考售价
    pub min_price_cny: Option<f64>,  // 最低售价
    pub max_price_cny: Option<f64>,  // 最高售价
}
