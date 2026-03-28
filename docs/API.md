# ciciERP API 文档

## 文档信息
- **版本**: v1.0
- **Base URL**: http://localhost:3000

---

## 通用说明

### 认证
部分 API 需要在请求头中携带 JWT Token：
```
Authorization: Bearer <token>
```

### 响应格式
所有 API 返回统一的 JSON 格式：

**成功响应**
```json
{
  "code": 200,
  "message": "success",
  "data": { ... },
  "timestamp": 1709000000
}
```

**错误响应**
```json
{
  "code": 400,
  "message": "错误描述",
  "timestamp": 1709000000
}
```

### 分页格式
```json
{
  "code": 200,
  "message": "success",
  "data": {
    "items": [...],
    "pagination": {
      "page": 1,
      "page_size": 20,
      "total": 100,
      "total_pages": 5
    }
  }
}
```

### HTTP 状态码

| 状态码 | 说明 |
|-------|------|
| 200 | 成功 |
| 400 | 请求参数错误 |
| 401 | 未授权 |
| 403 | 禁止访问 |
| 404 | 资源不存在 |
| 409 | 资源冲突 |
| 422 | 验证失败 |
| 500 | 服务器内部错误 |

---

## 系统健康检查

### GET /health

健康检查接口

**响应**:
```json
{
  "status": "ok",
  "database": "ok",
  "timestamp": "2026-02-27T10:00:00Z"
}
```

**示例**:
```bash
curl -X GET "http://localhost:3000/health"
```

---

## 产品管理

### GET /api/v1/products

获取产品列表

**查询参数**:

| 参数 | 类型 | 必填 | 说明 |
|-----|------|-----|------|
| page | number | 否 | 页码，默认1 |
| page_size | number | 否 | 每页数量，默认20，最大100 |
| category_id | number | 否 | 分类ID |
| brand_id | number | 否 | 品牌ID |
| status | number | 否 | 状态：1上架 2下架 3草稿 |
| keyword | string | 否 | 搜索关键词 |

**响应**: `PagedResponse<ProductListItem>`

**示例**:
```bash
curl -X GET "http://localhost:3000/api/v1/products?page=1&page_size=20"
```

---

### GET /api/v1/products/search

全文搜索产品（使用 SQLite FTS5）

**查询参数**:

| 参数 | 类型 | 必填 | 说明 |
|-----|------|-----|------|
| keyword | string | 是 | 搜索关键词 |
| page | number | 否 | 页码，默认1 |
| page_size | number | 否 | 每页数量，默认20 |

**响应**: `PagedResponse<ProductListItem>`

**示例**:
```bash
curl -X GET "http://localhost:3000/api/v1/products/search?keyword=手机"
```

---

### GET /api/v1/products/:id

获取产品详情

**路径参数**:
- `id` (number): 产品ID

**响应**: `ProductDetail`

**示例**:
```bash
curl -X GET "http://localhost:3000/api/v1/products/1"
```

---

### POST /api/v1/products

创建产品

**请求体**: `CreateProductRequest`

```json
{
  "product_code": "P001",
  "name": "产品名称",
  "name_en": "Product Name",
  "category_id": 1,
  "brand_id": 1,
  "purchase_price": 10.00,
  "sale_price": 20.00,
  "compare_price": 25.00,
  "description": "产品描述",
  "main_image": "https://example.com/image.jpg",
  "status": 1
}
```

**响应**: `Product`

**示例**:
```bash
curl -X POST "http://localhost:3000/api/v1/products" \
  -H "Content-Type: application/json" \
  -d '{"product_code":"P001","name":"测试产品","purchase_price":10,"sale_price":20}'
```

---

### PUT /api/v1/products/:id

更新产品

**路径参数**:
- `id` (number): 产品ID

**请求体**: `UpdateProductRequest`

```json
{
  "name": "新名称",
  "sale_price": 25.00
}
```

**响应**: `Product`

**示例**:
```bash
curl -X PUT "http://localhost:3000/api/v1/products/1" \
  -H "Content-Type: application/json" \
  -d '{"name":"新名称"}'
```

---

### DELETE /api/v1/products/:id

删除产品（软删除）

**路径参数**:
- `id` (number): 产品ID

