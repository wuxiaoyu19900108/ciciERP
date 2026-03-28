# ciciERP 数据模型设计

## 文档信息
- **项目名称**: ciciERP
- **版本**: v2.0
- **文档日期**: 2026-02-27
- **数据库**: SQLite

---

## 1. 数据模型概览

### 1.1 模块关系图

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           数据模型关系                                    │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│   ┌──────────────┐                    ┌──────────────┐                  │
│   │  categories  │                    │    brands    │                  │
│   │   (分类)     │                    │   (品牌)     │                  │
│   └──────┬───────┘                    └──────┬───────┘                  │
│          │                                   │                          │
│          └──────────────┬────────────────────┘                          │
│                         │                                               │
│                         ▼                                               │
│   ┌─────────────────────────────────┐         ┌──────────────┐         │
│   │           products              │◀────────│  suppliers   │         │
│   │          (产品主表)              │         │  (供应商)    │         │
│   └───────────────┬─────────────────┘         └──────────────┘         │
│                   │                               ▲                     │
│                   │                               │                     │
│                   ▼                               │                     │
│   ┌─────────────────────────────────┐    ┌──────┴───────┐              │
│   │          product_skus           │    │product_      │              │
│   │           (SKU)                 │────│suppliers     │              │
│   └───────────────┬─────────────────┘    │(多对多)      │              │
│                   │                      └──────────────┘              │
│         ┌─────────┴─────────┐                                          │
│         │                   │                                          │
│         ▼                   ▼                                          │
│   ┌───────────┐      ┌───────────┐                                     │
│   │ inventory │      │   price   │                                     │
│   │  (库存)   │      │  (价格)   │                                     │
│   └───────────┘      └───────────┘                                     │
│                                                                          │
│   ┌──────────────┐         ┌──────────────┐                            │
│   │  customers   │────────▶│   orders     │                            │
│   │   (客户)     │  1:N    │   (订单)     │                            │
│   └──────────────┘         └──────┬───────┘                            │
│                                   │                                     │
│                         ┌─────────┴─────────┐                          │
│                         │                   │                          │
│                         ▼                   ▼                          │
│                  ┌──────────────┐    ┌──────────────┐                  │
│                  │ order_items  │    │ order_addrs  │                  │
│                  │ (订单明细)    │    │ (收货地址)    │                  │
│                  └──────┬───────┘    └──────────────┘                  │
│                         │                                               │
│                         ▼                                               │
│                  ┌──────────────┐                                       │
│                  │  shipments   │                                       │
│                  │   (发货单)    │                                       │
│                  └──────────────┘                                       │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 2. 产品管理模块

### 2.1 产品主表 (products)

| 字段名 | 数据类型 | 必填 | 默认值 | 说明 |
|-------|---------|-----|-------|------|
| id | INTEGER | 是 | AUTO | 主键，自增 |
| product_code | TEXT | 是 | - | 产品编码，唯一 |
| name | TEXT | 是 | - | 产品名称 |
| name_en | TEXT | 否 | NULL | 英文名称 |
| slug | TEXT | 否 | NULL | URL 友好名称，唯一 |
| category_id | INTEGER | 否 | NULL | 分类ID，外键 |
| brand_id | INTEGER | 否 | NULL | 品牌ID，外键 |
| purchase_price | REAL | 是 | - | 采购价（参考） |
| sale_price | REAL | 是 | - | 销售价（参考） |
| compare_price | REAL | 否 | NULL | 划线价 |
| weight | REAL | 否 | NULL | 重量(kg) |
| volume | REAL | 否 | NULL | 体积(m³) |
| description | TEXT | 否 | NULL | 详细描述 |
| description_en | TEXT | 否 | NULL | 英文描述 |
| specifications | TEXT | 否 | '{}' | 规格参数(JSON) |
| main_image | TEXT | 否 | NULL | 主图URL |
| images | TEXT | 否 | '[]' | 图片列表(JSON数组) |
| status | INTEGER | 是 | 1 | 状态：1上架 2下架 3草稿 |
| is_featured | INTEGER | 否 | 0 | 是否推荐：0否 1是 |
| is_new | INTEGER | 否 | 0 | 是否新品：0否 1是 |
| view_count | INTEGER | 否 | 0 | 浏览次数 |
| sales_count | INTEGER | 否 | 0 | 销售数量 |
| created_at | TEXT | 是 | datetime('now') | 创建时间 |
| updated_at | TEXT | 是 | datetime('now') | 更新时间 |
| deleted_at | TEXT | 否 | NULL | 软删除时间 |

**索引**：
- `idx_products_code` ON (product_code)
- `idx_products_category` ON (category_id)
- `idx_products_brand` ON (brand_id)
- `idx_products_status` ON (status)

**Rust 结构体**：
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: i64,
    pub product_code: String,
    pub name: String,
    pub name_en: Option<String>,
    pub slug: Option<String>,
    pub category_id: Option<i64>,
    pub brand_id: Option<i64>,
    pub purchase_price: f64,
    pub sale_price: f64,
    pub compare_price: Option<f64>,
    pub weight: Option<f64>,
    pub volume: Option<f64>,
    pub description: Option<String>,
    pub description_en: Option<String>,
    pub specifications: serde_json::Value,
    pub main_image: Option<String>,
    pub images: serde_json::Value,
    pub status: ProductStatus,
    pub is_featured: bool,
    pub is_new: bool,
    pub view_count: i64,
    pub sales_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(rename_all = "lowercase")]
