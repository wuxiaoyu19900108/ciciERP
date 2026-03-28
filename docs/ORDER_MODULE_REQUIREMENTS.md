# 订单模块需求文档

**版本**: 1.0
**日期**: 2026-03-25
**状态**: 待审核

---

## 一、设计理念

**核心理念：订单是核心，PI/CI 是订单在不同状态下可下载的文件。**

- 不需要独立的 PI/CI 管理入口
- PI（形式发票）和 CI（商业发票）是订单流程中生成的文档
- 所有操作都在订单管理模块中完成

---

## 二、订单状态设计

| 状态 | 编号 | 说明 | 可下载文件 |
|------|:----:|------|-----------|
| 未成交 | 1 | 新创建，等待确认 | PI |
| 价格锁定 | 2 | 客户确认价格 | PI |
| 已付款 | 3 | 客户已付款 | CI |
| 已发货 | 4 | 已发出货物 | CI |
| 已收货 | 5 | 客户确认收货 | CI |
| 已取消 | 6 | 订单取消 | - |

**状态流转：**
```
未成交(1) → 价格锁定(2) → 已付款(3) → 已发货(4) → 已收货(5)
                                ↘ 已取消(6)
```

---

## 三、创建订单需求

### 3.1 客户信息

**功能要求：**
1. 可以从客户列表中选择已有客户
2. 选择客户后自动填充：客户名称、邮箱、电话
3. **地址处理（多地址支持）**：
   - 选择客户后，自动加载该客户的所有地址列表
   - 如果客户有地址，显示地址下拉框供选择
   - 选择地址后自动填充：收件人、电话、国家、省份、城市、区、详细地址
   - 如果客户没有地址，显示提示："该客户暂无地址信息，请手动填写"
   - 手动填写的地址可以勾选"保存到客户地址列表"
4. 也可以直接手动输入客户信息（不选择下拉）
5. 如果客户信息在系统中不存在，保存时弹出提示：
   - "客户不存在，是否添加到客户列表？"
   - 用户选择"是"则自动创建客户记录

### 3.2 产品信息

**功能要求：**
1. 支持添加多个产品（不是一个）
2. 每行包含：
   - 产品选择（下拉/搜索）
   - 数量输入
   - 单价输入
   - 小计（自动计算）
3. 可以添加多行
4. 可以删除某行
5. 底部显示总计金额

### 3.3 产品价格优化

**参考价格提示：**
1. 选择产品后，价格输入框显示 placeholder 为参考价格
2. 参考价格来源：`product_prices` 表中 `platform='website'` 且 `is_reference=1` 的记录
3. 显示格式：`placeholder="参考价格: ¥99.00"` 或 `placeholder="参考价格: $14.99"`
4. 如果没有参考价格，placeholder 显示 "请输入报价"

**历史成交价格查看：**
1. 每个产品行添加"查看历史价格"按钮
2. 点击后弹出该产品的历史成交价格列表
3. 数据来源：从 `orders` 和 `order_items` 表查询
   ```sql
   SELECT o.order_code, o.customer_name, oi.unit_price, o.created_at
   FROM order_items oi
   JOIN orders o ON oi.order_id = o.id
   WHERE oi.product_id = ? AND o.status IN (3, 4, 5)  -- 已付款、已发货、已收货
   ORDER BY o.created_at DESC
   LIMIT 10
   ```
4. 显示内容：
   - 订单编号
   - 客户名称
   - 成交单价
   - 成交日期
5. 如果没有历史成交记录，显示"暂无历史成交记录"

### 3.3 其他信息

- 订单日期（默认当天）
- 付款条款（默认：100% before shipment）
- 交货条款（默认：EXW）
- 交货期（默认：3-7 working days）
- 备注

### 3.4 保存

- 保存后状态为"未成交"
- 可以后续编辑（未成交状态）

---

## 四、订单列表需求

### 4.1 显示内容

| 列 | 说明 |
|---|------|
| 订单编号 | 自动生成，格式：ORD-YYYYMMDD-XXXX |
| 客户名称 | - |
| 金额 | 订单总金额 |
| 状态 | 用不同颜色标签 |
| 日期 | 创建日期 |
| 操作 | 操作按钮 |

### 4.2 筛选功能

- 按状态筛选
- 按客户名称搜索
- 按日期范围筛选

### 4.3 操作按钮

根据订单状态显示不同操作：

| 状态 | 可用操作 |
|------|---------|
| 未成交 | 查看、编辑、下载PI、标记价格锁定、取消 |
| 价格锁定 | 查看、下载PI、标记已付款、取消 |
| 已付款 | 查看、下载CI、标记已发货 |
| 已发货 | 查看、下载CI、标记已收货 |
| 已收货 | 查看、下载CI |
| 已取消 | 查看 |

---

## 五、订单详情需求

### 5.1 显示内容

1. **订单基本信息**
   - 订单编号
   - 状态（带颜色标签）
   - 创建时间
   - 最后更新时间

2. **客户信息**
   - 客户名称
   - 邮箱
   - 电话
   - 地址

3. **产品列表**
   - 产品名称
   - 数量
   - 单价
   - 小计

4. **金额汇总**
   - 小计
   - 总计

5. **条款信息**
   - 付款条款
   - 交货条款
   - 交货期
   - 备注

6. **状态历史**（可选）
   - 显示状态变更记录

### 5.2 操作按钮

根据状态显示：
- 编辑订单（未成交状态）
- 下载 PI（未成交、价格锁定状态）
- 下载 CI（已付款及之后状态）
- 状态流转按钮

---

## 六、PI/CI 下载需求

### 6.1 PI 下载

**触发条件：** 订单状态为"未成交"或"价格锁定"

**格式：** Excel 文件

