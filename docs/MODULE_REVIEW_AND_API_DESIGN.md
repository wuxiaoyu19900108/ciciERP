# ciciERP 模块评审与 API 对接方案

**报告日期**: 2026-03-24
**评审人**: 研究专家 (researcher)
**项目版本**: 0.1.0
**评审范围**: 全部已实现模块 + API 对接设计

---

## 一、已实现模块评审

### 1.1 评审总览

| 模块 | 完成度 | 代码质量 | API 设计 | 安全性 | 综合评分 |
|-----|--------|---------|---------|--------|---------|
| 产品管理 | 95% | 90 | 90 | 85 | **89** |
| 客户管理 | 90% | 85 | 85 | 85 | **85** |
| 供应商管理 | 90% | 85 | 85 | 85 | **85** |
| 采购管理 | 95% | 88 | 90 | 85 | **88** |
| 库存管理 | 95% | 90 | 88 | 85 | **88** |
| 订单管理 | 90% | 85 | 85 | 85 | **85** |
| 物流管理 | 80% | 80 | 80 | 80 | **80** |
| 汇率管理 | 100% | 90 | 85 | 85 | **87** |
| 用户认证 | 95% | 88 | 85 | 90 | **88** |

**总体评分: 86/100**

---

### 1.2 产品管理模块

#### 实现范围
- ✅ 产品主表 (products) CRUD
- ✅ SKU 管理 (product_skus)
- ✅ 产品成本 (product_costs)
- ✅ 产品售价 (product_prices) - 支持多平台
- ✅ 产品内容 (product_content) - 独立站内容
- ✅ 分类/品牌管理
- ✅ 全文搜索 (SQLite FTS5)

#### 代码质量评审

| 维度 | 评分 | 说明 |
|-----|------|------|
| API 设计 | 90 | RESTful 风格，注释完善 |
| 数据验证 | 85 | 使用 validator，验证规则完整 |
| 错误处理 | 85 | 统一 AppError 处理 |
| SQL 安全 | 90 | QueryBuilder 参数化查询 |
| 代码复用 | 80 | 部分查询逻辑可抽取公共方法 |

#### 亮点
```rust
// 产品编码自动生成 - 格式：SKU-YYYYMMDD-XXXX
pub async fn generate_product_code(&self) -> Result<String> {
    let today = chrono::Utc::now().format("%Y%m%d").to_string();
    let prefix = format!("SKU-{}-", today);
    // ...
}
```

#### 待改进
- [ ] 产品批量操作接口（批量上架/下架）
- [ ] 产品图片上传接口
- [ ] 产品规格模板功能

---

### 1.3 库存管理模块

#### 实现范围
- ✅ 库存查询/更新
- ✅ 库存锁定/解锁（订单关联）
- ✅ 库存流水记录
- ✅ 库存预警
- ✅ 多类型库存变动（入库/出库/调拨/盘点/损耗）

#### 代码质量评审

| 维度 | 评分 | 说明 |
|-----|------|------|
| 事务处理 | 95 | 库存操作使用事务保护 |
| 并发安全 | 85 | SQLite 事务级别保证 |
| 业务逻辑 | 90 | 库存锁定/解锁逻辑完整 |
| 流水记录 | 90 | 完整的变动历史追踪 |

#### 亮点
```rust
// 通用库存调整函数 - 统一处理各种变动类型
pub async fn adjust_inventory(
    &self,
    sku_id: i64,
    delta_total: i64,
    delta_available: i64,
    delta_locked: i64,
    delta_damaged: i64,
    // ...
) -> Result<()>
```

#### 待改进
- [ ] 多仓库支持（字段已预留但未实现）
- [ ] 库存快照功能
- [ ] 批量库存调整

---

### 1.4 订单管理模块

#### 实现范围
- ✅ 订单创建（含库存锁定）
- ✅ 订单状态流转
- ✅ 订单发货（含物流单创建）
- ✅ 订单取消（含库存解锁）
- ✅ 订单明细/地址管理
- ✅ 多状态查询过滤

#### 代码质量评审

| 维度 | 评分 | 说明 |
|-----|------|------|
| 业务完整性 | 90 | 状态流转逻辑完整 |
| 库存联动 | 95 | 创建锁定、取消解锁 |
| 事务一致性 | 90 | 订单+明细+地址+库存 事务保护 |
| 平台扩展 | 85 | 支持 platform 字段区分来源 |

#### 订单状态机
```
待审核(1) → 待发货(2) → 部分发货(3) → 已发货(4) → 已完成(5)
    ↓
已取消(6)
    ↓
售后中(7)
```

#### 待改进
- [ ] 订单修改接口（修改收货地址等）
- [ ] 订单拆分发货
- [ ] 退款/售后流程
- [ ] 订单导入接口

---