**响应**:
```json
{
  "code": 200,
  "message": "删除成功"
}
```

**示例**:
```bash
curl -X DELETE "http://localhost:3000/api/v1/products/1"
```

---

## 供应商管理

### GET /api/v1/suppliers

获取供应商列表

**查询参数**:

| 参数 | 类型 | 必填 | 说明 |
|-----|------|-----|------|
| page | number | 否 | 页码，默认1 |
| page_size | number | 否 | 每页数量，默认20 |
| status | number | 否 | 状态：1合作 2暂停 3终止 |
| rating_level | string | 否 | 评级 A/B/C/D |
| keyword | string | 否 | 搜索关键词 |

**响应**: `PagedResponse<Supplier>`

**示例**:
```bash
curl -X GET "http://localhost:3000/api/v1/suppliers?page=1&page_size=20"
```

---

### GET /api/v1/suppliers/:id

获取供应商详情

**路径参数**:
- `id` (number): 供应商ID

**响应**: `SupplierDetail`

**示例**:
```bash
curl -X GET "http://localhost:3000/api/v1/suppliers/1"
```

---

### POST /api/v1/suppliers

创建供应商

**请求体**: `CreateSupplierRequest`

```json
{
  "supplier_code": "S001",
  "name": "供应商名称",
  "contact_person": "联系人",
  "contact_phone": "13800138000",
  "contact_email": "contact@example.com",
  "address": "地址",
  "rating_level": "B"
}
```

**响应**: `Supplier`

**示例**:
```bash
curl -X POST "http://localhost:3000/api/v1/suppliers" \
  -H "Content-Type: application/json" \
  -d '{"supplier_code":"S001","name":"测试供应商"}'
```

---

### PUT /api/v1/suppliers/:id

更新供应商

**路径参数**:
- `id` (number): 供应商ID

**请求体**: `UpdateSupplierRequest`

**响应**: `Supplier`

**示例**:
```bash
curl -X PUT "http://localhost:3000/api/v1/suppliers/1" \
  -H "Content-Type: application/json" \
  -d '{"name":"新名称"}'
```

---

### DELETE /api/v1/suppliers/:id

删除供应商（软删除）

**路径参数**:
- `id` (number): 供应商ID

**响应**:
```json
{
  "code": 200,
  "message": "删除成功"
}
```

---

### GET /api/v1/suppliers/:id/products

获取供应商的产品列表

**路径参数**:
- `id` (number): 供应商ID

**响应**: `[ProductSupplierInfo]`

**示例**:
```bash
curl -X GET "http://localhost:3000/api/v1/suppliers/1/products"
```

---

## 客户管理

### GET /api/v1/customers

获取客户列表

**查询参数**:

| 参数 | 类型 | 必填 | 说明 |
|-----|------|-----|------|
| page | number | 否 | 页码，默认1 |
| page_size | number | 否 | 每页数量，默认20 |
| level_id | number | 否 | 客户等级ID |
| status | number | 否 | 状态：1正常 2冻结 3黑名单 |
| source | string | 否 | 来源平台 |
| keyword | string | 否 | 搜索关键词 |

**响应**: `PagedResponse<Customer>`

**示例**:
```bash
curl -X GET "http://localhost:3000/api/v1/customers?page=1&page_size=20"
```

---

### GET /api/v1/customers/:id

获取客户详情

**路径参数**:
- `id` (number): 客户ID

**响应**: `Customer`

**示例**:
```bash
curl -X GET "http://localhost:3000/api/v1/customers/1"
```

---

### POST /api/v1/customers

创建客户

**请求体**: `CreateCustomerRequest`

```json
{
  "name": "客户名称",
  "mobile": "13800138000",
  "email": "customer@example.com",
  "source": "manual"
}
```

**响应**: `Customer`

**示例**:
```bash
curl -X POST "http://localhost:3000/api/v1/customers" \
  -H "Content-Type: application/json" \
  -d '{"name":"张三","source":"manual"}'
```

---

### PUT /api/v1/customers/:id

更新客户

**路径参数**:
- `id` (number): 客户ID

**请求体**: `UpdateCustomerRequest`

**响应**: `Customer`

**示例**:
```bash
curl -X PUT "http://localhost:3000/api/v1/customers/1" \
  -H "Content-Type: application/json" \
  -d '{"name":"新名称"}'
```

