# ChatGPT Plus 重构建议 — 评估报告

> 生成时间：2026-04-02  
> 评估人：Copilot（基于当前 ciciERP 代码库实际状态）

---

## 一、原始建议原文

以下为 ChatGPT Plus 提出的完整重构建议（原文保留）：

---

### 核心目标
1. 打通 产品 / 采购 / 订单 / 库存 数据链路
2. 确保利润计算100%准确
3. 建立完整采购流程
4. 增加客户跟进系统
5. 提升系统稳定性和可扩展性

---

### 数据结构重构

**1. products 表**
```
id, product_name, model(唯一), cost_rmb, cost_usd, price_rmb, price_usd,
supplier_id, stock_qty, track_stock, exchange_rate, created_at, updated_at
```
要求：model 作为唯一标识，所有价格从此表读取

**2. orders 表**
```
id, order_no, platform(AE/ALI), customer_name, product_id, qty,
unit_price, order_amount, cost_snapshot, price_snapshot, exchange_rate_snapshot,
shipping_fee, platform_fee, other_fee, gross_profit, net_profit, profit_margin, created_at
```
要求：下单时写入快照，历史数据不受产品价格变更影响

**3. purchase_orders 表**
```
id, purchase_no, supplier_id, product_id, qty, unit_cost, total_cost,
status, expected_arrival_date, tracking_no, remark, created_at
```

**4. customers 表**
```
id, name, country, contact, status, last_contact_date, next_followup_date, notes, created_at
```

**5. logs 表**
```
id, user, action, target_table, target_id, old_value, new_value, created_at
```

---

### 核心业务逻辑

**利润计算**
```
gross_profit   = order_amount - (cost_snapshot × qty)
net_profit     = order_amount - (cost_snapshot × qty) - shipping_fee - platform_fee - other_fee
profit_margin  = net_profit / order_amount
```

**币种规则**
- AliExpress → RMB
- Alibaba → USD
- 汇率只从 products.exchange_rate 读取，禁止多处手动输入

**库存联动**
- 采购单 status = "stocked" → stock_qty += qty
- 订单创建 → stock_qty -= qty
- 支持部分/多次入库

**采购状态**
```
pending → ordered → paid → shipped → arrived → stocked → completed
```

**客户跟进状态**
```
Inquiry → Quoted → Sample → Negotiating → Order / Lost
```
规则：next_followup_date <= today → 标记"需跟进"

---

### 功能模块优化

1. Dashboard：今日待跟进客户、本月销售额/利润/订单数、高利润产品 TOP5、低库存预警
2. 列表页统一：搜索 + 筛选 + 排序 + 分页
3. 导入导出：Excel 导入（products/orders/customers）+ Excel/CSV 导出
4. 操作安全：删除二次确认 + 所有修改写 logs 表
5. 统一接口格式：`{ "success": true/false, "message": "", "data": {} }`

---

### 优化顺序

Step 1：重构数据结构 + 利润逻辑  
Step 2：重构采购模块 + 库存联动  
Step 3：实现客户跟进系统 + Dashboard  
Step 4：增加导入导出 + 日志 + 性能优化  

---

## 二、当前系统实际状态（对比评估基础）

| 维度 | 当前状态 |
|------|---------|
| 数据库表 | 35 张表，完整关系型设计 |
| 订单数据 | 169 条（AE + ALI），已通过 Excel 对账验证 |
| 产品数据 | 175 条，含成本/价格/平台费率分离存储 |
| 客户数据 | 178 条，含跟进状态字段 |
| 采购单 | 3 条，完整采购审批流 |
| 技术栈 | Rust / Axum / SQLite / Askama + 内联 HTML |
| 代码量 | web.rs ≈ 7000 行，完整 CRUD |

---

## 三、逐条评估

### 3.1 数据结构重构

