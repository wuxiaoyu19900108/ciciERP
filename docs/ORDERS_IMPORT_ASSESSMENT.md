# orders_ali.xlsx 导入评估报告

## 1. 数据概览

### Excel 文件 (orders_ali.xlsx)
| 项目 | 值 |
|------|-----|
| 订单数量 | 69 |
| 列数 | 10 |
| 货币单位 | USD |
| 日期范围 | 2025-08-14 起 |

### ciciERP 数据库
| 表 | 现有数据量 |
|----|-----------|
| orders | 2 |
| order_items | 2 |

---

## 2. 数据结构对比

### Excel 列定义

| # | Excel 列名 | 数据类型 | 非空数 | 示例 |
|---|-----------|---------|-------|------|
| 1 | Date | str | 69 | 2025-08-14 |
| 2 | Order No. | str | 69 | ORD-20250814-0001 |
| 3 | Client Name | str | 69 | Tokpasoua Haba |
| 4 | Product | str | 68 | COMFAST CF-EW85 |
| 5 | Qty | int | 69 | 3 |
| 6 | Order Amount (USD) | float | 69 | 138.0 |
| 7 | Sales Unit Price (USD) | float | 69 | 46.0 |
| 8 | Cost per Unit (USD) | float | 69 | 45.13 |
| 9 | Gross Profit (USD) | float | 69 | 2.61 |
| 10 | Notes | str | 46 | SAMPLE - delete |

### ciciERP orders 表字段 (35个)

| 字段名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| id | INTEGER | PK | 主键 |
| order_code | TEXT | ✓ | 订单编号 |
| platform | TEXT | ✓ | 来源平台 |
| platform_order_id | TEXT | | 平台订单ID |
| customer_id | INTEGER | | 客户ID (外键) |
| customer_name | TEXT | | 客户名称 |
| customer_mobile | TEXT | | 客户电话 |
| customer_email | TEXT | | 客户邮箱 |
| order_type | INTEGER | | 订单类型 |
| order_status | INTEGER | | 订单状态 |
| payment_status | INTEGER | | 支付状态 |
| fulfillment_status | INTEGER | | 履约状态 |
| total_amount | REAL | ✓ | 总金额 |
| subtotal | REAL | ✓ | 小计 |
| discount_amount | REAL | | 折扣金额 |
| shipping_fee | REAL | | 运费 |
| tax_amount | REAL | | 税费 |
| paid_amount | REAL | | 已付金额 |
| refund_amount | REAL | | 退款金额 |
| currency | TEXT | | 货币 |
| exchange_rate | REAL | | 汇率 |
| ... | ... | ... | (其他字段省略) |

### ciciERP order_items 表字段 (20个)

| 字段名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| id | INTEGER | PK | 主键 |
| order_id | INTEGER | ✓ | 订单ID (外键) |
| product_id | INTEGER | | 产品ID (外键) |
| sku_id | INTEGER | | SKU ID (外键) |
| product_name | TEXT | ✓ | 产品名称 |
| product_code | TEXT | | 产品编码 |
| sku_code | TEXT | | SKU编码 |
| quantity | INTEGER | ✓ | 数量 |
| unit_price | REAL | ✓ | 单价 |
| subtotal | REAL | ✓ | 小计 |
| cost_price | REAL | | 成本价 |
| total_amount | REAL | ✓ | 总金额 |
| ... | ... | ... | (其他字段省略) |

---

## 3. 字段映射关系

### 可直接映射

| Excel 字段 | ciciERP 字段 | 映射说明 |
|-----------|-------------|---------|
| Order No. | orders.order_code | 直接映射 |
| Client Name | orders.customer_name | 直接映射 |
| Date | orders.created_at | 需转换格式 |
| Notes | orders.customer_note 或 internal_note | 备注信息 |
| Order Amount (USD) | orders.total_amount | 需货币转换 |
| Product | order_items.product_name | 直接映射 |
| Qty | order_items.quantity | 直接映射 |
| Sales Unit Price (USD) | order_items.unit_price | 需货币转换 |
| Cost per Unit (USD) | order_items.cost_price | 需货币转换 |
| Gross Profit (USD) | 计算字段 | 可存储或计算 |

### ciciERP 缺失的 Excel 数据

| Excel 字段 | 状态 | 处理方案 |
|-----------|------|---------|
| Gross Profit (USD) | ❌ 无直接字段 | 可忽略(可计算) 或存入 internal_note |

### Excel 缺失的 ciciERP 字段

| ciciERP 字段 | 必填 | 默认值建议 |
|-------------|------|-----------|
| platform | ✓ | 'ali_import' |
| currency | | 'CNY' (需转换) |
| order_status | | 1 (待处理) |
| payment_status | | 1 (已支付) |
| fulfillment_status | | 1 (已完成) |
| order_type | | 1 (普通订单) |
| subtotal | ✓ | 同 total_amount |
| customer_id | | NULL |
| customer_mobile | | NULL |
| customer_email | | NULL |
| discount_amount | | 0 |
| shipping_fee | | 0 |
| tax_amount | | 0 |
| paid_amount | | 同 total_amount |

