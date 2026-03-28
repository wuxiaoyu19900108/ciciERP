-- PI → 订单 → CI 流程相关表
-- 迁移版本: 008
-- 创建时间: 2026-03-25

-- ============================================================================
-- 1. PI 表 (形式发票)
-- ============================================================================
CREATE TABLE IF NOT EXISTS proforma_invoices (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pi_code TEXT UNIQUE NOT NULL,              -- PI-YYYYMMDD-XXXX
    customer_id INTEGER REFERENCES customers(id),
    customer_name TEXT NOT NULL,
    customer_email TEXT,
    customer_phone TEXT,
    customer_address TEXT,

    -- 卖家信息
    seller_name TEXT NOT NULL DEFAULT 'Shenzhen Westway Technology Co., Ltd',
    seller_address TEXT,
    seller_phone TEXT,
    seller_email TEXT,

    -- 金额
    currency TEXT DEFAULT 'USD',
    subtotal REAL DEFAULT 0,
    discount REAL DEFAULT 0,
    total_amount REAL DEFAULT 0,

    -- 状态: 1=草稿 2=已发送 3=已确认 4=已转订单 5=已取消
    status INTEGER DEFAULT 1,

    -- 日期
    pi_date TEXT NOT NULL,
    valid_until TEXT,
    confirmed_at TEXT,
    converted_at TEXT,

    -- 条款
    payment_terms TEXT DEFAULT '100% before shipment',
    delivery_terms TEXT DEFAULT 'EXW',
    lead_time TEXT DEFAULT '3-7 working days',
    notes TEXT,

    -- 关联
    sales_order_id INTEGER REFERENCES sales_orders(id),

    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);

-- ============================================================================
-- 2. PI 明细表
-- ============================================================================
CREATE TABLE IF NOT EXISTS proforma_invoice_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pi_id INTEGER NOT NULL REFERENCES proforma_invoices(id) ON DELETE CASCADE,
    product_id INTEGER REFERENCES products(id),
    product_name TEXT NOT NULL,
    model TEXT,                                -- 型号
    quantity INTEGER NOT NULL,
    unit_price REAL NOT NULL,
    total_price REAL NOT NULL,
    notes TEXT,
    sort_order INTEGER DEFAULT 0,
    created_at TEXT DEFAULT (datetime('now'))
);

-- ============================================================================
-- 3. CI 表 (商业发票)
-- ============================================================================
CREATE TABLE IF NOT EXISTS commercial_invoices (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    ci_code TEXT UNIQUE NOT NULL,              -- CI-YYYYMMDD-XXXX
    sales_order_id INTEGER NOT NULL REFERENCES sales_orders(id),
    pi_id INTEGER REFERENCES proforma_invoices(id),

    -- 客户信息（从订单复制）
    customer_id INTEGER,
    customer_name TEXT NOT NULL,
    customer_email TEXT,
    customer_phone TEXT,
    customer_address TEXT,

    -- 金额
    currency TEXT DEFAULT 'USD',
    subtotal REAL DEFAULT 0,
    discount REAL DEFAULT 0,
    total_amount REAL DEFAULT 0,
    paid_amount REAL DEFAULT 0,

    -- 状态: 1=草稿 2=已发送 3=已付款
    status INTEGER DEFAULT 1,

    -- 日期
    ci_date TEXT NOT NULL,
    paid_at TEXT,

    notes TEXT,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);

-- ============================================================================
-- 4. CI 明细表
-- ============================================================================
CREATE TABLE IF NOT EXISTS commercial_invoice_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    ci_id INTEGER NOT NULL REFERENCES commercial_invoices(id) ON DELETE CASCADE,
    product_id INTEGER REFERENCES products(id),
    product_name TEXT NOT NULL,
    model TEXT,
    quantity INTEGER NOT NULL,
    unit_price REAL NOT NULL,
    total_price REAL NOT NULL,
    notes TEXT,
    sort_order INTEGER DEFAULT 0,
    created_at TEXT DEFAULT (datetime('now'))
);

-- ============================================================================
-- 5. 修改订单表 (如果不存在则添加字段)
-- ============================================================================
-- 添加 pi_id 字段
ALTER TABLE orders ADD COLUMN pi_id INTEGER REFERENCES proforma_invoices(id);

-- 添加 ci_id 字段
ALTER TABLE orders ADD COLUMN ci_id INTEGER REFERENCES commercial_invoices(id);

-- 添加 order_source 字段
ALTER TABLE orders ADD COLUMN order_source TEXT DEFAULT 'manual';

-- ============================================================================
-- 6. 索引
-- ============================================================================
CREATE INDEX IF NOT EXISTS idx_pi_code ON proforma_invoices(pi_code);
CREATE INDEX IF NOT EXISTS idx_pi_customer_id ON proforma_invoices(customer_id);
CREATE INDEX IF NOT EXISTS idx_pi_status ON proforma_invoices(status);
CREATE INDEX IF NOT EXISTS idx_pi_date ON proforma_invoices(pi_date);

CREATE INDEX IF NOT EXISTS idx_pi_items_pi_id ON proforma_invoice_items(pi_id);

CREATE INDEX IF NOT EXISTS idx_ci_code ON commercial_invoices(ci_code);
CREATE INDEX IF NOT EXISTS idx_ci_order_id ON commercial_invoices(sales_order_id);
CREATE INDEX IF NOT EXISTS idx_ci_status ON commercial_invoices(status);
CREATE INDEX IF NOT EXISTS idx_ci_date ON commercial_invoices(ci_date);

CREATE INDEX IF NOT EXISTS idx_ci_items_ci_id ON commercial_invoice_items(ci_id);