pub enum ProductStatus {
    #[serde(rename = "1")]
    Active = 1,     // 上架
    #[serde(rename = "2")]
    Inactive = 2,   // 下架
    #[serde(rename = "3")]
    Draft = 3,      // 草稿
}
```

### 2.2 SKU 表 (product_skus)

| 字段名 | 数据类型 | 必填 | 默认值 | 说明 |
|-------|---------|-----|-------|------|
| id | INTEGER | 是 | AUTO | 主键，自增 |
| product_id | INTEGER | 是 | - | 产品ID，外键 |
| sku_code | TEXT | 是 | - | SKU编码，唯一 |
| spec_values | TEXT | 是 | '{}' | 规格值(JSON对象) |
| sale_price | REAL | 是 | - | 销售价 |
| cost_price | REAL | 是 | - | 成本价 |
| compare_price | REAL | 否 | NULL | 划线价 |
| stock_quantity | INTEGER | 是 | 0 | 库存总数 |
| available_quantity | INTEGER | 是 | 0 | 可用库存 |
| locked_quantity | INTEGER | 是 | 0 | 锁定库存 |
| safety_stock | INTEGER | 否 | 10 | 安全库存 |
| sku_image | TEXT | 否 | NULL | SKU图片 |
| barcode | TEXT | 否 | NULL | 条形码 |
| qr_code | TEXT | 否 | NULL | 二维码 |
| status | INTEGER | 是 | 1 | 状态：1正常 2停用 |
| created_at | TEXT | 是 | datetime('now') | 创建时间 |
| updated_at | TEXT | 是 | datetime('now') | 更新时间 |

**索引**：
- `idx_skus_product` ON (product_id)
- `idx_skus_code` ON (sku_code)
- `idx_skus_barcode` ON (barcode)

**Rust 结构体**：
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductSku {
    pub id: i64,
    pub product_id: i64,
    pub sku_code: String,
    pub spec_values: serde_json::Value,  // {"颜色": "红色", "尺寸": "L"}
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
    pub status: SkuStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

### 2.3 分类表 (categories)

| 字段名 | 数据类型 | 必填 | 默认值 | 说明 |
|-------|---------|-----|-------|------|
| id | INTEGER | 是 | AUTO | 主键 |
| name | TEXT | 是 | - | 分类名称 |
| name_en | TEXT | 否 | NULL | 英文名称 |
| slug | TEXT | 是 | - | URL友好名称 |
| parent_id | INTEGER | 否 | NULL | 父分类ID |
| level | INTEGER | 是 | 0 | 层级(0,1,2...) |
| path | TEXT | 否 | NULL | 路径(如:1,5,12) |
| icon | TEXT | 否 | NULL | 图标 |
| image | TEXT | 否 | NULL | 分类图片 |
| sort_order | INTEGER | 否 | 0 | 排序 |
| is_visible | INTEGER | 否 | 1 | 是否显示 |
| created_at | TEXT | 是 | datetime('now') | 创建时间 |
| updated_at | TEXT | 是 | datetime('now') | 更新时间 |
| deleted_at | TEXT | 否 | NULL | 软删除时间 |

### 2.4 品牌表 (brands)

| 字段名 | 数据类型 | 必填 | 默认值 | 说明 |
|-------|---------|-----|-------|------|
| id | INTEGER | 是 | AUTO | 主键 |
| name | TEXT | 是 | - | 品牌名称 |
| name_en | TEXT | 否 | NULL | 英文名称 |
| slug | TEXT | 是 | - | URL友好名称 |
| logo | TEXT | 否 | NULL | 品牌Logo |
| description | TEXT | 否 | NULL | 品牌描述 |
| sort_order | INTEGER | 否 | 0 | 排序 |
| status | INTEGER | 是 | 1 | 状态：1启用 2停用 |
| created_at | TEXT | 是 | datetime('now') | 创建时间 |
| updated_at | TEXT | 是 | datetime('now') | 更新时间 |

---

## 3. 供应商管理模块

### 3.1 供应商表 (suppliers)

| 字段名 | 数据类型 | 必填 | 默认值 | 说明 |
|-------|---------|-----|-------|------|
| id | INTEGER | 是 | AUTO | 主键 |
| supplier_code | TEXT | 是 | - | 供应商编码，唯一 |
| name | TEXT | 是 | - | 供应商名称 |
| name_en | TEXT | 否 | NULL | 英文名称 |
| contact_person | TEXT | 否 | NULL | 联系人 |
| contact_phone | TEXT | 否 | NULL | 联系电话 |
| contact_email | TEXT | 否 | NULL | 联系邮箱 |
| address | TEXT | 否 | NULL | 地址 |
| credit_code | TEXT | 否 | NULL | 统一社会信用代码 |
| tax_id | TEXT | 否 | NULL | 税号 |
| bank_name | TEXT | 否 | NULL | 开户银行 |
| bank_account | TEXT | 否 | NULL | 银行账号 |
| rating_level | TEXT | 否 | 'C' | 评级(A/B/C/D) |
| rating_score | REAL | 否 | 3.5 | 评分(0-5) |
| payment_terms | INTEGER | 否 | 30 | 账期(天) |
| payment_method | TEXT | 否 | NULL | 付款方式 |
| total_orders | INTEGER | 否 | 0 | 采购订单数 |
| total_amount | REAL | 否 | 0 | 采购总金额 |
| status | INTEGER | 是 | 1 | 状态：1合作 2暂停 3终止 |
| notes | TEXT | 否 | NULL | 备注 |
| created_at | TEXT | 是 | datetime('now') | 创建时间 |
| updated_at | TEXT | 是 | datetime('now') | 更新时间 |
| deleted_at | TEXT | 否 | NULL | 软删除时间 |

**索引**：
- `idx_suppliers_code` ON (supplier_code)
- `idx_suppliers_rating` ON (rating_level)
- `idx_suppliers_status` ON (status)

**Rust 结构体**：
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Supplier {
    pub id: i64,
    pub supplier_code: String,
    pub name: String,
    pub name_en: Option<String>,
    pub contact_person: Option<String>,
    pub contact_phone: Option<String>,
    pub contact_email: Option<String>,
    pub address: Option<String>,
    pub credit_code: Option<String>,
    pub tax_id: Option<String>,
    pub bank_name: Option<String>,
    pub bank_account: Option<String>,
    pub rating_level: String,
    pub rating_score: f64,
    pub payment_terms: i32,
    pub payment_method: Option<String>,
    pub total_orders: i64,
    pub total_amount: f64,
    pub status: SupplierStatus,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}
```