---

## 4. 数据结构差异分析

### 关键差异

| 差异点 | Excel | ciciERP | 影响 |
|--------|-------|---------|------|
| 数据结构 | 扁平结构 (订单+产品同行) | 规范化结构 (订单/订单项分表) | 需拆分导入 |
| 货币 | USD | CNY | 需汇率转换 |
| 客户关联 | 仅名称 | customer_id 外键 | 客户需先导入或创建 |
| 产品关联 | 仅名称 | product_id 外键 | 产品需先匹配或创建 |
| 订单项 | 每行一个产品 | order_items 表 | 需合并同订单产品 |

### 订单合并问题

Excel 中同一订单号可能有多行（多个产品），需要：
1. 按 Order No. 分组
2. 每个订单号创建 1 条 orders 记录
3. 每个产品创建 1 条 order_items 记录

---

## 5. 导入可行性评估

### 结论：✅ 可以导入

### 需要的前置条件

| 序号 | 条件 | 状态 | 说明 |
|------|------|------|------|
| 1 | 汇率配置 | ⚠️ 需配置 | USD→CNY 汇率 |
| 2 | 客户数据 | ⚠️ 可选 | 可用 customer_name 替代 |
| 3 | 产品数据 | ⚠️ 可选 | 可用 product_name 替代 |

### 数据转换需求

```
1. 日期格式转换
   Excel: "2025-08-14"
   ciciERP: "2025-08-14T00:00:00+00:00"

2. 货币转换
   USD → CNY (需要当日汇率或固定汇率)

3. 订单拆分
   按 Order No. 分组，创建 orders + order_items

4. 订单编号检查
   检查 Order No. 是否与现有订单冲突
```

### 潜在问题

| 问题 | 风险等级 | 解决方案 |
|------|---------|---------|
| 订单编号冲突 | 低 | 检查后添加前缀或跳过 |
| 客户名称重复 | 中 | 按 customer_name 匹配，不匹配则创建 |
| 产品名称不匹配 | 中 | 按 product_name 匹配，不匹配则创建占位 |
| 汇率准确性 | 低 | 使用导入日汇率或历史汇率 |
| Product 列有空值 | 低 | 1条记录 Product 为空，需处理 |

---

## 6. 导入方案建议

### 方案 A：直接导入（推荐）

```python
# 导入流程
1. 读取 Excel 数据
2. 按 Order No. 分组
3. 对每个订单组:
   a. 创建 orders 记录
      - order_code = Order No.
      - customer_name = Client Name
      - platform = 'ali_import'
      - total_amount = sum(Order Amount) * exchange_rate
      - currency = 'CNY'
   b. 为每个产品创建 order_items 记录
      - product_name = Product
      - quantity = Qty
      - unit_price = Sales Unit Price * exchange_rate
      - cost_price = Cost per Unit * exchange_rate
      - total_amount = unit_price * quantity
```

### 方案 B：先导入基础数据

```python
# 先处理客户和产品
1. 提取所有 Client Name → 创建 customers
2. 提取所有 Product → 创建 products
3. 导入订单时关联 ID
```

---

## 7. 导入脚本要点

```python
# 关键代码逻辑
import pandas as pd
import sqlite3

# 汇率配置
EXCHANGE_RATE = 7.2  # USD to CNY

# 读取 Excel
df = pd.read_excel('orders_ali.xlsx')

# 按订单号分组
orders = df.groupby('Order No.')

for order_no, items in orders:
    # 创建订单
    order_data = {
        'order_code': order_no,
        'customer_name': items['Client Name'].iloc[0],
        'platform': 'ali_import',
        'currency': 'CNY',
        'total_amount': items['Order Amount (USD)'].sum() * EXCHANGE_RATE,
        'subtotal': items['Order Amount (USD)'].sum() * EXCHANGE_RATE,
        # ...
    }

    # 创建订单项
    for _, item in items.iterrows():
        item_data = {
            'product_name': item['Product'],
            'quantity': item['Qty'],
            'unit_price': item['Sales Unit Price (USD)'] * EXCHANGE_RATE,
            'cost_price': item['Cost per Unit (USD)'] * EXCHANGE_RATE,
            # ...
        }
```

---

## 8. 总结

| 评估项 | 结果 |
|--------|------|
| **可导入性** | ✅ 可以导入 |
| **数据完整性** | ⚠️ 基本完整，需补充默认值 |
| **转换复杂度** | 中等 (需拆分+货币转换) |
| **预计工作量** | 1-2小时开发脚本 |
| **风险等级** | 低 |

### 下一步建议

1. 确认汇率使用方式（固定汇率 vs 历史汇率）
2. 确认客户/产品是否需要先导入
3. 编写并测试导入脚本
4. 先导入 1-2 条测试数据验证
5. 批量导入剩余数据
