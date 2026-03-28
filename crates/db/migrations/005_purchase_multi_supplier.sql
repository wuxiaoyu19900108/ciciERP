-- 采购单多供应商支持
-- 版本: 005
-- 日期: 2026-03-23
-- 说明: 将采购单从单供应商模式改为一单多供应商模式

-- ============================================================================
-- 1. 修改 purchase_order_items 表，添加供应商字段
-- ============================================================================

-- 添加供应商ID字段（允许为空，兼容历史数据）
ALTER TABLE purchase_order_items ADD COLUMN supplier_id INTEGER REFERENCES suppliers(id);

-- 添加供应商名称字段（冗余存储便于显示）
ALTER TABLE purchase_order_items ADD COLUMN supplier_name TEXT;

-- ============================================================================
-- 2. 修改 purchase_orders 表，移除单供应商约束
-- ============================================================================

-- 注意：SQLite 不支持 DROP COLUMN，所以我们保留原字段但不再使用
-- supplier_id 和 supplier_name 字段保留用于向后兼容，但新数据将使用明细中的供应商

-- ============================================================================
-- 3. 更新状态定义
-- ============================================================================

-- 状态说明：
-- 1: 草稿 - 新创建，可编辑
-- 2: 待审核 - 提交审核
-- 3: 已审核/执行中 - 审批通过，等待入库
-- 4: 部分入库 - 部分明细已入库
-- 5: 已完成 - 所有明细已入库
-- 6: 已取消 - 取消采购

-- ============================================================================
-- 4. 创建索引优化查询
-- ============================================================================

-- 按供应商查询采购明细
CREATE INDEX IF NOT EXISTS idx_purchase_items_supplier ON purchase_order_items(supplier_id);

-- ============================================================================
-- 5. 迁移历史数据
-- ============================================================================

-- 将主表的供应商信息复制到明细表（仅更新空的供应商字段）
UPDATE purchase_order_items
SET supplier_id = (SELECT supplier_id FROM purchase_orders WHERE purchase_orders.id = purchase_order_items.order_id),
    supplier_name = (SELECT supplier_name FROM purchase_orders WHERE purchase_orders.id = purchase_order_items.order_id)
WHERE supplier_id IS NULL
  AND EXISTS (SELECT 1 FROM purchase_orders WHERE purchase_orders.id = purchase_order_items.order_id AND purchase_orders.supplier_id IS NOT NULL);