### 1.5 采购管理模块

#### 实现范围
- ✅ 采购单 CRUD
- ✅ 一单多供应商模式
- ✅ 采购审批流程
- ✅ 采购入库（含质检）
- ✅ 供应商关联验证

#### 代码质量评审

| 维度 | 评分 | 说明 |
|-----|------|------|
| 业务模型 | 90 | 一单多供应商设计灵活 |
| 审批流程 | 85 | 审批人记录完整 |
| 入库质检 | 85 | 合格/不合格数量分离 |
| 库存联动 | 90 | 入库自动更新库存 |

#### 待改进
- [ ] 采购退货
- [ ] 采购付款记录
- [ ] 采购报表统计

---

### 1.6 物流管理模块

#### 实现范围
- ✅ 物流公司管理
- ✅ 发货单管理
- ⚠️ 物流轨迹（表已建，API 未完善）

#### 待改进
- [ ] 物流轨迹查询/更新 API
- [ ] 物流单号自动识别
- [ ] 电子面单对接

---

## 二、API 对接需求分析

### 2.1 独立站 (cicishop) 对接需求

| 场景 | 方向 | 数据流 | 频率 | 优先级 |
|-----|------|--------|------|--------|
| 产品上架 | ERP → 商城 | 产品信息+价格+库存 | 实时/定时 | P0 |
| 订单同步 | 商城 → ERP | 订单信息+客户信息 | 实时 | P0 |
| 库存同步 | 双向 | 可用库存 | 实时 | P0 |
| 订单状态 | ERP → 商城 | 发货/取消状态 | 实时 | P1 |
| 客户同步 | 商城 → ERP | 客户注册/更新 | 实时 | P2 |

### 2.2 第三方平台对接需求（未来）

| 平台 | 场景 | 方向 | 复杂度 |
|-----|------|------|--------|
| Amazon | 产品上架、订单拉取 | 双向 | 高 |
| Shopee | 产品上架、订单拉取 | 双向 | 高 |
| Lazada | 产品上架、订单拉取 | 双向 | 高 |
| 1688 | 采购单同步 | 入向 | 中 |

---

## 三、API 接口设计方案

### 3.1 对接 API 设计原则

1. **安全性**: API Key + 签名验证
2. **幂等性**: 支持幂等键防止重复
3. **版本控制**: /api/v1/integration/
4. **限流保护**: 防止滥用
5. **日志追踪**: 完整请求日志

### 3.2 新增 API 接口列表

#### A. 产品同步 API (供商城调用)

| 方法 | 路径 | 说明 |
|-----|------|------|
| GET | /api/v1/integration/products | 批量获取产品列表（含SKU、价格、库存） |
| GET | /api/v1/integration/products/:id | 获取单个产品详情 |
| GET | /api/v1/integration/products/updated | 获取增量更新产品（按时间戳） |
| POST | /api/v1/integration/products/batch | 批量查询产品（按ID列表） |

**响应示例**:
```json
{
  "code": 200,
  "data": {
    "items": [{
      "id": 1,
      "product_code": "SKU-20260324-0001",
      "name": "产品名称",
      "name_en": "Product Name",
      "main_image": "https://...",
      "images": ["https://..."],
      "category": {"id": 1, "name": "分类"},
      "brand": {"id": 1, "name": "品牌"},
      "skus": [{
        "id": 1,
        "sku_code": "SKU-001-RED-L",
        "spec_values": {"颜色": "红色", "尺寸": "L"},
        "sale_price": 99.00,
        "available_quantity": 100,
        "status": 1
      }],
      "prices": {
        "website": {"sale_price_cny": 99.00, "sale_price_usd": 14.99},
        "amazon": {"sale_price_usd": 16.99}
      },
      "content": {
        "title_en": "...",
        "description_en": "...",
        "meta_title": "..."
      },
      "updated_at": "2026-03-24T10:00:00Z"
    }],
    "pagination": {...}
  }
}
```

#### B. 订单接收 API (商城推送)

| 方法 | 路径 | 说明 |
|-----|------|------|
| POST | /api/v1/integration/orders | 创建订单（来自商城） |
| PUT | /api/v1/integration/orders/:platform_order_id | 更新订单状态 |
| GET | /api/v1/integration/orders/:platform_order_id | 查询订单状态 |

**请求示例**:
```json
{
  "idempotency_key": "shop-order-12345",
  "platform": "cicishop",
  "platform_order_id": "ORD20260324001",
  "customer": {
    "external_id": "user-123",
    "name": "张三",
    "email": "test@example.com",
    "mobile": "13800138000"
  },
  "items": [{
    "sku_code": "SKU-001-RED-L",
    "quantity": 2,
    "unit_price": 99.00,
    "subtotal": 198.00
  }],
  "shipping": {
    "receiver_name": "张三",
    "receiver_phone": "13800138000",
    "country": "CN",
    "province": "广东省",
    "city": "深圳市",
    "address": "南山区xxx",
    "postal_code": "518000"
  },
  "total_amount": 198.00,
  "shipping_fee": 0,
  "discount_amount": 0,
  "customer_note": "尽快发货"
}
```