| 建议 | 评估结论 | 说明 |
|------|---------|------|
| products 扁平化（cost_rmb/usd/price_rmb/usd 直接在 products 表） | ❌ **不适合** | 当前设计将成本存 `product_costs`、售价存 `product_prices`，支持多条历史记录、多平台价格、采购单关联。ChatGPT 的扁平化设计会丢失历史数据和多平台能力 |
| model 作为唯一标识 | ⚠️ **部分适合** | 当前用 `product_code`（系统自动生成）作为唯一编码，`model` 字段存在但无唯一约束。可以加唯一索引，但不需要把 model 改成主要业务键 |
| orders 增加 cost_snapshot / exchange_rate_snapshot | ✅ **适合** | 当前 `order_items.cost_price` 已存快照，但缺少 `platform_fee` 和 `other_fee` 明细字段。这是真实缺口，值得补充 |
| orders 增加 gross_profit / net_profit / profit_margin 计算字段 | ✅ **适合** | 当前无这些字段，利润需实时计算。作为存储字段可提升查询性能和历史一致性 |
| purchase_orders 简化（单品单行） | ❌ **不适合** | 当前 `purchase_orders` + `purchase_order_items` 支持一单多品，与实际业务更符合 |
| customers 增加 next_followup_date | ✅ **已存在** | `customers` 表已有 `lead_status` 字段，但缺少 `next_followup_date`，值得补充 |
| logs 表 | ✅ **适合** | 当前无操作日志表，这是真实缺口 |

**结论：不建议推倒重来，建议在现有结构上补充缺失字段。**

---

### 3.2 利润计算逻辑

| 建议 | 评估结论 | 说明 |
|------|---------|------|
| gross_profit = order_amount - cost_snapshot × qty | ✅ **正确思路** | 当前计算分散在各处，没有统一存储，需要整理 |
| net_profit 扣除 shipping_fee + platform_fee + other_fee | ✅ **适合** | `orders` 表已有 `shipping_fee`，但缺少 `platform_fee` 单独字段（当前通过 `product_prices.platform_fee_rate` 存储费率，未在订单层面展开为实际金额） |
| 币种规则（AE=RMB，ALI=USD） | ✅ **已实现** | 当前数据已按此规则区分，通过 `orders.currency` 字段区分 |
| 汇率统一来源 | ⚠️ **部分适合** | 当前有 `exchange_rates` 独立表，但订单也可手动输入汇率。可以收紧为强制引用 `exchange_rates` 表 |

---

### 3.3 采购模块

| 建议 | 评估结论 | 说明 |
|------|---------|------|
| 采购状态流转（7步） | ⚠️ **部分重叠** | 当前 `purchase_orders.status` 有 5 个状态（待审/已审/执行中/已完成/已取消），加上 `delivery_status`（3个）和 `payment_status`（3个）分开存储，实际能力更强 |
| 采购入库自动更新库存 | ✅ **真实缺口** | 当前采购入库和库存 `stock_movements` 表存在，但触发逻辑是否完整需验证 |
| 部分/多次入库 | ✅ **架构支持** | `purchase_order_items` 有 `received_qty` 字段，架构已支持，需验证前端流程 |

---

### 3.4 客户跟进系统

| 建议 | 评估结论 | 说明 |
|------|---------|------|
| 客户状态（6个阶段） | ✅ **已部分实现** | `customers.lead_status` 已存在，但前端展示不完整 |
| next_followup_date 字段 | ✅ **需要新增** | 当前无此字段，Dashboard 展示"今日待跟进"需要它 |
| 跟进记录历史 | ⚠️ **未实现** | 当前无跟进记录表，每次跟进后只有状态更新，无记录追踪 |

---

### 3.5 功能模块优化