### 3.2 产品-供应商关联表 (product_suppliers)

| 字段名 | 数据类型 | 必填 | 默认值 | 说明 |
|-------|---------|-----|-------|------|
| id | INTEGER | 是 | AUTO | 主键 |
| product_id | INTEGER | 是 | - | 产品ID，外键 |
| supplier_id | INTEGER | 是 | - | 供应商ID，外键 |
| supplier_sku | TEXT | 否 | NULL | 供应商SKU编码 |
| purchase_price | REAL | 否 | NULL | 采购价 |
| min_order_qty | INTEGER | 否 | 1 | 最小起订量 |
| lead_time | INTEGER | 否 | NULL | 交货周期(天) |
| is_primary | INTEGER | 否 | 0 | 是否首选供应商 |
| created_at | TEXT | 是 | datetime('now') | 创建时间 |

**约束**：
- UNIQUE(product_id, supplier_id)

**索引**：
- `idx_product_suppliers_product` ON (product_id)
- `idx_product_suppliers_supplier` ON (supplier_id)

**Rust 结构体**：
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductSupplier {
    pub id: i64,
    pub product_id: i64,
    pub supplier_id: i64,
    pub supplier_sku: Option<String>,
    pub purchase_price: Option<f64>,
    pub min_order_qty: i32,
    pub lead_time: Option<i32>,
    pub is_primary: bool,
    pub created_at: DateTime<Utc>,
}
```

---

## 4. 客户管理模块

### 4.1 客户表 (customers)

| 字段名 | 数据类型 | 必填 | 默认值 | 说明 |
|-------|---------|-----|-------|------|
| id | INTEGER | 是 | AUTO | 主键 |
| customer_code | TEXT | 是 | - | 客户编码，唯一 |
| name | TEXT | 是 | - | 客户名称 |
| mobile | TEXT | 否 | NULL | 手机号，唯一 |
| email | TEXT | 否 | NULL | 邮箱 |
| gender | INTEGER | 否 | NULL | 性别：1男 2女 |
| birthday | TEXT | 否 | NULL | 生日(YYYY-MM-DD) |
| avatar | TEXT | 否 | NULL | 头像URL |
| level_id | INTEGER | 否 | NULL | 客户等级ID |
| points | INTEGER | 否 | 0 | 积分 |
| total_orders | INTEGER | 否 | 0 | 订单总数 |
| total_amount | REAL | 否 | 0 | 消费总额 |
| avg_order_amount | REAL | 否 | NULL | 平均订单金额 |
| tags | TEXT | 否 | '[]' | 标签(JSON数组) |
| attributes | TEXT | 否 | '{}' | 扩展属性(JSON) |
| source | TEXT | 是 | - | 来源(cicishop/amazon/线下) |
| external_id | TEXT | 否 | NULL | 外部系统ID |
| external_platform | TEXT | 否 | NULL | 外部平台名称 |
| status | INTEGER | 是 | 1 | 状态：1正常 2冻结 3黑名单 |
| last_login_at | TEXT | 否 | NULL | 最后登录时间 |
| last_order_at | TEXT | 否 | NULL | 最后下单时间 |
| created_at | TEXT | 是 | datetime('now') | 创建时间 |
| updated_at | TEXT | 是 | datetime('now') | 更新时间 |
| deleted_at | TEXT | 否 | NULL | 软删除时间 |

**索引**：
- `idx_customers_code` ON (customer_code)
- `idx_customers_mobile` ON (mobile)
- `idx_customers_email` ON (email)
- `idx_customers_external` ON (external_platform, external_id)
- `idx_customers_level` ON (level_id)

**Rust 结构体**：
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Customer {
    pub id: i64,
    pub customer_code: String,
    pub name: String,
    pub mobile: Option<String>,
    pub email: Option<String>,
    pub gender: Option<Gender>,
    pub birthday: Option<String>,
    pub avatar: Option<String>,
    pub level_id: Option<i64>,
    pub points: i64,
    pub total_orders: i64,
    pub total_amount: f64,
    pub avg_order_amount: Option<f64>,
    pub tags: serde_json::Value,
    pub attributes: serde_json::Value,
    pub source: String,
    pub external_id: Option<String>,
    pub external_platform: Option<String>,
    pub status: CustomerStatus,
    pub last_login_at: Option<DateTime<Utc>>,
    pub last_order_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
pub enum Gender {
    #[serde(rename = "1")]
    Male = 1,
    #[serde(rename = "2")]
    Female = 2,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
pub enum CustomerStatus {
    #[serde(rename = "1")]
    Active = 1,
    #[serde(rename = "2")]
    Frozen = 2,
    #[serde(rename = "3")]
    Blacklist = 3,
}
```

