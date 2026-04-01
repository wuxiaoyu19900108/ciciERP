-- ciciERP 数据库迁移脚本
-- 版本: 010
-- 日期: 2026-03-31
-- 描述: 添加产品型号字段

-- 添加 model 字段（型号）到 products 表
ALTER TABLE products ADD COLUMN model TEXT;

-- model 字段加索引，方便按型号搜索/过滤
CREATE INDEX IF NOT EXISTS idx_products_model ON products(model);