**内容：** 参考 templates/invoice/PI_template.xlsx 模板
- PI 编号（与订单编号相同或单独编号）
- 日期
- 买家信息（客户名称、地址、电话、邮箱）
- 卖家信息（公司名称、地址、电话、邮箱）
- 产品列表（产品名称、型号、数量、单价、小计）
- 总金额
- 条款（付款条款、交货条款、交货期）

### 6.2 CI 下载

**触发条件：** 订单状态为"已付款"及之后

**格式：** Excel 文件

**内容：** 类似 PI，增加付款信息

---

## 七、数据库需求

### 7.1 订单表 (orders)

需要包含的字段：
- id
- order_code（订单编号）
- status（状态：1-6）
- customer_id（可选，关联客户表）
- customer_name
- customer_email
- customer_phone
- customer_address
- total_amount
- payment_terms
- delivery_terms
- lead_time
- notes
- created_at
- updated_at

### 7.2 订单明细表 (order_items)

需要包含的字段：
- id
- order_id
- product_id
- product_name
- quantity
- unit_price
- subtotal

### 7.3 客户地址表 (customer_addresses) - 多地址管理

**已存在表结构：**
```sql
CREATE TABLE customer_addresses (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    customer_id INTEGER NOT NULL REFERENCES customers(id) ON DELETE CASCADE,
    receiver_name TEXT NOT NULL,        -- 收件人
    receiver_phone TEXT NOT NULL,       -- 收件电话
    country TEXT NOT NULL,              -- 国家
    country_code TEXT,                  -- 国家代码
    province TEXT,                      -- 省份
    city TEXT,                          -- 城市
    district TEXT,                      -- 区
    address TEXT NOT NULL,              -- 详细地址
    postal_code TEXT,                   -- 邮编
    is_default INTEGER DEFAULT 0,       -- 是否默认地址（新增）
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);
```

**需要确保：**
- `is_default` 字段存在（如不存在需要添加迁移）
- 每个 customer_id 下只能有一个 is_default = 1 的地址

---

## 八、客户地址管理需求

### 8.1 网页操作入口

**客户详情页 /customers/:id**

在客户详情页添加"地址管理"区域：
1. 显示该客户的所有地址列表
2. 每个地址显示：收件人、电话、完整地址、是否默认
3. 操作按钮：设为默认、编辑、删除
4. 添加"新增地址"按钮

**新增地址弹窗/页面：**
- 收件人（必填）
- 电话（必填）
- 国家（默认 CN）
- 省份
- 城市
- 区
- 详细地址（必填）
- 邮编
- 设为默认地址（复选框）

### 8.2 API 接口

```
GET    /api/v1/customers/:id/addresses      - 获取客户地址列表
POST   /api/v1/customers/:id/addresses      - 新增地址
PUT    /api/v1/customer-addresses/:id       - 更新地址
DELETE /api/v1/customer-addresses/:id       - 删除地址
POST   /api/v1/customer-addresses/:id/set-default - 设为默认地址
```

---

## 九、路由设计

### 9.1 订单相关

```
GET  /orders              - 订单列表页
GET  /orders/new          - 创建订单页
POST /orders              - 创建订单处理
GET  /orders/:id          - 订单详情页
GET  /orders/:id/edit     - 编辑订单页
POST /orders/:id          - 更新订单处理
GET  /orders/:id/download-pi - 下载 PI
GET  /orders/:id/download-ci - 下载 CI
POST /orders/:id/lock-price  - 标记价格锁定
POST /orders/:id/mark-paid   - 标记已付款
POST /orders/:id/ship        - 标记已发货
POST /orders/:id/deliver     - 标记已收货
POST /orders/:id/cancel      - 取消订单
```

### 9.2 客户地址相关

```
GET    /customers/:id             - 客户详情页（包含地址列表）
GET    /customers/:id/addresses   - 获取客户地址列表（API）
POST   /customers/:id/addresses   - 新增地址（API）
PUT    /customer-addresses/:id    - 更新地址（API）
DELETE /customer-addresses/:id    - 删除地址（API）
POST   /customer-addresses/:id/set-default - 设为默认（API）
```

### 9.3 产品价格相关

```
GET /api/v1/products/:id/reference-price - 获取产品参考价格
GET /api/v1/products/:id/history-prices  - 获取产品历史成交价格
```

---

## 十、审核要点

请 researcher 专家审核以下内容：

1. **订单创建页**
   - [ ] 是否支持选择客户？
   - [ ] 选择客户后是否自动填充信息？
   - [ ] 是否支持手动输入客户信息？
   - [ ] 是否支持添加多个产品？
   - [ ] 是否有添加/删除产品功能？
   - [ ] 是否自动计算总计？

2. **订单列表页**
   - [ ] 是否有状态筛选？
   - [ ] 操作按钮是否根据状态显示？
   - [ ] 是否有"创建订单"按钮？

3. **订单详情页**
   - [ ] 是否显示完整信息？
   - [ ] 是否有下载 PI/CI 按钮？
   - [ ] 是否有状态流转按钮？

4. **PI/CI 下载**
   - [ ] 是否能正常下载？
   - [ ] 下载的文件格式是否正确？

5. **状态流转**
   - [ ] 状态流转是否符合设计？

6. **客户自动添加**
   - [ ] 新客户保存时是否提示添加？

---

## 十一、预期结果

订单模块应该实现：
1. 创建订单 → 选择/输入客户 → 添加多个产品 → 保存（未成交状态）
2. 订单列表 → 查看所有订单 → 按状态筛选
3. 订单详情 → 下载 PI → 发给客户确认
4. 客户确认 → 标记价格锁定
5. 客户付款 → 标记已付款 → 下载 CI
6. 发货 → 标记已发货
7. 收货 → 标记已收货 → 订单完成