### 4.2 客户地址表 (customer_addresses)

| 字段名 | 数据类型 | 必填 | 默认值 | 说明 |
|-------|---------|-----|-------|------|
| id | INTEGER | 是 | AUTO | 主键 |
| customer_id | INTEGER | 是 | - | 客户ID，外键 |
| receiver_name | TEXT | 是 | - | 收货人姓名 |
| receiver_phone | TEXT | 是 | - | 收货人电话 |
| country | TEXT | 是 | - | 国家 |
| country_code | TEXT | 否 | NULL | 国家代码 |
| province | TEXT | 否 | NULL | 省/州 |
| city | TEXT | 否 | NULL | 城市 |
| district | TEXT | 否 | NULL | 区/县 |
| address | TEXT | 是 | - | 详细地址 |
| postal_code | TEXT | 否 | NULL | 邮编 |
| address_type | INTEGER | 否 | 1 | 类型：1家庭 2公司 |
| is_default | INTEGER | 否 | 0 | 是否默认 |
| created_at | TEXT | 是 | datetime('now') | 创建时间 |
| updated_at | TEXT | 是 | datetime('now') | 更新时间 |

### 4.3 客户等级表 (customer_levels)

| 字段名 | 数据类型 | 必填 | 默认值 | 说明 |
|-------|---------|-----|-------|------|
| id | INTEGER | 是 | AUTO | 主键 |
| name | TEXT | 是 | - | 等级名称 |
| name_en | TEXT | 否 | NULL | 英文名称 |
| level | INTEGER | 是 | - | 等级数值，唯一 |
| min_amount | REAL | 否 | 0 | 升级消费金额 |
| min_orders | INTEGER | 否 | 0 | 升级订单数 |
| min_points | INTEGER | 否 | 0 | 升级积分 |
| discount_percent | REAL | 否 | 0 | 折扣百分比 |
| free_shipping | INTEGER | 否 | 0 | 是否免运费 |
| special_services | TEXT | 否 | NULL | 特殊服务说明 |
| sort_order | INTEGER | 否 | 0 | 排序 |
| created_at | TEXT | 是 | datetime('now') | 创建时间 |
| updated_at | TEXT | 是 | datetime('now') | 更新时间 |

---

## 5. 订单管理模块

### 5.1 订单主表 (orders)

| 字段名 | 数据类型 | 必填 | 默认值 | 说明 |
|-------|---------|-----|-------|------|
| id | INTEGER | 是 | AUTO | 主键 |
| order_code | TEXT | 是 | - | 订单号，唯一 |
| platform | TEXT | 是 | - | 来源平台 |
| platform_order_id | TEXT | 否 | NULL | 平台订单ID |
| customer_id | INTEGER | 否 | NULL | 客户ID，外键 |
| customer_name | TEXT | 否 | NULL | 客户姓名(快照) |
| customer_mobile | TEXT | 否 | NULL | 客户手机(快照) |
| customer_email | TEXT | 否 | NULL | 客户邮箱(快照) |
| order_type | INTEGER | 是 | 1 | 类型：1普通 2预售 3换货 |
| order_status | INTEGER | 是 | 1 | 订单状态 |
| payment_status | INTEGER | 是 | 1 | 支付状态 |
| fulfillment_status | INTEGER | 是 | 1 | 履约状态 |
| total_amount | REAL | 是 | - | 订单总额 |
| subtotal | REAL | 是 | - | 商品小计 |
| discount_amount | REAL | 否 | 0 | 优惠金额 |
| shipping_fee | REAL | 否 | 0 | 运费 |
| tax_amount | REAL | 否 | 0 | 税费 |
| paid_amount | REAL | 否 | 0 | 已付金额 |
| refund_amount | REAL | 否 | 0 | 退款金额 |
| currency | TEXT | 否 | 'CNY' | 币种 |
| exchange_rate | REAL | 否 | NULL | 汇率 |
| coupon_id | INTEGER | 否 | NULL | 优惠券ID |
| coupon_amount | REAL | 否 | 0 | 优惠券金额 |
| points_used | INTEGER | 否 | 0 | 使用积分 |
| points_discount | REAL | 否 | 0 | 积分抵扣金额 |
| customer_note | TEXT | 否 | NULL | 客户备注 |
| internal_note | TEXT | 否 | NULL | 内部备注 |
| is_rated | INTEGER | 否 | 0 | 是否已评价 |
| payment_time | TEXT | 否 | NULL | 支付时间 |
| ship_time | TEXT | 否 | NULL | 发货时间 |
| finish_time | TEXT | 否 | NULL | 完成时间 |
| cancel_time | TEXT | 否 | NULL | 取消时间 |
| cancel_reason | TEXT | 否 | NULL | 取消原因 |
| created_at | TEXT | 是 | datetime('now') | 创建时间 |
| updated_at | TEXT | 是 | datetime('now') | 更新时间 |