---

### DELETE /api/v1/customers/:id

删除客户（软删除）

**路径参数**:
- `id` (number): 客户ID

**响应**:
```json
{
  "code": 200,
  "message": "删除成功"
}
```

---

## 订单管理

### GET /api/v1/orders

获取订单列表

**查询参数**:

| 参数 | 类型 | 必填 | 说明 |
|-----|------|-----|------|
| page | number | 否 | 页码，默认1 |
| page_size | number | 否 | 每页数量，默认20 |
| order_status | number | 否 | 订单状态 |
| payment_status | number | 否 | 支付状态 |
| customer_id | number | 否 | 客户ID |
| platform | string | 否 | 来源平台 |
| date_from | string | 否 | 开始日期 YYYY-MM-DD |
| date_to | string | 否 | 结束日期 YYYY-MM-DD |
| keyword | string | 否 | 搜索关键词 |

**响应**: `PagedResponse<OrderListItem>`

**示例**:
```bash
curl -X GET "http://localhost:3000/api/v1/orders?page=1&page_size=20"
```

---

### GET /api/v1/orders/:id

获取订单详情

**路径参数**:
- `id` (number): 订单ID

**响应**: `OrderDetail`

**示例**:
```bash
curl -X GET "http://localhost:3000/api/v1/orders/1"
```

---

### POST /api/v1/orders

创建订单

**请求体**: `CreateOrderRequest`

```json
{
  "platform": "manual",
  "customer_name": "客户名称",
  "customer_mobile": "13800138000",
  "items": [
    {
      "product_name": "产品A",
      "quantity": 1,
      "unit_price": 100.00
    }
  ],
  "receiver_name": "收货人",
  "receiver_phone": "13800138000",
  "country": "CN",
  "province": "广东省",
  "city": "深圳市",
  "address": "详细地址"
}
```

**响应**: `Order`

**示例**:
```bash
curl -X POST "http://localhost:3000/api/v1/orders" \
  -H "Content-Type: application/json" \
  -d '{"platform":"manual","items":[{"product_name":"产品A","quantity":1,"unit_price":100}],"receiver_name":"张三","receiver_phone":"13800138000","country":"CN","address":"测试地址"}'
```

---

### PUT /api/v1/orders/:id

更新订单（内部备注、状态等）

**路径参数**:
- `id` (number): 订单ID

**请求体**: `UpdateOrderRequest`

```json
{
  "internal_note": "备注信息"
}
```

**响应**: `Order`

**示例**:
```bash
curl -X PUT "http://localhost:3000/api/v1/orders/1" \
  -H "Content-Type: application/json" \
  -d '{"internal_note":"备注信息"}'
```

---

### POST /api/v1/orders/:id/ship

订单发货

**路径参数**:
- `id` (number): 订单ID

**请求体**: `ShipOrderRequest`

```json
{
  "logistics_name": "顺丰速运",
  "tracking_number": "SF1234567890",
  "shipping_note": "发货备注"
}
```

**响应**:
```json
{
  "code": 200,
  "message": "发货成功"
}
```

**示例**:
```bash
curl -X POST "http://localhost:3000/api/v1/orders/1/ship" \
  -H "Content-Type: application/json" \
  -d '{"tracking_number":"SF1234567890"}'
```

---

### POST /api/v1/orders/:id/cancel

取消订单

**路径参数**:
- `id` (number): 订单ID

**请求体**: `CancelOrderRequest`

```json
{
  "reason": "客户要求取消"
}
```

**响应**:
```json
{
  "code": 200,
  "message": "订单已取消"
}
```

**示例**:
```bash
curl -X POST "http://localhost:3000/api/v1/orders/1/cancel" \
  -H "Content-Type: application/json" \
  -d '{"reason":"客户要求取消"}'
```

---

## 库存管理

### GET /api/v1/inventory

获取库存列表

**查询参数**:

| 参数 | 类型 | 必填 | 说明 |
|-----|------|-----|------|
| page | number | 否 | 页码，默认1 |
| page_size | number | 否 | 每页数量，默认20 |
| low_stock | boolean | 否 | 只显示低库存 |
| sku_code | string | 否 | SKU编码 |
| product_name | string | 否 | 产品名称 |

