# ciciERP 功能清单

> 生成日期：2026-03-28
> 项目版本：v0.1.0
> 技术栈：Rust + Axum + SQLite

---

## 目录

1. [功能模块概览](#功能模块概览)
2. [用户与认证模块](#1-用户与认证模块)
3. [产品模块](#2-产品模块)
4. [客户模块](#3-客户模块)
5. [供应商模块](#4-供应商模块)
6. [订单模块](#5-订单模块)
7. [采购模块](#6-采购模块)
8. [库存模块](#7-库存模块)
9. [物流模块](#8-物流模块)
10. [发票模块](#9-发票模块)
11. [汇率模块](#10-汇率模块)
12. [集成对接模块](#11-集成对接模块)
13. [Web 管理界面](#12-web-管理界面)

---

## 功能模块概览

| 模块 | API 端点数 | Web 页面数 | 状态 |
|------|-----------|-----------|------|
| 用户与认证 | 8 | 1 | ✅ 已实现 |
| 产品管理 | 16 | 5 | ✅ 已实现 |
| 客户管理 | 6 | 7 | ✅ 已实现 |
| 供应商管理 | 6 | 4 | ✅ 已实现 |
| 订单管理 | 9 | 5 | ✅ 已实现 |
| 采购管理 | 7 | 3 | ✅ 已实现 |
| 库存管理 | 6 | 5 | ✅ 已实现 |
| 物流管理 | 10 | 1 | ✅ 已实现 |
| 发票管理 (PI/CI) | 14 | 4 | ✅ 已实现 |
| 汇率管理 | 4 | 0 | ✅ 已实现 |
| 集成对接 | 17 | 0 | 🚧 部分实现 |
| Web 界面 | - | 35+ | ✅ 已实现 |

**总计**：103+ API 端点，35+ Web 页面

---

## 1. 用户与认证模块

### 功能点

| 功能 | 描述 | API | Web |
|------|------|-----|-----|
| 用户登录 | 用户名密码登录，JWT Token 认证 | ✅ | ✅ |
| 用户登出 | 清除 Token，退出系统 | ✅ | ✅ |
| 获取当前用户 | 获取登录用户信息 | ✅ | ✅ |
| 修改密码 | 用户修改自己的密码 | ✅ | - |
| 用户列表 | 分页查询用户列表（管理员） | ✅ | - |
| 创建用户 | 创建新用户（管理员） | ✅ | - |
| 编辑用户 | 修改用户信息（管理员） | ✅ | - |
| 删除用户 | 软删除用户（管理员） | ✅ | - |
| 重置密码 | 管理员重置用户密码 | ✅ | - |
| 角色列表 | 获取系统角色列表 | ✅ | - |

### API 端点

```
POST   /api/v1/auth/login           # 用户登录
POST   /api/v1/auth/logout          # 用户登出
GET    /api/v1/auth/me              # 获取当前用户
POST   /api/v1/auth/password        # 修改密码
GET    /api/v1/users                # 用户列表
POST   /api/v1/users                # 创建用户
GET    /api/v1/users/:id            # 获取用户详情
PUT    /api/v1/users/:id            # 更新用户
DELETE /api/v1/users/:id            # 删除用户
POST   /api/v1/users/:id/reset-password  # 重置密码
GET    /api/v1/roles                # 角色列表
```

### Web 页面

- `/login` - 登录页面

### 数据模型

- **User**: 用户基本信息（用户名、密码哈希、邮箱、手机、真实姓名、状态）
- **Role**: 角色（管理员、普通用户等）
- **Permission**: 权限定义

---

## 2. 产品模块

### 功能点

| 功能 | 描述 | API | Web |
|------|------|-----|-----|
| 产品列表 | 分页查询产品，支持分类、品牌、状态筛选 | ✅ | ✅ |
| 产品搜索 | 全文搜索产品（SQLite FTS5） | ✅ | ✅ |
| 产品详情 | 获取产品详情含 SKU 列表 | ✅ | ✅ |
| 创建产品 | 创建新产品 | ✅ | ✅ |
| 编辑产品 | 修改产品信息 | ✅ | ✅ |
| 删除产品 | 软删除产品 | ✅ | - |
| 历史价格 | 获取产品历史成交价格 | ✅ | - |
| 价格统计 | 获取产品价格汇总（参考价、平均价） | ✅ | - |
| 产品成本管理 | 创建/更新/删除产品成本 | ✅ | - |
| 产品价格管理 | 多平台价格设置 | ✅ | ✅ |
| 产品内容管理 | 中英文内容、SEO 信息 | ✅ | - |

### API 端点

```
# 产品基础
GET    /api/v1/products                  # 产品列表
GET    /api/v1/products/search           # 产品搜索
POST   /api/v1/products                  # 创建产品
GET    /api/v1/products/:id              # 产品详情
PUT    /api/v1/products/:id              # 更新产品
DELETE /api/v1/products/:id              # 删除产品
GET    /api/v1/products/:id/history-prices    # 历史价格
GET    /api/v1/products/:id/price-summary     # 价格统计

# 产品成本
GET    /api/v1/products/:product_id/cost      # 获取当前成本
POST   /api/v1/products/:product_id/cost      # 创建成本
PUT    /api/v1/products/:product_id/cost      # 更新成本
DELETE /api/v1/products/:product_id/cost      # 删除成本
GET    /api/v1/products/:product_id/costs     # 成本历史
GET    /api/v1/product-costs/:id              # 按ID获取成本
PUT    /api/v1/product-costs/:id              # 按ID更新成本
DELETE /api/v1/product-costs/:id              # 按ID删除成本

# 产品价格
GET    /api/v1/products/:id/prices            # 价格列表
GET    /api/v1/products/:id/prices/summary    # 价格汇总
POST   /api/v1/products/:id/prices            # 创建价格
PUT    /api/v1/products/:product_id/prices/:price_id    # 更新价格
DELETE /api/v1/products/:product_id/prices/:price_id    # 删除价格

# 产品内容
GET    /api/v1/products/:product_id/content   # 获取内容
POST   /api/v1/products/:product_id/content   # 创建内容
PUT    /api/v1/products/:product_id/content   # 创建/更新内容
DELETE /api/v1/products/:product_id/content   # 删除内容
```

### Web 页面

- `/products` - 产品列表页
- `/products/new` - 新建产品页
- `/products/:id` - 产品详情页
- `/products/:id/edit` - 编辑产品页

### 数据模型

- **Product**: 产品（编码、名称、英文名、分类、品牌、状态）
- **ProductSku**: SKU（规格、价格、库存）
- **ProductCost**: 成本记录（采购成本、加工成本、运费）
- **ProductPrice**: 销售价格（平台、CNY/USD、是否参考价）
- **ProductContent**: 内容（中英文标题、描述、SEO 信息）

---

## 3. 客户模块

### 功能点

| 功能 | 描述 | API | Web |
|------|------|-----|-----|
| 客户列表 | 分页查询客户，支持等级、状态、来源筛选 | ✅ | ✅ |
| 客户详情 | 获取客户信息 | ✅ | ✅ |
| 创建客户 | 手动创建客户 | ✅ | ✅ |
| 编辑客户 | 修改客户信息 | ✅ | ✅ |
| 删除客户 | 软删除客户 | ✅ | - |
| 地址管理 | 客户收货地址 CRUD | ✅ | ✅ |
| 设置默认地址 | 设置默认收货地址 | - | ✅ |

### API 端点

```
GET    /api/v1/customers                 # 客户列表
POST   /api/v1/customers                 # 创建客户
GET    /api/v1/customers/:id             # 客户详情
PUT    /api/v1/customers/:id             # 更新客户
DELETE /api/v1/customers/:id             # 删除客户
GET    /api/v1/customers/:id/addresses   # 客户地址列表
```

### Web 页面

- `/customers` - 客户列表页
- `/customers/new` - 新建客户页
- `/customers/:id` - 客户详情页
- `/customers/:id/edit` - 编辑客户页
- `/customers/:id/addresses` - 地址管理

### 数据模型

- **Customer**: 客户（姓名、邮箱、手机、等级、状态、来源）
- **CustomerAddress**: 收货地址（收件人、电话、国家/省/市/地址）

---

## 4. 供应商模块

### 功能点

| 功能 | 描述 | API | Web |
|------|------|-----|-----|
| 供应商列表 | 分页查询供应商 | ✅ | ✅ |
| 供应商详情 | 获取供应商信息及关联产品 | ✅ | ✅ |
| 创建供应商 | 创建新供应商 | ✅ | ✅ |
| 编辑供应商 | 修改供应商信息 | ✅ | ✅ |
| 删除供应商 | 软删除供应商 | ✅ | - |
| 供应商产品 | 获取供应商关联的产品 | ✅ | - |

### API 端点

```
GET    /api/v1/suppliers                 # 供应商列表
POST   /api/v1/suppliers                 # 创建供应商
GET    /api/v1/suppliers/:id             # 供应商详情
PUT    /api/v1/suppliers/:id             # 更新供应商
DELETE /api/v1/suppliers/:id             # 删除供应商
GET    /api/v1/suppliers/:id/products    # 供应商产品
```

### Web 页面

- `/suppliers` - 供应商列表页
- `/suppliers/new` - 新建供应商页
- `/suppliers/:id` - 供应商详情页
- `/suppliers/:id/edit` - 编辑供应商页

### 数据模型

- **Supplier**: 供应商（编码、名称、联系方式、评级、状态）

---

## 5. 订单模块

### 功能点

| 功能 | 描述 | API | Web |
|------|------|-----|-----|
| 订单列表 | 分页查询订单，支持状态、日期、客户筛选 | ✅ | ✅ |
| 订单详情 | 获取订单详情含商品、收货地址 | ✅ | ✅ |
| 创建订单 | 手动创建订单 | ✅ | ✅ |
| 编辑订单 | 修改订单内部备注、状态 | ✅ | ✅ |
| 状态更新 | 更新订单状态 | ✅ | - |
| 订单发货 | 填写物流单号发货 | ✅ | ✅ |
| 取消订单 | 取消订单并恢复库存 | ✅ | - |
| 下载 PI | 下载形式发票 Excel | ✅ | - |
| 下载 CI | 下载商业发票 Excel | ✅ | - |

### API 端点

```
GET    /api/v1/orders                    # 订单列表
POST   /api/v1/orders                    # 创建订单
GET    /api/v1/orders/:id                # 订单详情
PUT    /api/v1/orders/:id                # 更新订单
POST   /api/v1/orders/:id/status         # 更新状态
POST   /api/v1/orders/:id/ship           # 发货
POST   /api/v1/orders/:id/cancel         # 取消
GET    /api/v1/orders/:id/download-pi    # 下载 PI
GET    /api/v1/orders/:id/download-ci    # 下载 CI
```

### Web 页面

- `/orders` - 订单列表页
- `/orders/new` - 新建订单页
- `/orders/:id` - 订单详情页
- `/orders/:id/edit` - 编辑订单页

### 数据模型

- **Order**: 订单（订单号、客户、金额、状态、支付状态）
- **OrderItem**: 订单商品（产品、数量、单价、小计）
- **OrderAddress**: 收货地址

### 订单状态流转

```
1. 待确认 → 2. 已确认/锁价 → 3. 已付款 → 4. 已发货 → 5. 已完成
                    ↓
              6. 已取消
```

---

## 6. 采购模块

### 功能点

| 功能 | 描述 | API | Web |
|------|------|-----|-----|
| 采购单列表 | 分页查询采购单 | ✅ | ✅ |
| 采购单详情 | 获取采购单详情含商品 | ✅ | ✅ |
| 创建采购单 | 一单多供应商模式 | ✅ | ✅ |
| 编辑采购单 | 仅待审核状态可修改 | ✅ | - |
| 删除采购单 | 仅待审核状态可删除 | ✅ | - |
| 审批采购单 | 审批通过/拒绝 | ✅ | - |
| 采购入库 | 按SKU入库，记录良品/次品 | ✅ | - |

### API 端点

```
GET    /api/v1/purchases                 # 采购单列表
POST   /api/v1/purchases                 # 创建采购单
GET    /api/v1/purchases/:id             # 采购单详情
PUT    /api/v1/purchases/:id             # 更新采购单
DELETE /api/v1/purchases/:id             # 删除采购单
POST   /api/v1/purchases/:id/approve     # 审批
POST   /api/v1/purchases/:id/receive     # 入库
```

### Web 页面

- `/purchase` - 采购单列表页
- `/purchase/new` - 新建采购单页
- `/purchase/:id` - 采购单详情页

### 数据模型

- **PurchaseOrder**: 采购单（订单号、状态、付款状态、发货状态）
- **PurchaseOrderItem**: 采购商品（产品、供应商、数量、单价）

### 采购单状态流转

```
1. 待审核 → 2. 已审批 → 3. 部分到货 → 4. 已完成
                ↓
          5. 已取消
```

---

## 7. 库存模块

### 功能点

| 功能 | 描述 | API | Web |
|------|------|-----|-----|
| 库存列表 | 分页查询库存，支持低库存筛选 | ✅ | ✅ |
| 库存详情 | 获取 SKU 库存信息 | ✅ | ✅ |
| 更新库存 | 手动调整库存数量 | ✅ | ✅ |
| 库存锁定 | 下单时锁定库存 | ✅ | - |
| 库存解锁 | 订单取消时解锁库存 | ✅ | - |
| 库存预警 | 获取低于安全库存的 SKU | ✅ | ✅ |
| 库存流水 | 查看库存变动记录 | - | ✅ |

### API 端点

```
GET    /api/v1/inventory                 # 库存列表
GET    /api/v1/inventory/alerts          # 库存预警
GET    /api/v1/inventory/:sku_id         # 库存详情
PUT    /api/v1/inventory/:sku_id         # 更新库存
POST   /api/v1/inventory/lock            # 锁定库存
POST   /api/v1/inventory/unlock          # 解锁库存
```

### Web 页面

- `/inventory` - 库存列表页
- `/inventory/new` - 新建库存页
- `/inventory/:id` - 库存详情页
- `/inventory/:id/adjust` - 库存调整页
- `/inventory/:id/movements` - 库存流水页

### 数据模型

- **Inventory**: 库存（SKU、总数量、可用数量、锁定数量、次品数量、安全库存）
- **InventoryMovement**: 库存流水（变动类型、数量、关联单据）

---

## 8. 物流模块

### 功能点

| 功能 | 描述 | API | Web |
|------|------|-----|-----|
| 物流公司列表 | 获取物流公司列表 | ✅ | ✅ |
| 创建物流公司 | 添加物流公司 | ✅ | - |
| 编辑物流公司 | 修改物流公司信息 | ✅ | - |
| 删除物流公司 | 禁用物流公司 | ✅ | - |
| 发货单列表 | 分页查询发货单 | ✅ | - |
| 发货单详情 | 获取发货单详情含物流轨迹 | ✅ | - |
| 创建发货单 | 订单发货 | ✅ | - |
| 更新发货单 | 修改物流信息 | ✅ | - |
| 物流轨迹 | 获取物流轨迹 | ✅ | - |
| 添加轨迹 | 添加物流轨迹节点 | ✅ | - |

### API 端点

```
# 物流公司
GET    /api/v1/logistics/companies       # 物流公司列表
POST   /api/v1/logistics/companies       # 创建物流公司
PUT    /api/v1/logistics/companies/:id   # 更新物流公司
DELETE /api/v1/logistics/companies/:id   # 删除物流公司

# 发货单
GET    /api/v1/shipments                 # 发货单列表
POST   /api/v1/shipments                 # 创建发货单
GET    /api/v1/shipments/:id             # 发货单详情
PUT    /api/v1/shipments/:id             # 更新发货单
GET    /api/v1/shipments/:id/tracking    # 物流轨迹
POST   /api/v1/shipments/:id/tracking    # 添加轨迹
```

### Web 页面

- `/logistics` - 物流管理页

### 数据模型

- **LogisticsCompany**: 物流公司（编码、名称、服务类型、API 配置）
- **Shipment**: 发货单（发货单号、订单、物流公司、运单号、状态）
- **ShipmentTracking**: 物流轨迹（时间、状态、描述）

---

## 9. 发票模块

### 9.1 形式发票 (PI - Proforma Invoice)

| 功能 | 描述 | API | Web |
|------|------|-----|-----|
| PI 列表 | 分页查询 PI | ✅ | ✅ |
| PI 详情 | 获取 PI 详情 | ✅ | ✅ |
| 创建 PI | 创建形式发票 | ✅ | ✅ |
| 编辑 PI | 修改 PI | ✅ | ✅ |
| 删除 PI | 仅草稿状态可删除 | ✅ | - |
| 发送 PI | 状态变为已发送 | ✅ | - |
| 确认 PI | 客户确认 PI | ✅ | - |
| PI 转订单 | 将 PI 转为销售订单 | ✅ | - |
| 取消 PI | 取消 PI | ✅ | - |

### API 端点

```
GET    /api/v1/proforma-invoices             # PI 列表
POST   /api/v1/proforma-invoices             # 创建 PI
GET    /api/v1/proforma-invoices/:id         # PI 详情
PUT    /api/v1/proforma-invoices/:id         # 更新 PI
DELETE /api/v1/proforma-invoices/:id         # 删除 PI
POST   /api/v1/proforma-invoices/:id/send    # 发送 PI
POST   /api/v1/proforma-invoices/:id/confirm # 确认 PI
POST   /api/v1/proforma-invoices/:id/convert # PI 转订单
POST   /api/v1/proforma-invoices/:id/cancel  # 取消 PI
```

### 9.2 商业发票 (CI - Commercial Invoice)

| 功能 | 描述 | API | Web |
|------|------|-----|-----|
| CI 列表 | 分页查询 CI | ✅ | ✅ |
| CI 详情 | 获取 CI 详情 | ✅ | ✅ |
| 从订单创建 CI | 基于订单生成 CI | ✅ | ✅ |
| 发送 CI | 状态变为已发送 | ✅ | - |
| 标记已付款 | 记录付款信息 | ✅ | - |

### API 端点

```
GET    /api/v1/commercial-invoices                   # CI 列表
GET    /api/v1/commercial-invoices/:id               # CI 详情
POST   /api/v1/commercial-invoices/from-order/:order_id  # 从订单创建 CI
POST   /api/v1/commercial-invoices/:id/send          # 发送 CI
POST   /api/v1/commercial-invoices/:id/mark-paid     # 标记已付款
```

### Web 页面

- `/pi` - PI 列表页
- `/pi/new` - 新建 PI 页
- `/pi/:id` - PI 详情页
- `/ci` - CI 列表页
- `/ci/:id` - CI 详情页

### 数据模型

- **ProformaInvoice**: PI（PI号、客户、金额、付款条款、交货条款）
- **ProformaInvoiceItem**: PI 商品
- **CommercialInvoice**: CI（CI号、订单、金额、付款状态）
- **CommercialInvoiceItem**: CI 商品

---

## 10. 汇率模块

### 功能点

| 功能 | 描述 | API | Web |
|------|------|-----|-----|
| 获取当前汇率 | 获取 USD/CNY 汇率 | ✅ | - |
| 手动更新汇率 | 手动设置汇率 | ✅ | - |
| 汇率历史 | 查询历史汇率 | ✅ | - |
| 自动获取汇率 | 从外部 API 获取（每日定时） | ✅ | - |

### API 端点

```
GET    /api/v1/exchange-rates/current    # 当前汇率
POST   /api/v1/exchange-rates/update     # 手动更新
GET    /api/v1/exchange-rates/history    # 历史汇率
POST   /api/v1/exchange-rates/fetch      # 手动获取外部汇率
```

### 数据模型

- **ExchangeRate**: 汇率（源货币、目标货币、汇率、来源、生效日期）

---

## 11. 集成对接模块

> 用于与 cicishop 等外部系统对接

### 功能点

| 功能 | 描述 | API | 状态 |
|------|------|-----|------|
| 产品同步 | 批量获取产品信息 | ✅ | ✅ |
| 增量产品同步 | 按时间戳获取更新 | ✅ | ✅ |
| 批量查询产品 | 按ID批量获取 | ✅ | ✅ |
| 单个产品查询 | 获取产品详情 | ✅ | ✅ |
| 订单接收 | 从商城创建订单 | ✅ | 🚧 |
| 订单状态查询 | 查询订单状态 | ✅ | ✅ |
| 订单状态更新 | 更新订单状态 | ✅ | 🚧 |
| 库存同步 | 批量获取库存 | ✅ | 🚧 |
| SKU库存查询 | 按SKU查询库存 | ✅ | 🚧 |
| 库存预留 | 下单前预留库存 | ✅ | 🚧 |
| 库存释放 | 释放预留库存 | ✅ | 🚧 |
| 客户同步 | 同步客户信息 | ✅ | ✅ |
| 批量客户同步 | 批量同步客户 | ✅ | 🚧 |

### API 端点

```
# 产品同步
GET    /api/v1/integration/products              # 产品列表
GET    /api/v1/integration/products/updated      # 增量更新
POST   /api/v1/integration/products/batch        # 批量查询
GET    /api/v1/integration/products/:id          # 单个产品

# 订单
POST   /api/v1/integration/orders                # 创建订单
GET    /api/v1/integration/orders/:platform_order_id   # 查询订单
PUT    /api/v1/integration/orders/:platform_order_id   # 更新订单

# 库存
GET    /api/v1/integration/inventory             # 库存列表
GET    /api/v1/integration/inventory/sku/:sku_code    # SKU库存
POST   /api/v1/integration/inventory/reserve     # 预留库存
POST   /api/v1/integration/inventory/release     # 释放库存

# 客户
POST   /api/v1/integration/customers             # 同步客户
GET    /api/v1/integration/customers/:external_id     # 查询客户
POST   /api/v1/integration/customers/batch       # 批量同步
```

### 认证方式

- API Key + HMAC 签名
- 基于权限的访问控制

---

## 12. Web 管理界面

### 页面清单

| 路径 | 页面 | 功能 |
|------|------|------|
| `/login` | 登录页 | 用户登录 |
| `/` | 仪表盘 | 数据统计概览 |
| `/products` | 产品列表 | 产品管理 |
| `/products/new` | 新建产品 | 创建产品 |
| `/products/:id` | 产品详情 | 查看产品 |
| `/products/:id/edit` | 编辑产品 | 修改产品 |
| `/orders` | 订单列表 | 订单管理 |
| `/orders/new` | 新建订单 | 创建订单 |
| `/orders/:id` | 订单详情 | 查看订单 |
| `/orders/:id/edit` | 编辑订单 | 修改订单 |
| `/inventory` | 库存列表 | 库存管理 |
| `/inventory/new` | 新建库存 | 初始化库存 |
| `/inventory/:id` | 库存详情 | 查看库存 |
| `/inventory/:id/adjust` | 库存调整 | 调整库存 |
| `/inventory/:id/movements` | 库存流水 | 变动记录 |
| `/customers` | 客户列表 | 客户管理 |
| `/customers/new` | 新建客户 | 创建客户 |
| `/customers/:id` | 客户详情 | 查看客户 |
| `/customers/:id/edit` | 编辑客户 | 修改客户 |
| `/suppliers` | 供应商列表 | 供应商管理 |
| `/suppliers/new` | 新建供应商 | 创建供应商 |
| `/suppliers/:id` | 供应商详情 | 查看供应商 |
| `/suppliers/:id/edit` | 编辑供应商 | 修改供应商 |
| `/purchase` | 采购列表 | 采购管理 |
| `/purchase/new` | 新建采购单 | 创建采购 |
| `/purchase/:id` | 采购详情 | 查看采购 |
| `/logistics` | 物流管理 | 物流查询 |
| `/pi` | PI 列表 | PI 管理 |
| `/pi/new` | 新建 PI | 创建 PI |
| `/pi/:id` | PI 详情 | 查看 PI |
| `/ci` | CI 列表 | CI 管理 |
| `/ci/:id` | CI 详情 | 查看 CI |

### 界面特性

- 响应式设计（支持移动端）
- 侧边栏导航
- Tailwind CSS 样式
- HTMX 局部刷新
- Toast 消息提示

---

## 附录

### 技术架构

```
┌─────────────────────────────────────────────────────────┐
│                     ciciERP Architecture                │
├─────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐     │
│  │   Web UI    │  │  REST API   │  │ Integration │     │
│  │  (Tailwind) │  │   (Axum)    │  │    API      │     │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘     │
│         │                │                │             │
│         └────────────────┼────────────────┘             │
│                          ▼                              │
│  ┌───────────────────────────────────────────────────┐  │
│  │              Business Logic Layer                 │  │
│  │  Products | Orders | Inventory | Purchases | ...  │  │
│  └───────────────────────────────────────────────────┘  │
│                          │                              │
│                          ▼                              │
│  ┌───────────────────────────────────────────────────┐  │
│  │              Data Access Layer                    │  │
│  │         (SQLx + Repository Pattern)               │  │
│  └───────────────────────────────────────────────────┘  │
│                          │                              │
│                          ▼                              │
│  ┌───────────────────────────────────────────────────┐  │
│  │              Database (SQLite)                    │  │
│  └───────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

### 项目结构

```
ciciERP/
├── crates/
│   ├── api/           # API 服务 (Axum)
│   │   ├── routes/    # 路由定义
│   │   ├── middleware/# 中间件
│   │   ├── templates/ # HTML 模板
│   │   └── services/  # 业务服务
│   ├── models/        # 数据模型
│   ├── db/            # 数据库操作
│   │   └── queries/   # 查询模块
│   └── utils/         # 工具函数
├── migrations/        # 数据库迁移
├── config/            # 配置文件
├── docs/              # 文档
└── scripts/           # 脚本
```

### API 响应格式

```json
{
  "code": 200,
  "message": "success",
  "data": { ... }
}
```

### 分页响应格式

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

---

*文档生成于 2026-03-28*