| 建议 | 评估结论 | 说明 |
|------|---------|------|
| Dashboard 数据指标 | ✅ **部分实现** | 当前有 Dashboard，但"今日待跟进客户"缺失，利润统计不完整 |
| 列表页搜索/筛选/排序/分页 | ✅ **已实现** | 所有主要列表均已支持，订单/产品搜索已在本会话中完善 |
| Excel 导入 | ⚠️ **部分实现** | 有 AE 订单导入脚本（Python），但无 Web UI 入口；products/customers 无导入 |
| Excel/CSV 导出 | ❌ **未实现** | 当前无导出功能 |
| 删除二次确认 | ✅ **已实现** | 部分模块有确认弹窗 |
| 统一接口响应格式 | ✅ **已实现** | `ApiResponse<T>` 已统一，返回 `{ code, message, data, timestamp }` |
| silent error 禁止 | ⚠️ **待改进** | 部分处理器用 `let _ = ...` 忽略错误 |

---

## 四、综合评估

### 总体结论

**ChatGPT 的建议整体方向正确，但基于简化假设设计，与 ciciERP 当前已达到的成熟度不匹配。**

具体来说：

| 维度 | ChatGPT 建议水平 | ciciERP 当前水平 |
|------|----------------|-----------------|
| 数据模型 | 初级扁平化（≈ 单表) | 中级范式化（分离成本/价格/多平台） |
| 采购模块 | 基础单品采购 | 多品采购单 + 审批流 + 部分入库 |
| 订单模块 | 单品订单 | 多品订单 + 多平台 + 发票关联 |
| 客户模块 | 基础字段 | 完整 CRM 字段（已有） |
| 接口规范 | 基础统一 | 已通过 `ApiResponse<T>` 统一 |

### 不应该做的事

1. **不要推倒 products/orders/purchase_orders 表重建** — 会丢失 169 条真实订单数据和业务逻辑
2. **不要把 product_costs/product_prices 合并回 products 表** — 这是现有系统的核心设计优势（多平台、历史记录）
3. **不要把 purchase_order_items 合并为单品** — 实际业务一次采购多种产品

---

## 五、真正值得做的改进（优先级排序）

### 🔴 高优先级（真实缺口）

| 编号 | 改进项 | 工作量 |
|------|--------|--------|
| G1 | `order_items` 增加 `platform_fee`、`gross_profit`、`net_profit` 存储字段，下单时写入快照 | 中 |
| G2 | `customers` 增加 `next_followup_date` 字段，Dashboard 展示今日待跟进 | 小 |
| G3 | 新增 `operation_logs` 表，记录关键操作（修改产品成本/价格、创建/修改订单） | 中 |
| G4 | Excel 导出（订单、产品列表） | 中 |

### 🟡 中优先级

| 编号 | 改进项 | 工作量 |
|------|--------|--------|
| M1 | 采购入库 → 自动触发 `stock_movements` + 更新 `inventory` 验证完整性 | 小 |
| M2 | 订单创建 → 自动扣减库存（需要 SKU 映射） | 中 |
| M3 | 汇率来源统一（订单中强制引用 `exchange_rates` 表而非手动输入） | 小 |
| M4 | `products.model` 加唯一索引 | 小 |

### 🟢 低优先级

| 编号 | 改进项 | 工作量 |
|------|--------|--------|
| L1 | 客户跟进记录表（记录每次联系历史） | 中 |
| L2 | Web 端 Excel 导入入口 | 大 |
| L3 | silent error 全面清理 | 小 |

---

## 六、建议执行计划

如果要按 ChatGPT 的"Step 1-4"框架实施，建议调整为：

```
Step 1（已完成 ✅）：数据正确性 — 订单数据审计 + 搜索优化 + 平台费率
Step 2（建议下一步）：利润数字化 — G1（订单利润字段），让 Dashboard 能展示准确利润
Step 3：客户跟进闭环 — G2（next_followup_date）+ Dashboard 今日待跟进
Step 4：可追溯性 — G3（操作日志）+ G4（Excel 导出）
Step 5：库存联动完整验证 — M1 + M2
```

---

*文档路径：`docs/CHATGPT_SUGGESTIONS_EVALUATION.md`*
