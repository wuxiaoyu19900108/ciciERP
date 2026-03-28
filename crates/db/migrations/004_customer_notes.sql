-- ciciERP 数据库迁移脚本
-- 版本: 004
-- 日期: 2026-03-23
-- 描述: 为 customers 表添加 notes 字段

-- 添加 notes 字段到 customers 表
ALTER TABLE customers ADD COLUMN notes TEXT;