#### C. 库存同步 API

| 方法 | 路径 | 说明 |
|-----|------|------|
| GET | /api/v1/integration/inventory | 批量获取库存 |
| GET | /api/v1/integration/inventory/sku/:sku_code | 按SKU查询库存 |
| POST | /api/v1/integration/inventory/reserve | 预留库存（商城下单前） |
| POST | /api/v1/integration/inventory/release | 释放预留库存 |
| PUT | /api/v1/integration/inventory/sync | 库存调整（外部入库） |

**预留库存请求**:
```json
{
  "idempotency_key": "reserve-order-123",
  "items": [{
    "sku_code": "SKU-001-RED-L",
    "quantity": 2
  }],
  "reference_type": "order",
  "reference_id": "ORD20260324001"
}
```

#### D. 客户同步 API

| 方法 | 路径 | 说明 |
|-----|------|------|
| POST | /api/v1/integration/customers | 创建/更新客户 |
| GET | /api/v1/integration/customers/:external_id | 查询客户 |
| POST | /api/v1/integration/customers/batch | 批量同步客户 |

#### E. 订单状态回调 (Webhook)

| 事件 | 说明 |
|-----|------|
| order.shipped | 订单已发货 |
| order.cancelled | 订单已取消 |
| order.completed | 订单已完成 |
| inventory.low_stock | 库存低于安全库存 |
| inventory.out_of_stock | 库存售罄 |

**Webhook 载荷**:
```json
{
  "event": "order.shipped",
  "timestamp": "2026-03-24T10:00:00Z",
  "data": {
    "order_id": 1,
    "order_code": "ORD20260324001",
    "platform_order_id": "SHOP001",
    "tracking_number": "SF1234567890",
    "logistics_name": "顺丰速运",
    "ship_time": "2026-03-24T09:30:00Z"
  },
  "signature": "sha256=..."
}
```

### 3.3 需要修改的现有接口

| 接口 | 修改内容 | 原因 |
|-----|---------|------|
| POST /api/v1/orders | 增加 platform_order_id 唯一性检查 | 防止重复同步 |
| GET /api/v1/products | 增加返回 content 字段 | 商城需要完整产品信息 |
| GET /api/v1/inventory | 增加按更新时间过滤 | 支持增量同步 |

---

## 四、数据同步流程设计

### 4.1 产品同步流程

```
┌─────────────┐                    ┌─────────────┐
│   ciciERP   │                    │  cicishop   │
└──────┬──────┘                    └──────┬──────┘
       │                                  │
       │  1. 定时任务：获取更新的产品       │
       │<─────────────────────────────────│
       │                                  │
       │  GET /api/v1/integration/products?updated_after=...
       │─────────────────────────────────>│
       │                                  │
       │  2. 返回产品列表（含SKU/价格/库存） │
       │<─────────────────────────────────│
       │                                  │
       │  3. 更新本地产品数据               │
       │  (增量更新)                       │
       │                                  │
```

**同步策略**:
- 全量同步：每日凌晨执行
- 增量同步：每5分钟执行（按 updated_at）
- 首次同步：全量拉取

### 4.2 订单同步流程

```
┌─────────────┐                    ┌─────────────┐
│  cicishop   │                    │   ciciERP   │
└──────┬──────┘                    └──────┬──────┘
       │                                  │
       │  1. 用户下单                      │
       │                                  │
       │  2. 调用预留库存接口               │
       │  POST /api/v1/integration/inventory/reserve
       │─────────────────────────────────>│
       │                                  │
       │  3. 预留成功                      │
       │<─────────────────────────────────│
       │                                  │
       │  4. 创建订单                      │
       │  POST /api/v1/integration/orders │
       │─────────────────────────────────>│
       │                                  │
       │  5. 订单创建成功                  │
       │<─────────────────────────────────│
       │                                  │
       │  (后续状态变更通过 Webhook 通知)   │
       │                                  │
       │  6. 订单发货 Webhook              │
       │  POST /webhook/order.shipped     │
       │<─────────────────────────────────│
       │                                  │
```

### 4.3 库存同步流程

```
┌─────────────┐                    ┌─────────────┐
│   ciciERP   │                    │  cicishop   │
└──────┬──────┘                    └──────┬──────┘
       │                                  │
       │  库存变动触发                     │
       │  (入库/出库/盘点等)              │
       │                                  │
       │  1. 更新库存                     │
       │  2. 发送 Webhook 通知            │
       │  POST /webhook/inventory.changed │
       │─────────────────────────────────>│
       │                                  │
       │                                  │  3. 更新商城库存
       │                                  │
       │  或者：定时拉取                   │
       │<─────────────────────────────────│
       │  GET /api/v1/integration/inventory
       │─────────────────────────────────>│
       │                                  │
```

