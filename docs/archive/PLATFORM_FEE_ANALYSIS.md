# 平台费导入问题分析报告

## 检查概述

**检查日期**: 2026-03-29
**检查范围**: 产品平台费（Alibaba Fee）导入数据
**数据源**:
- Excel: `/home/wxy/.xiaozhi/files/file_1773984868713_a5025b1a_Foreign_Trade_Management_Template.xlsx.xlsx`
- Sheet: `2026 Product Cost List`
- 数据库: `data/cicierp.db`

---

## 检查结果摘要

| 检查项 | 结果 | 状态 |
|--------|------|------|
| Excel 产品总数 | 136 条 | - |
| 数据库产品总数 | 135 条 | - |
| 平台费数据一致性 | 135/135 条匹配 | ✅ |
| 币种一致性 | 全部为 USD | ✅ |
| 汇率应用 | 6.81 | ✅ |
| product_prices.platform_fee 空值 | 16 条 | ⚠️ |

---

## 详细分析

### 1. 币种确认

| 项目 | 值 |
|------|-----|
| Excel 列名 | `Alibaba Fee (USD)` |
| 币种 | **USD** (美元) |
| 数据库存储币种 | **USD** (美元) |
| 汇率 | 6.81 |

**结论**: 币种一致，无币种错误。

### 2. 计算公式验证

**Excel 计算公式**:
```
Alibaba Fee (USD) = Selling Price (USD) × 0.025
```

**数据库计算公式**:
```
platform_fee = sale_price_usd × 0.025
```

**验证结果**: 135/135 条产品公式计算一致 ✅

### 3. product_costs.platform_fee 对比

| 状态 | 数量 |
|------|------|
| 匹配 | 135 条 |
| 不匹配 | 0 条 |

**结论**: `product_costs` 表中的平台费数据完全正确。

### 4. product_prices.platform_fee 对比

| 状态 | 数量 |
|------|------|
| 匹配 | 119 条 |
| 为空 (NULL) | 16 条 |
| 不匹配 | 0 条 |

---

## 问题清单

### 问题 1: product_prices.platform_fee 部分数据为空

**严重程度**: ⚠️ 中等

**影响范围**: 16 条产品

**问题描述**:
以下产品的 `product_prices.platform_fee` 字段为空，但 `product_costs.platform_fee` 中有正确的值。

| product_id | product_code | 产品名称 | cost_platform_fee | sale_price_usd | 应填入的 price_platform_fee |
|------------|--------------|----------|-------------------|----------------|---------------------------|
| 3 | SP-01 | COMFAST CF-EW74 | 0.844 | 33.744 | 0.844 |
| 4 | SP-02 | 27显示器 | 1.702 | 68.082 | 1.702 |
| 5 | SP-03 | COMFAST CF-EW72 | 0.881 | 35.250 | 0.881 |
| 6 | SP-04 | Sonoff D1调光器 - 无遥控器 - 220V | 0.422 | 16.876 | 0.422 |
| 7 | SP-05 | Sonoff风扇-无遥控-220V | 0.482 | 19.285 | 0.482 |
| 8 | SP-06 | R02 | 0.414 | 16.574 | 0.414 |
| 9 | SP-07 | COMFAST CF-E312A | 0.806 | 32.238 | 0.806 |
| 10 | SP-08 | COMFAST CF-E319A | 1.258 | 50.311 | 1.258 |
| 11 | SP-09 | COMFAST CF-E314N V2 | 0.712 | 28.473 | 0.712 |
| 12 | SP-11 | 87背光键盘黑色 | 0.241 | 9.646 | 0.241 |
| 13 | SP-12 | 87背光键盘白色 | 0.241 | 9.646 | 0.241 |
| 14 | SP-13 | 87背光键盘黑灰 | 0.249 | 9.948 | 0.249 |
| 15 | SP-14 | COMFAST CF-E314N V2 | 0.661 | 26.422 | 0.661 |
| 16 | SP-15 | COMFAST CF-E320N V2 | 0.637 | 25.461 | 0.637 |
| 17 | SP-16 | 稞米Mavit 3T 定制款 | 37.780 | 1511.209 | 37.780 |
| 18 | SP-17 | SNZB-05P Leak Detector | 0.316 | 12.658 | 0.316 |

**原因分析**:
可能是导入脚本在创建 `product_prices` 记录时，没有同时填充 `platform_fee` 字段。

**建议修复方案**:
```sql
UPDATE product_prices
SET platform_fee = (
    SELECT pc.platform_fee
    FROM product_costs pc
    WHERE pc.product_id = product_prices.product_id
)
WHERE platform_fee IS NULL;
```

---

### 问题 2: Excel 中有 1 条产品未导入数据库

**严重程度**: ⚠️ 低

**影响范围**: 1 条产品

**问题描述**:
| Model | Product Name | Alibaba Fee (USD) | 状态 |
|-------|--------------|-------------------|------|
| SP-136 | moes WRS-EUFL-BK | NaN | 未导入 |

**原因分析**:
该产品在 Excel 中的 `Alibaba Fee (USD)` 字段为空（NaN），可能是数据不完整导致导入时被跳过。

---

## 总结

### 数据质量评估

| 评估项 | 评分 | 说明 |
|--------|------|------|
| 数据完整性 | 4/5 | 有 16 条 product_prices.platform_fee 为空 |
| 数据准确性 | 5/5 | 已导入数据全部正确 |
| 币种一致性 | 5/5 | 全部为 USD |
| 汇率正确性 | 5/5 | 6.81 |

### 需要修复的问题

1. **高优先级**: 修复 16 条 `product_prices.platform_fee` 为空的记录
2. **低优先级**: 确认 SP-136 产品是否需要补充导入

---

## 修复脚本

```python
#!/usr/bin/env python3
"""
修复 product_prices.platform_fee 为空的记录
"""
import sqlite3

conn = sqlite3.connect('/home/wxy/data/ciciERP/data/cicierp.db')
cursor = conn.cursor()

# 更新 product_prices 中 platform_fee 为空的记录
cursor.execute('''
    UPDATE product_prices
    SET platform_fee = (
        SELECT pc.platform_fee
        FROM product_costs pc
        WHERE pc.product_id = product_prices.product_id
    )
    WHERE platform_fee IS NULL
''')

affected_rows = cursor.rowcount
conn.commit()
conn.close()

print(f'已修复 {affected_rows} 条记录')
```
