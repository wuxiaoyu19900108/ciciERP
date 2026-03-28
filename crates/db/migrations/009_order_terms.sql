-- 订单条款字段
-- 迁移版本: 009
-- 创建时间: 2026-03-25

-- 添加付款条款字段
ALTER TABLE orders ADD COLUMN payment_terms TEXT;

-- 添加交货条款字段
ALTER TABLE orders ADD COLUMN delivery_terms TEXT;

-- 添加交货期字段
ALTER TABLE orders ADD COLUMN lead_time TEXT;