---

## 五、安全性设计

### 5.1 API 认证方案

**方案**: API Key + HMAC 签名

```
Authorization: Bearer <api_key>
X-Signature: <hmac_sha256(api_key + timestamp + body, secret)>
X-Timestamp: <unix_timestamp>
```

**实现**:
```rust
// 新增中间件：integration_auth.rs
pub async fn integration_auth_middleware(
    State(config): State<IntegrationConfig>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    let api_key = req.headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(AppError::Unauthorized)?;

    // 验证 API Key
    let client = config.get_client(api_key)
        .ok_or(AppError::Unauthorized)?;

    // 验证签名
    let signature = req.headers()
        .get("X-Signature")
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::Unauthorized)?;

    let timestamp = req.headers()
        .get("X-Timestamp")
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::Unauthorized)?;

    // 验证时间戳（防重放）
    let ts: i64 = timestamp.parse().map_err(|_| AppError::Unauthorized)?;
    let now = chrono::Utc::now().timestamp();
    if (now - ts).abs() > 300 {  // 5分钟有效期
        return Err(AppError::Unauthorized);
    }

    // 验证签名
    let body = read_body(&mut req).await?;
    let expected = compute_hmac(&client.secret, api_key, timestamp, &body);
    if signature != expected {
        return Err(AppError::Unauthorized);
    }

    Ok(next.run(req).await)
}
```

### 5.2 数据库设计

新增 `api_clients` 表:
```sql
CREATE TABLE api_clients (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    client_id TEXT NOT NULL UNIQUE,
    client_name TEXT NOT NULL,
    api_key TEXT NOT NULL UNIQUE,
    api_secret TEXT NOT NULL,
    permissions TEXT NOT NULL DEFAULT '[]',  -- JSON数组
    rate_limit INTEGER NOT NULL DEFAULT 1000,  -- 每小时请求限制
    status INTEGER NOT NULL DEFAULT 1,
    last_used_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

新增 `webhook_subscriptions` 表:
```sql
CREATE TABLE webhook_subscriptions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    client_id INTEGER NOT NULL,
    event_type TEXT NOT NULL,
    endpoint_url TEXT NOT NULL,
    secret TEXT NOT NULL,
    status INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (client_id) REFERENCES api_clients(id)
);
```

---

## 六、实施建议

### 6.1 开发优先级

| 阶段 | 任务 | 工作量 | 依赖 |
|-----|------|--------|------|
| P0 | API 认证中间件 | 2天 | - |
| P0 | 产品同步 API | 3天 | 认证中间件 |
| P0 | 订单接收 API | 3天 | 认证中间件 |
| P0 | 库存同步 API | 2天 | 认证中间件 |
| P1 | Webhook 推送 | 2天 | 订单/库存 API |
| P1 | 客户同步 API | 1天 | 认证中间件 |
| P2 | 限流机制 | 1天 | 认证中间件 |
| P2 | 日志审计 | 1天 | - |

### 6.2 测试建议

1. **单元测试**: 覆盖所有新增 API 接口
2. **集成测试**: 模拟 cicishop 调用场景
3. **压力测试**: 验证高并发下的库存一致性
4. **安全测试**: 签名验证、重放攻击防护

### 6.3 文档输出

1. API 接口文档（OpenAPI/Swagger）
2. 接入指南（供 cicishop 开发者使用）
3. 错误码说明
4. Webhook 事件说明

---

## 七、总结

### 7.1 模块评审结论

ciciERP 已实现的核心模块代码质量良好，架构清晰，具备以下特点：

**优点**:
- 清晰的 Workspace 模块化设计
- 完善的 JWT 认证和权限控制
- 参数化 SQL 查询防止注入
- 完整的库存锁定/解锁机制
- 统一的 API 响应格式

**待改进**:
- 测试覆盖率不足
- 物流轨迹功能未完善
- 部分错误消息语言不统一

### 7.2 API 对接建议

1. **优先实现**：产品同步、订单接收、库存同步 API
2. **安全第一**：API Key + 签名验证机制
3. **幂等设计**：所有写入接口支持幂等键
4. **增量同步**：基于时间戳的增量数据拉取
5. **Webhook 通知**：关键状态变更主动推送

### 7.3 预计工作量

- API 对接开发：约 10-12 人天
- 测试和调试：约 3-5 人天
- 文档编写：约 2 人天

**总计：15-20 人天**

---

*报告生成时间: 2026-03-24*
*评审人: 研究专家 (researcher)*
