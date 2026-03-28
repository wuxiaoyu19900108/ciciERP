-- ciciERP 数据库迁移脚本
-- 版本: 003
-- 日期: 2026-03-21
-- 描述: 产品价格模型重构 - 将产品属性与价格分离

-- ============================================================================
-- 1. product_costs 表添加新字段
-- ============================================================================

-- 添加 quantity 列（如果不存在）
-- SQLite 不支持 IF NOT EXISTS for columns，使用安全的添加方式
ALTER TABLE product_costs ADD COLUMN quantity INTEGER DEFAULT 1;
ALTER TABLE product_costs ADD COLUMN purchase_order_id INTEGER REFERENCES purchase_orders(id);
ALTER TABLE product_costs ADD COLUMN is_reference INTEGER DEFAULT 0;

-- 更新现有记录为参考成本
UPDATE product_costs SET is_reference = 1 WHERE purchase_order_id IS NULL;

-- 为参考成本创建唯一索引（每个产品只能有一条参考成本）
CREATE UNIQUE INDEX IF NOT EXISTS idx_product_costs_reference
ON product_costs(product_id) WHERE is_reference = 1;

-- ============================================================================
-- 2. 创建产品销售价格表 (product_prices)
-- ============================================================================

CREATE TABLE IF NOT EXISTS product_prices (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    product_id INTEGER NOT NULL,
    platform TEXT NOT NULL DEFAULT 'website',  -- website, alibaba, amazon
    sale_price_cny REAL NOT NULL,
    sale_price_usd REAL,
    exchange_rate REAL DEFAULT 7.2,
    profit_margin REAL DEFAULT 0.15,
    platform_fee_rate REAL DEFAULT 0.025,
    platform_fee REAL,
    is_reference INTEGER DEFAULT 0,  -- 是否为参考售价
    effective_date TEXT,
    notes TEXT,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now')),
    FOREIGN KEY (product_id) REFERENCES products(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_product_prices_product ON product_prices(product_id);
CREATE INDEX IF NOT EXISTS idx_product_prices_platform ON product_prices(platform);
CREATE INDEX IF NOT EXISTS idx_product_prices_effective ON product_prices(effective_date);

-- 为参考售价创建唯一索引（每个产品每个平台只能有一条参考售价）
CREATE UNIQUE INDEX IF NOT EXISTS idx_product_prices_reference
ON product_prices(product_id, platform) WHERE is_reference = 1;

-- ============================================================================
-- 3. 迁移现有价格数据到 product_prices 表 (只迁移不存在的)
-- ============================================================================

-- 将 products 表中的 sale_price 迁移到 product_prices 作为 website 平台的参考售价
-- 使用 INSERT OR IGNORE 避免重复
INSERT OR IGNORE INTO product_prices (product_id, platform, sale_price_cny, is_reference, effective_date)
SELECT
    id,
    'website',
    sale_price,
    1,
    date('now')
FROM products
WHERE sale_price > 0 AND deleted_at IS NULL;

-- ============================================================================
-- 4. products 表删除价格字段
-- 注意: SQLite 不支持 DROP COLUMN，我们需要重建表
-- 此步骤已在之前执行，跳过
-- ============================================================================

-- 以下重建表操作已执行，跳过
-- CREATE TABLE products_new (...);
-- INSERT INTO products_new SELECT ... FROM products;
-- DROP TABLE products;
-- ALTER TABLE products_new RENAME TO products;

-- 重建索引
CREATE INDEX IF NOT EXISTS idx_products_code ON products(product_code);
CREATE INDEX IF NOT EXISTS idx_products_category ON products(category_id);
CREATE INDEX IF NOT EXISTS idx_products_brand ON products(brand_id);
CREATE INDEX IF NOT EXISTS idx_products_status ON products(status);

-- ============================================================================
-- 5. 重建全文搜索触发器
-- ============================================================================

-- 删除旧触发器
DROP TRIGGER IF EXISTS products_ai;
DROP TRIGGER IF EXISTS products_ad;
DROP TRIGGER IF EXISTS products_au;

-- 重新创建全文搜索触发器
CREATE TRIGGER IF NOT EXISTS products_ai AFTER INSERT ON products BEGIN
    INSERT INTO products_fts(rowid, name, description)
    VALUES (new.id, new.name, COALESCE(new.description, ''));
END;

CREATE TRIGGER IF NOT EXISTS products_ad AFTER DELETE ON products BEGIN
    INSERT INTO products_fts(products_fts, rowid, name, description)
    VALUES('delete', old.id, old.name, COALESCE(old.description, ''));
END;

CREATE TRIGGER IF NOT EXISTS products_au AFTER UPDATE ON products BEGIN
    INSERT INTO products_fts(products_fts, rowid, name, description)
    VALUES('delete', old.id, old.name, COALESCE(old.description, ''));
    INSERT INTO products_fts(rowid, name, description)
    VALUES (new.id, new.name, COALESCE(new.description, ''));
END;