**订单状态说明**：
| 值 | 状态 | 说明 |
|---|------|------|
| 1 | 待审核 | 新订单，待处理 |
| 2 | 待发货 | 已审核，待发货 |
| 3 | 部分发货 | 部分商品已发货 |
| 4 | 已发货 | 全部发货 |
| 5 | 已完成 | 订单完成 |
| 6 | 已取消 | 订单取消 |
| 7 | 售后中 | 有售后申请 |

**支付状态说明**：
| 值 | 状态 | 说明 |
|---|------|------|
| 1 | 未支付 | 等待付款 |
| 2 | 部分支付 | 部分付款 |
| 3 | 已支付 | 全部付款 |
| 4 | 已退款 | 全部退款 |
| 5 | 部分退款 | 部分退款 |

**履约状态说明**：
| 值 | 状态 | 说明 |
|---|------|------|
| 1 | 未发货 | 等待发货 |
| 2 | 部分发货 | 部分发货 |
| 3 | 已发货 | 全部发货 |
| 4 | 已签收 | 已签收 |

**索引**：
- `idx_orders_code` ON (order_code)
- `idx_orders_platform` ON (platform, platform_order_id)
- `idx_orders_customer` ON (customer_id)
- `idx_orders_status` ON (order_status)
- `idx_orders_payment` ON (payment_status)
- `idx_orders_created` ON (created_at)

**Rust 结构体**：
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: i64,
    pub order_code: String,
    pub platform: String,
    pub platform_order_id: Option<String>,
    pub customer_id: Option<i64>,
    pub customer_name: Option<String>,
    pub customer_mobile: Option<String>,
    pub customer_email: Option<String>,
    pub order_type: OrderType,
    pub order_status: OrderStatus,
    pub payment_status: PaymentStatus,
    pub fulfillment_status: FulfillmentStatus,
    pub total_amount: f64,
    pub subtotal: f64,
    pub discount_amount: f64,
    pub shipping_fee: f64,
    pub tax_amount: f64,
    pub paid_amount: f64,
    pub refund_amount: f64,
    pub currency: String,
    pub exchange_rate: Option<f64>,
    pub coupon_id: Option<i64>,
    pub coupon_amount: f64,
    pub points_used: i64,
    pub points_discount: f64,
    pub customer_note: Option<String>,
    pub internal_note: Option<String>,
    pub is_rated: bool,
    pub payment_time: Option<DateTime<Utc>>,
    pub ship_time: Option<DateTime<Utc>>,
    pub finish_time: Option<DateTime<Utc>>,
    pub cancel_time: Option<DateTime<Utc>>,
    pub cancel_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

### 5.2 订单明细表 (order_items)

| 字段名 | 数据类型 | 必填 | 默认值 | 说明 |
|-------|---------|-----|-------|------|
| id | INTEGER | 是 | AUTO | 主键 |
| order_id | INTEGER | 是 | - | 订单ID，外键 |
| product_id | INTEGER | 否 | NULL | 产品ID |
| sku_id | INTEGER | 否 | NULL | SKU ID |
| product_name | TEXT | 是 | - | 产品名称(快照) |
| product_code | TEXT | 否 | NULL | 产品编码(快照) |
| sku_code | TEXT | 否 | NULL | SKU编码(快照) |
| sku_spec | TEXT | 否 | NULL | 规格值(JSON快照) |
| product_image | TEXT | 否 | NULL | 产品图片(快照) |
| quantity | INTEGER | 是 | - | 数量 |
| unit_price | REAL | 是 | - | 单价 |
| subtotal | REAL | 是 | - | 小计 |
| discount_amount | REAL | 否 | 0 | 优惠金额 |
| total_amount | REAL | 是 | - | 总价 |
| cost_price | REAL | 否 | NULL | 成本价 |
| tax_rate | REAL | 否 | NULL | 税率 |
| tax_amount | REAL | 否 | NULL | 税额 |
| refund_quantity | INTEGER | 否 | 0 | 退款数量 |
| refund_amount | REAL | 否 | 0 | 退款金额 |
| created_at | TEXT | 是 | datetime('now') | 创建时间 |

**索引**：
- `idx_order_items_order` ON (order_id)
- `idx_order_items_product` ON (product_id)
- `idx_order_items_sku` ON (sku_id)

### 5.3 订单收货地址表 (order_addresses)

| 字段名 | 数据类型 | 必填 | 默认值 | 说明 |
|-------|---------|-----|-------|------|
| id | INTEGER | 是 | AUTO | 主键 |
| order_id | INTEGER | 是 | - | 订单ID，外键 |
| receiver_name | TEXT | 是 | - | 收货人姓名 |
| receiver_phone | TEXT | 是 | - | 收货人电话 |
| country | TEXT | 是 | - | 国家 |
| country_code | TEXT | 否 | NULL | 国家代码 |
| province | TEXT | 否 | NULL | 省/州 |
| city | TEXT | 否 | NULL | 城市 |
| district | TEXT | 否 | NULL | 区/县 |
| address | TEXT | 是 | - | 详细地址 |
| postal_code | TEXT | 否 | NULL | 邮编 |
| address_type | INTEGER | 否 | 1 | 类型：1家庭 2公司 |
| created_at | TEXT | 是 | datetime('now') | 创建时间 |

