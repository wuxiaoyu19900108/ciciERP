# orders_ae.xlsx 导入分析报告

## 概述

本报告分析 orders_ae.xlsx 文件与 ciciERP 数据库的匹配情况，确定哪些数据可以直接录入，哪些需要新增。

---

## 一、Excel 文件数据摘要

### 1. 基本信息
- **文件位置**: `/home/wxy/.xiaozhi/files/file_1774829048324_a84af3a2_orders_ae.xlsx.xlsx`
- **总记录数**: 73 行
- **订单数**: 72 个
- **日期范围**: 2025-11-03 ~ 2026-01-16
- **总金额**: 20,395.23 RMB

### 2. 产品统计

| 产品名称 | 总数量 | 订单数 |
|---------|--------|--------|
| Zigbee 3.0UsB Dongle ETH-52P | 75 | 61 |
| SONOFF Basic | 9 | 6 |
| LOONA003 | 4 | 3 |
| MOES Plug-in Display with Humidity Temperature Sensor | 1 | 1 |
| SONOFF Slampher | 1 | 1 |
| ZBM-ZBT-1 Dongle | 1 | 1 |

**产品总数**: 6 种

### 3. 客户统计

- **客户总数**: 68 个
- **主要客户** (按金额排序):
  - 毕加索吴: 4,800.00 RMB (1单)
  - 钟珊-1: 2,705.00 RMB (5单)
  - Janey: 2,650.00 RMB (1单)
  - Труфанов Владимир Васильевич: 1,136.00 RMB (1单)

### 4. 订单金额分布

| 指标 | 值 |
|-----|-----|
| 最小金额 | 18.39 RMB |
| 最大金额 | 4,800.00 RMB |
| 平均金额 | 279.39 RMB |
| 总金额 | 20,395.23 RMB |

---

## 二、数据库现有数据

### 1. 产品
- **数据库产品数**: 140 种
- **产品编码格式**: SP-01 ~ SP-140

### 2. 客户
- **数据库客户数**: 109 个
- **客户编码格式**: C20260302075153 或 CUS-20260329-XXXX

### 3. 订单
- **数据库订单数**: 62 个
- **订单编码格式**: ORD-YYYYMMDD-XXXX 或 ORDYYYYMMDDHHMMSS

---

## 三、匹配分析

### 1. 产品匹配结果

| Excel 产品 | 匹配状态 | 数据库产品 | 数据库 ID |
|-----------|---------|-----------|----------|
| Zigbee 3.0UsB Dongle ETH-52P | ✅ 匹配 | Zigbee 3.0UsB Dongle ETH-52P | 83 |
| MOES Plug-in Display with Humidity Temperature Sensor | ✅ 匹配 | MOES Plug-in Display with Humidity Temperature Sensor | 106 |
| ZBM-ZBT-1 Dongle | ✅ 匹配 | ZBM-ZBT-1 Dongle | 113 |
| LOONA003 | ✅ 匹配 | LOONA003 | 85 |
| SONOFF Slampher | ❌ 需新增 | - | - |
| SONOFF Basic | ❌ 需新增 | - | - |

**匹配统计**:
- 已匹配: 4 种 (66.7%)
- 需新增: 2 种 (33.3%)

### 2. 客户匹配结果

| 状态 | 数量 | 百分比 |
|-----|------|--------|
| ✅ 已匹配 | 1 | 1.5% |
| ❌ 需新增 | 67 | 98.5% |

**已匹配客户**:
- 钟珊-1 → ID: 103

**需新增客户** (67 个):
- 大部分为俄语/乌克兰语客户名
- 部分英语客户名
- 需要创建客户编码

### 3. 订单匹配结果

| 状态 | 数量 |
|-----|------|
| 需要导入 | 72 个 |
| 订单号格式 | ORD-YYYYMMDD-XXXX |

**注意**: 数据库中已有部分相同格式的订单号，需要检查是否已导入过部分数据。

---

## 四、导入建议

### 1. 产品处理

**需要新增的产品** (2 种):

| 产品名称 | 建议编码 | 备注 |
|---------|---------|------|
| SONOFF Slampher | SP-141 | 智能灯座 |
| SONOFF Basic | SP-142 | 智能开关 |

**新增产品 SQL**:
```sql
INSERT INTO products (product_code, name, status, created_at, updated_at)
VALUES
('SP-141', 'SONOFF Slampher', 1, datetime('now'), datetime('now')),
('SP-142', 'SONOFF Basic', 1, datetime('now'), datetime('now'));
```

### 2. 客户处理

**需要新增 67 个客户**

建议按以下规则生成客户编码:
- 格式: `CUS-YYYYMMDD-XXXX`
- 示例: `CUS-20260330-0001`

**部分客户名单**:
1. 毕加索吴
2. Janey
3. Труфанов Владимир Васильевич
4. Залунин Михаил Владимирович
5. Alexander Murashko
6. ... (共 67 个)

### 3. 订单处理

**需要导入 72 个订单**

**导入流程**:
1. 先新增 2 个产品
2. 新增 67 个客户
3. 导入 72 个订单记录
4. 导入订单明细 (order_items)

### 4. 数据映射表

**订单字段映射**:

| Excel 字段 | 数据库字段 | 说明 |
|-----------|-----------|------|
| Order No. | order_code | 订单号 |
| Date | created_at | 订单日期 |
| Client Name | customer_name | 客户名 |
| Product | - | 关联 order_items |
| Qty | - | 关联 order_items |
| Order Amount (RMB) | total_amount | 订单金额 |
| Sales Unit Price (RMB) | - | 关联 order_items |
| Cost per Unit (RMB) | - | 关联 order_items |
| Gross Profit (RMB) | - | 计算字段 |
| Loss Flag | internal_note | 亏损标记 |
| Shipping Status | fulfillment_status | 发货状态 |
| Notes | internal_note | 备注 |

---

## 五、导入脚本建议

### 优先级

1. **高优先级**: 新增产品 (2个)
2. **高优先级**: 新增客户 (67个)
3. **中优先级**: 导入订单 (72个)
4. **低优先级**: 导入订单明细

### 风险点

1. **客户名重复**: 部分客户可能只是名字拼写差异
2. **订单号冲突**: 需要检查是否有订单号已存在
3. **产品关联**: 新订单需要正确关联产品和客户 ID

### 建议导入方式

推荐使用 Python 脚本进行批量导入，包含以下功能:
- 自动检测重复
- 生成唯一编码
- 事务处理
- 错误回滚

---

## 六、总结

| 类别 | Excel 数量 | 已匹配 | 需新增 | 匹配率 |
|-----|-----------|--------|--------|--------|
| 产品 | 6 | 4 | 2 | 66.7% |
| 客户 | 68 | 1 | 67 | 1.5% |
| 订单 | 72 | 0 | 72 | 0% |

**结论**: 大部分数据需要新增导入，尤其是客户数据。建议先处理产品新增，再处理客户新增，最后导入订单数据。

---

*报告生成时间: 2026-03-30*
*数据来源: orders_ae.xlsx, cicierp.db*
