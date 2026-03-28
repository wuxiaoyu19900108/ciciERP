-- ciciERP 数据库迁移脚本
-- 版本: 002
-- 日期: 2026-03-20
-- 描述: 添加产品成本表、内容表

-- ============================================================================
-- 1. 创建产品成本表 (product_costs)
-- ============================================================================

CREATE TABLE IF NOT EXISTS product_costs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    product_id INTEGER NOT NULL,
    supplier_id INTEGER,
    cost_cny REAL NOT NULL,
    cost_usd REAL,
    currency TEXT DEFAULT 'CNY',
    exchange_rate REAL DEFAULT 6.81,
    profit_margin REAL DEFAULT 0,
    platform_fee_rate REAL DEFAULT 0.025,
    platform_fee REAL,
    sale_price_usd REAL,
    effective_date TEXT,
    notes TEXT,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now')),
    FOREIGN KEY (product_id) REFERENCES products(id) ON DELETE CASCADE,
    FOREIGN KEY (supplier_id) REFERENCES suppliers(id)
);

CREATE INDEX IF NOT EXISTS idx_product_costs_product ON product_costs(product_id);
CREATE INDEX IF NOT EXISTS idx_product_costs_supplier ON product_costs(supplier_id);
CREATE INDEX IF NOT EXISTS idx_product_costs_effective ON product_costs(effective_date);

-- ============================================================================
-- 2. 创建产品内容表 (product_content)
-- ============================================================================

CREATE TABLE IF NOT EXISTS product_content (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    product_id INTEGER NOT NULL UNIQUE,
    title_en TEXT,
    description TEXT,
    description_en TEXT,
    main_image TEXT,
    images TEXT,
    specifications TEXT,
    meta_title TEXT,
    meta_description TEXT,
    meta_keywords TEXT,
    content_html TEXT,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now')),
    FOREIGN KEY (product_id) REFERENCES products(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_product_content_product ON product_content(product_id);