---

## 6. 库存管理模块

### 6.1 库存主表 (inventory)

| 字段名 | 数据类型 | 必填 | 默认值 | 说明 |
|-------|---------|-----|-------|------|
| id | INTEGER | 是 | AUTO | 主键 |
| sku_id | INTEGER | 是 | - | SKU ID，外键，唯一 |
| total_quantity | INTEGER | 是 | 0 | 库存总数 |
| available_quantity | INTEGER | 是 | 0 | 可用库存 |
| locked_quantity | INTEGER | 是 | 0 | 锁定库存 |
| damaged_quantity | INTEGER | 否 | 0 | 损坏库存 |
| safety_stock | INTEGER | 否 | 10 | 安全库存(预警线) |
| max_stock | INTEGER | 否 | NULL | 最大库存 |
| warehouse_id | INTEGER | 否 | NULL | 仓库ID |
| location | TEXT | 否 | NULL | 库位 |
| created_at | TEXT | 是 | datetime('now') | 创建时间 |
| updated_at | TEXT | 是 | datetime('now') | 更新时间 |

**索引**：
- `idx_inventory_sku` ON (sku_id) UNIQUE
- `idx_inventory_warehouse` ON (warehouse_id)

**Rust 结构体**：
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inventory {
    pub id: i64,
    pub sku_id: i64,
    pub total_quantity: i64,
    pub available_quantity: i64,
    pub locked_quantity: i64,
    pub damaged_quantity: i64,
    pub safety_stock: i64,
    pub max_stock: Option<i64>,
    pub warehouse_id: Option<i64>,
    pub location: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Inventory {
    /// 检查是否低于安全库存
    pub fn is_low_stock(&self) -> bool {
        self.available_quantity < self.safety_stock
    }

    /// 检查是否可以锁定指定数量
    pub fn can_lock(&self, quantity: i64) -> bool {
        self.available_quantity >= quantity
    }
}
```

### 6.2 库存流水表 (stock_movements)

| 字段名 | 数据类型 | 必填 | 默认值 | 说明 |
|-------|---------|-----|-------|------|
| id | INTEGER | 是 | AUTO | 主键 |
| movement_code | TEXT | 是 | - | 流水号，唯一 |
| sku_id | INTEGER | 是 | - | SKU ID，外键 |
| warehouse_id | INTEGER | 否 | NULL | 仓库ID |
| movement_type | INTEGER | 是 | - | 变动类型 |
| quantity | INTEGER | 是 | - | 变动数量(正负) |
| before_quantity | INTEGER | 是 | - | 变动前数量 |
| after_quantity | INTEGER | 是 | - | 变动后数量 |
| reference_type | TEXT | 否 | NULL | 关联类型 |
| reference_id | INTEGER | 否 | NULL | 关联ID |
| reference_code | TEXT | 否 | NULL | 关联单号 |
| note | TEXT | 否 | NULL | 备注 |
| operator_id | INTEGER | 否 | NULL | 操作人ID |
| operator_name | TEXT | 否 | NULL | 操作人姓名 |
| created_at | TEXT | 是 | datetime('now') | 创建时间 |

**变动类型说明**：
| 值 | 类型 | 说明 |
|---|------|------|
| 1 | 入库 | 采购入库、退货入库 |
| 2 | 出库 | 销售出库 |
| 3 | 调拨 | 仓库调拨 |
| 4 | 盘点 | 库存盘点调整 |
| 5 | 损耗 | 损坏、丢失 |
| 6 | 锁定 | 订单锁定 |
| 7 | 解锁 | 订单取消解锁 |

**索引**：
- `idx_stock_movements_sku` ON (sku_id)
- `idx_stock_movements_type` ON (movement_type)
- `idx_stock_movements_reference` ON (reference_type, reference_id)
- `idx_stock_movements_created` ON (created_at)

**Rust 结构体**：
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockMovement {
    pub id: i64,
    pub movement_code: String,
    pub sku_id: i64,
    pub warehouse_id: Option<i64>,
    pub movement_type: MovementType,
    pub quantity: i64,
    pub before_quantity: i64,
    pub after_quantity: i64,
    pub reference_type: Option<String>,
    pub reference_id: Option<i64>,
    pub reference_code: Option<String>,
    pub note: Option<String>,
    pub operator_id: Option<i64>,
    pub operator_name: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
pub enum MovementType {
    #[serde(rename = "1")]
    Inbound = 1,      // 入库
    #[serde(rename = "2")]
    Outbound = 2,     // 出库
    #[serde(rename = "3")]
    Transfer = 3,     // 调拨
    #[serde(rename = "4")]
    Adjustment = 4,   // 盘点
    #[serde(rename = "5")]
    Damage = 5,       // 损耗
    #[serde(rename = "6")]
    Lock = 6,         // 锁定
    #[serde(rename = "7")]
    Unlock = 7,       // 解锁
}
```

---

## 7. 采购管理模块

### 7.1 采购单表 (purchase_orders)

| 字段名 | 数据类型 | 必填 | 默认值 | 说明 |
|-------|---------|-----|-------|------|
| id | INTEGER | 是 | AUTO | 主键 |
| order_code | TEXT | 是 | - | 采购单号，唯一 |
| supplier_id | INTEGER | 是 | - | 供应商ID，外键 |
| supplier_name | TEXT | 否 | NULL | 供应商名称(快照) |
| total_amount | REAL | 是 | - | 采购总额 |
| tax_amount | REAL | 否 | 0 | 税额 |
| paid_amount | REAL | 否 | 0 | 已付金额 |
| payment_status | INTEGER | 是 | 1 | 付款状态 |
| delivery_status | INTEGER | 是 | 1 | 交货状态 |
| expected_date | TEXT | 否 | NULL | 预计到货日期 |
| actual_date | TEXT | 否 | NULL | 实际到货日期 |
| status | INTEGER | 是 | 1 | 单据状态 |
| approved_by | INTEGER | 否 | NULL | 审批人ID |
| approved_at | TEXT | 否 | NULL | 审批时间 |
| approval_note | TEXT | 否 | NULL | 审批备注 |
| supplier_note | TEXT | 否 | NULL | 供应商备注 |
| internal_note | TEXT | 否 | NULL | 内部备注 |
| attachments | TEXT | 否 | '[]' | 附件(JSON数组) |
| created_at | TEXT | 是 | datetime('now') | 创建时间 |
| updated_at | TEXT | 是 | datetime('now') | 更新时间 |

**采购单状态**：
| 值 | 状态 | 说明 |
|---|------|------|
| 1 | 待审核 | 新建，待审批 |
| 2 | 已审核 | 审批通过 |
| 3 | 执行中 | 部分入库 |
| 4 | 已完成 | 全部入库 |
| 5 | 已取消 | 取消 |

**索引**：
- `idx_purchase_orders_code` ON (order_code)
- `idx_purchase_orders_supplier` ON (supplier_id)
- `idx_purchase_orders_status` ON (status)

### 7.2 采购明细表 (purchase_order_items)

| 字段名 | 数据类型 | 必填 | 默认值 | 说明 |
|-------|---------|-----|-------|------|
| id | INTEGER | 是 | AUTO | 主键 |
| order_id | INTEGER | 是 | - | 采购单ID，外键 |
| product_id | INTEGER | 否 | NULL | 产品ID |
| sku_id | INTEGER | 否 | NULL | SKU ID |
| product_name | TEXT | 是 | - | 产品名称 |
| sku_code | TEXT | 否 | NULL | SKU编码 |
| spec_values | TEXT | 否 | NULL | 规格值(JSON) |
| quantity | INTEGER | 是 | - | 采购数量 |
| received_qty | INTEGER | 否 | 0 | 已收货数量 |
| unit_price | REAL | 是 | - | 单价 |
| subtotal | REAL | 是 | - | 小计 |
| expected_qty | INTEGER | 否 | NULL | 预计数量 |
| expected_date | TEXT | 否 | NULL | 预计到货日期 |
| inspected_qty | INTEGER | 否 | 0 | 已质检数量 |
| qualified_qty | INTEGER | 否 | 0 | 合格数量 |
| defective_qty | INTEGER | 否 | 0 | 不合格数量 |
| batch_code | TEXT | 否 | NULL | 批次号 |
| production_date | TEXT | 否 | NULL | 生产日期 |
| expiry_date | TEXT | 否 | NULL | 有效期 |
| created_at | TEXT | 是 | datetime('now') | 创建时间 |

---

## 8. 物流管理模块

### 8.1 物流公司表 (logistics_companies)

| 字段名 | 数据类型 | 必填 | 默认值 | 说明 |
|-------|---------|-----|-------|------|
| id | INTEGER | 是 | AUTO | 主键 |
| code | TEXT | 是 | - | 物流公司代码，唯一 |
| name | TEXT | 是 | - | 公司名称 |
| name_en | TEXT | 否 | NULL | 英文名称 |
| service_type | TEXT | 是 | - | 服务类型(express/air/sea/land) |
| api_code | TEXT | 否 | NULL | API对接代码 |
| api_config | TEXT | 否 | '{}' | API配置(JSON) |
| contact_phone | TEXT | 否 | NULL | 客服电话 |
| contact_email | TEXT | 否 | NULL | 客服邮箱 |
| website | TEXT | 否 | NULL | 官网 |
| tracking_url_template | TEXT | 否 | NULL | 查询URL模板 |
| status | INTEGER | 是 | 1 | 状态：1启用 2停用 |
| created_at | TEXT | 是 | datetime('now') | 创建时间 |
| updated_at | TEXT | 是 | datetime('now') | 更新时间 |

### 8.2 发货单表 (shipments)

| 字段名 | 数据类型 | 必填 | 默认值 | 说明 |
|-------|---------|-----|-------|------|
| id | INTEGER | 是 | AUTO | 主键 |
| shipment_code | TEXT | 是 | - | 发货单号，唯一 |
| order_id | INTEGER | 是 | - | 订单ID，外键 |
| logistics_id | INTEGER | 否 | NULL | 物流公司ID |
| logistics_name | TEXT | 否 | NULL | 物流公司名称 |
| tracking_number | TEXT | 否 | NULL | 物流单号 |
| receiver_name | TEXT | 是 | - | 收货人(快照) |
| receiver_phone | TEXT | 是 | - | 收货电话(快照) |
| receiver_address | TEXT | 是 | - | 收货地址(快照) |
| package_weight | REAL | 否 | NULL | 包裹重量(kg) |
| package_volume | REAL | 否 | NULL | 包裹体积(m³) |
| package_items | TEXT | 是 | '[]' | 包裹商品(JSON) |
| package_count | INTEGER | 否 | 1 | 包裹数量 |
| shipping_fee | REAL | 否 | 0 | 运费 |
| actual_shipping_fee | REAL | 否 | NULL | 实际运费 |
| estimated_arrival | TEXT | 否 | NULL | 预计到达日期 |
| actual_arrival | TEXT | 否 | NULL | 实际到达时间 |
| status | INTEGER | 是 | 1 | 状态 |
| shipping_note | TEXT | 否 | NULL | 发货备注 |
| ship_time | TEXT | 否 | NULL | 发货时间 |
| created_at | TEXT | 是 | datetime('now') | 创建时间 |
| updated_at | TEXT | 是 | datetime('now') | 更新时间 |

**发货单状态**：
| 值 | 状态 | 说明 |
|---|------|------|
| 1 | 已发货 | 已揽收 |
| 2 | 运输中 | 配送中 |
| 3 | 已签收 | 已送达 |
| 4 | 异常 | 配送异常 |
| 5 | 已退货 | 退回 |

**索引**：
- `idx_shipments_code` ON (shipment_code)
- `idx_shipments_order` ON (order_id)
- `idx_shipments_tracking` ON (tracking_number)
- `idx_shipments_status` ON (status)

### 8.3 物流轨迹表 (shipment_tracking)

| 字段名 | 数据类型 | 必填 | 默认值 | 说明 |
|-------|---------|-----|-------|------|
| id | INTEGER | 是 | AUTO | 主键 |
| shipment_id | INTEGER | 是 | - | 发货单ID，外键 |
| tracking_time | TEXT | 是 | - | 轨迹时间 |
| tracking_status | TEXT | 是 | - | 轨迹状态码 |
| tracking_description | TEXT | 是 | - | 轨迹描述 |
| location | TEXT | 否 | NULL | 所在位置 |
| raw_data | TEXT | 否 | NULL | 原始数据(JSON) |
| created_at | TEXT | 是 | datetime('now') | 创建时间 |

**索引**：
- `idx_tracking_shipment` ON (shipment_id)
- `idx_tracking_time` ON (tracking_time)

---

## 9. 用户和权限模块

### 9.1 用户表 (users)

| 字段名 | 数据类型 | 必填 | 默认值 | 说明 |
|-------|---------|-----|-------|------|
| id | INTEGER | 是 | AUTO | 主键 |
| username | TEXT | 是 | - | 用户名，唯一 |
| password_hash | TEXT | 是 | - | 密码哈希 |
| email | TEXT | 否 | NULL | 邮箱，唯一 |
| mobile | TEXT | 否 | NULL | 手机号，唯一 |
| real_name | TEXT | 否 | NULL | 真实姓名 |
| avatar | TEXT | 否 | NULL | 头像 |
| status | INTEGER | 是 | 1 | 状态：1正常 2禁用 |
| last_login_at | TEXT | 否 | NULL | 最后登录时间 |
| last_login_ip | TEXT | 否 | NULL | 最后登录IP |
| created_at | TEXT | 是 | datetime('now') | 创建时间 |
| updated_at | TEXT | 是 | datetime('now') | 更新时间 |
| deleted_at | TEXT | 否 | NULL | 软删除时间 |

### 9.2 角色表 (roles)

| 字段名 | 数据类型 | 必填 | 默认值 | 说明 |
|-------|---------|-----|-------|------|
| id | INTEGER | 是 | AUTO | 主键 |
| name | TEXT | 是 | - | 角色名称，唯一 |
| code | TEXT | 是 | - | 角色代码，唯一 |
| description | TEXT | 否 | NULL | 描述 |
| permissions | TEXT | 否 | '[]' | 权限列表(JSON) |
| status | INTEGER | 是 | 1 | 状态：1启用 2禁用 |
| created_at | TEXT | 是 | datetime('now') | 创建时间 |
| updated_at | TEXT | 是 | datetime('now') | 更新时间 |

### 9.3 用户角色关联表 (user_roles)

| 字段名 | 数据类型 | 必填 | 默认值 | 说明 |
|-------|---------|-----|-------|------|
| id | INTEGER | 是 | AUTO | 主键 |
| user_id | INTEGER | 是 | - | 用户ID，外键 |
| role_id | INTEGER | 是 | - | 角色ID，外键 |
| created_at | TEXT | 是 | datetime('now') | 创建时间 |

---

## 10. 通用字段约定

### 10.1 时间字段
- `created_at`: 创建时间，默认 `datetime('now')`
- `updated_at`: 更新时间，每次更新时自动更新
- `deleted_at`: 软删除时间，NULL 表示未删除

### 10.2 状态字段
- 使用 INTEGER 类型
- 1 通常表示正常/启用状态
- 数值越大通常表示状态越靠后或越异常

### 10.3 JSON 字段
- 使用 TEXT 类型存储
- 默认值使用字符串形式：`'{}'` 或 `'[]'`
- 使用 `serde_json` 进行序列化/反序列化

### 10.4 外键约束
- 启用 `PRAGMA foreign_keys = ON`
- 删除行为：CASCADE 或 SET NULL

---

**文档结束**

**更新记录**:
- 2026-02-27: v1.0 初始版本，完整数据模型定义