**响应**: `PagedResponse<InventoryListItem>`

**示例**:
```bash
curl -X GET "http://localhost:3000/api/v1/inventory?page=1&page_size=20"
```

---

### GET /api/v1/inventory/:sku_id

获取指定 SKU 的库存

**路径参数**:
- `sku_id` (number): SKU ID

**响应**: `Inventory`

**示例**:
```bash
curl -X GET "http://localhost:3000/api/v1/inventory/1"
```

---

### PUT /api/v1/inventory/:sku_id

更新库存数量

**路径参数**:
- `sku_id` (number): SKU ID

**请求体**: `UpdateInventoryRequest`

```json
{
  "quantity": 100,
  "note": "盘点调整"
}
```

**响应**: `Inventory`

**示例**:
```bash
curl -X PUT "http://localhost:3000/api/v1/inventory/1" \
  -H "Content-Type: application/json" \
  -d '{"quantity":100,"note":"盘点调整"}'
```

---

### POST /api/v1/inventory/lock

锁定库存（下单时使用）

**请求体**: `LockInventoryRequest`

```json
{
  "sku_id": 1,
  "quantity": 10,
  "order_id": 123
}
```

**响应**:
```json
{
  "code": 200,
  "message": "锁定成功"
}
```

**示例**:
```bash
curl -X POST "http://localhost:3000/api/v1/inventory/lock" \
  -H "Content-Type: application/json" \
  -d '{"sku_id":1,"quantity":10}'
```

---

### POST /api/v1/inventory/unlock

解锁库存（订单取消时使用）

**请求体**: `UnlockInventoryRequest`

```json
{
  "sku_id": 1,
  "quantity": 10,
  "order_id": 123
}
```

**响应**:
```json
{
  "code": 200,
  "message": "解锁成功"
}
```

**示例**:
```bash
curl -X POST "http://localhost:3000/api/v1/inventory/unlock" \
  -H "Content-Type: application/json" \
  -d '{"sku_id":1,"quantity":10}'
```

---

### GET /api/v1/inventory/alerts

获取库存预警列表

**响应**: `[InventoryAlert]`

**示例**:
```bash
curl -X GET "http://localhost:3000/api/v1/inventory/alerts"
```

---

## 数据模型

### Product (产品)

| 字段 | 类型 | 说明 |
|-----|------|------|
| id | number | 产品ID |
| product_code | string | 产品编码 |
| name | string | 产品名称 |
| category_id | number | 分类ID |
| brand_id | number | 品牌ID |
| purchase_price | number | 采购价 |
| sale_price | number | 销售价 |
| status | number | 状态：1上架 2下架 3草稿 |
| created_at | string | 创建时间 |

### Supplier (供应商)

| 字段 | 类型 | 说明 |
|-----|------|------|
| id | number | 供应商ID |
| supplier_code | string | 供应商编码 |
| name | string | 供应商名称 |
| contact_person | string | 联系人 |
| contact_phone | string | 联系电话 |
| rating_level | string | 评级 |
| status | number | 状态 |

### Customer (客户)

| 字段 | 类型 | 说明 |
|-----|------|------|
| id | number | 客户ID |
| customer_code | string | 客户编码 |
| name | string | 客户名称 |
| mobile | string | 手机号 |
| email | string | 邮箱 |
| level_id | number | 等级ID |
| total_orders | number | 订单总数 |
| total_amount | number | 消费总额 |

### Order (订单)

| 字段 | 类型 | 说明 |
|-----|------|------|
| id | number | 订单ID |
| order_code | string | 订单号 |
| customer_name | string | 客户名称 |
| total_amount | number | 订单总额 |
| order_status | number | 订单状态 |
| payment_status | number | 支付状态 |
| fulfillment_status | number | 履约状态 |
| created_at | string | 创建时间 |

### Inventory (库存)

| 字段 | 类型 | 说明 |
|-----|------|------|
| id | number | 库存ID |
| sku_id | number | SKU ID |
| total_quantity | number | 库存总数 |
| available_quantity | number | 可用库存 |
| locked_quantity | number | 锁定库存 |
| safety_stock | number | 安全库存 |

---

**文档结束**

**更新记录**:
- 2026-02-27: v1.0 初始版本
