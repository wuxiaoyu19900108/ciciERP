-- ciciERP 数据库初始化脚本
-- 版本: 001
-- 日期: 2026-02-27

-- ============================================================================
-- 用户和权限
-- ============================================================================

-- 用户表
CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    email TEXT UNIQUE,
    mobile TEXT UNIQUE,
    real_name TEXT,
    avatar TEXT,
    status INTEGER DEFAULT 1,  -- 1:正常 2:禁用
    last_login_at TEXT,
    last_login_ip TEXT,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now')),
    deleted_at TEXT
);

-- 角色表
CREATE TABLE IF NOT EXISTS roles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    code TEXT NOT NULL UNIQUE,
    description TEXT,
    permissions TEXT DEFAULT '[]',  -- JSON 数组
    status INTEGER DEFAULT 1,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);

-- 用户角色关联
CREATE TABLE IF NOT EXISTS user_roles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role_id INTEGER NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    created_at TEXT DEFAULT (datetime('now')),
    UNIQUE(user_id, role_id)
);

-- ============================================================================
-- 产品管理
-- ============================================================================

-- 分类表
CREATE TABLE IF NOT EXISTS categories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    name_en TEXT,
    slug TEXT UNIQUE NOT NULL,
    parent_id INTEGER REFERENCES categories(id),
    level INTEGER DEFAULT 0,
    path TEXT,  -- 如: "1,5,12"
    icon TEXT,
    image TEXT,
    sort_order INTEGER DEFAULT 0,
    is_visible INTEGER DEFAULT 1,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now')),
    deleted_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_categories_parent ON categories(parent_id);
CREATE INDEX IF NOT EXISTS idx_categories_slug ON categories(slug);

-- 品牌表
CREATE TABLE IF NOT EXISTS brands (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    name_en TEXT,
    slug TEXT UNIQUE NOT NULL,
    logo TEXT,
    description TEXT,
    sort_order INTEGER DEFAULT 0,
    status INTEGER DEFAULT 1,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);

-- 产品主表
CREATE TABLE IF NOT EXISTS products (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    product_code TEXT UNIQUE NOT NULL,
    name TEXT NOT NULL,
    name_en TEXT,
    slug TEXT UNIQUE,
    category_id INTEGER REFERENCES categories(id),
    brand_id INTEGER REFERENCES brands(id),
    purchase_price REAL NOT NULL,
    sale_price REAL NOT NULL,
    compare_price REAL,
    weight REAL,
    volume REAL,
    description TEXT,
    description_en TEXT,
    specifications TEXT DEFAULT '{}',  -- JSON
    main_image TEXT,
    images TEXT DEFAULT '[]',  -- JSON 数组
    status INTEGER DEFAULT 1,  -- 1:上架 2:下架 3:草稿
    is_featured INTEGER DEFAULT 0,
    is_new INTEGER DEFAULT 0,
    view_count INTEGER DEFAULT 0,
    sales_count INTEGER DEFAULT 0,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now')),
    deleted_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_products_code ON products(product_code);
CREATE INDEX IF NOT EXISTS idx_products_category ON products(category_id);
CREATE INDEX IF NOT EXISTS idx_products_brand ON products(brand_id);
CREATE INDEX IF NOT EXISTS idx_products_status ON products(status);

-- 全文搜索虚拟表
CREATE VIRTUAL TABLE IF NOT EXISTS products_fts USING fts5(
    name,
    description,
    content='products',
    content_rowid='id'
);

-- FTS 触发器
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

-- SKU 表
CREATE TABLE IF NOT EXISTS product_skus (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    product_id INTEGER NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    sku_code TEXT UNIQUE NOT NULL,
    spec_values TEXT NOT NULL DEFAULT '{}',  -- JSON
    sale_price REAL NOT NULL,
    cost_price REAL NOT NULL,
    compare_price REAL,
    stock_quantity INTEGER DEFAULT 0,
    available_quantity INTEGER DEFAULT 0,
    locked_quantity INTEGER DEFAULT 0,
    safety_stock INTEGER DEFAULT 10,
    sku_image TEXT,
    barcode TEXT,
    qr_code TEXT,
    status INTEGER DEFAULT 1,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_skus_product ON product_skus(product_id);
CREATE INDEX IF NOT EXISTS idx_skus_code ON product_skus(sku_code);
CREATE INDEX IF NOT EXISTS idx_skus_barcode ON product_skus(barcode);

-- ============================================================================
-- 供应商管理
-- ============================================================================

CREATE TABLE IF NOT EXISTS suppliers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    supplier_code TEXT UNIQUE NOT NULL,
    name TEXT NOT NULL,
    name_en TEXT,
    contact_person TEXT,
    contact_phone TEXT,
    contact_email TEXT,
    address TEXT,
    credit_code TEXT,
    tax_id TEXT,
    bank_name TEXT,
    bank_account TEXT,
    rating_level TEXT DEFAULT 'C',
    rating_score REAL DEFAULT 3.5,
    payment_terms INTEGER DEFAULT 30,
    payment_method TEXT,
    total_orders INTEGER DEFAULT 0,
    total_amount REAL DEFAULT 0,
    status INTEGER DEFAULT 1,  -- 1:合作 2:暂停 3:终止
    notes TEXT,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now')),
    deleted_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_suppliers_code ON suppliers(supplier_code);
CREATE INDEX IF NOT EXISTS idx_suppliers_status ON suppliers(status);

-- 产品-供应商关联（多对多）
CREATE TABLE IF NOT EXISTS product_suppliers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    product_id INTEGER NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    supplier_id INTEGER NOT NULL REFERENCES suppliers(id) ON DELETE CASCADE,
    supplier_sku TEXT,
    purchase_price REAL,
    min_order_qty INTEGER DEFAULT 1,
    lead_time INTEGER,  -- 交货周期（天）
    is_primary INTEGER DEFAULT 0,
    created_at TEXT DEFAULT (datetime('now')),
    UNIQUE(product_id, supplier_id)
);

CREATE INDEX IF NOT EXISTS idx_product_suppliers_product ON product_suppliers(product_id);
CREATE INDEX IF NOT EXISTS idx_product_suppliers_supplier ON product_suppliers(supplier_id);

-- ============================================================================
-- 客户管理
-- ============================================================================

-- 客户等级
CREATE TABLE IF NOT EXISTS customer_levels (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    name_en TEXT,
    level INTEGER UNIQUE NOT NULL,
    min_amount REAL DEFAULT 0,
    min_orders INTEGER DEFAULT 0,
    min_points INTEGER DEFAULT 0,
    discount_percent REAL DEFAULT 0,
    free_shipping INTEGER DEFAULT 0,
    special_services TEXT,
    sort_order INTEGER DEFAULT 0,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);

-- 客户表
CREATE TABLE IF NOT EXISTS customers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    customer_code TEXT UNIQUE NOT NULL,
    name TEXT NOT NULL,
    mobile TEXT UNIQUE,
    email TEXT,
    gender INTEGER,  -- 1:男 2:女
    birthday TEXT,
    avatar TEXT,
    level_id INTEGER REFERENCES customer_levels(id),
    points INTEGER DEFAULT 0,
    total_orders INTEGER DEFAULT 0,
    total_amount REAL DEFAULT 0,
    avg_order_amount REAL,
    tags TEXT DEFAULT '[]',
    attributes TEXT DEFAULT '{}',
    source TEXT NOT NULL,
    external_id TEXT,
    external_platform TEXT,
    status INTEGER DEFAULT 1,  -- 1:正常 2:冻结 3:黑名单
    last_login_at TEXT,
    last_order_at TEXT,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now')),
    deleted_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_customers_code ON customers(customer_code);
CREATE INDEX IF NOT EXISTS idx_customers_mobile ON customers(mobile);
CREATE INDEX IF NOT EXISTS idx_customers_external ON customers(external_platform, external_id);

-- 客户地址
CREATE TABLE IF NOT EXISTS customer_addresses (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    customer_id INTEGER NOT NULL REFERENCES customers(id) ON DELETE CASCADE,
    receiver_name TEXT NOT NULL,
    receiver_phone TEXT NOT NULL,
    country TEXT NOT NULL,
    country_code TEXT,
    province TEXT,
    city TEXT,
    district TEXT,
    address TEXT NOT NULL,
    postal_code TEXT,
    address_type INTEGER DEFAULT 1,  -- 1:家庭 2:公司
    is_default INTEGER DEFAULT 0,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_customer_addresses_customer ON customer_addresses(customer_id);

-- ============================================================================
-- 订单管理
-- ============================================================================

-- 订单主表
CREATE TABLE IF NOT EXISTS orders (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    order_code TEXT UNIQUE NOT NULL,
    platform TEXT NOT NULL,
    platform_order_id TEXT,
    customer_id INTEGER REFERENCES customers(id),
    customer_name TEXT,
    customer_mobile TEXT,
    customer_email TEXT,
    order_type INTEGER DEFAULT 1,  -- 1:普通 2:预售 3:换货
    order_status INTEGER DEFAULT 1,  -- 1:待审核 2:待发货 3:部分发货 4:已发货 5:已完成 6:已取消 7:售后中
    payment_status INTEGER DEFAULT 1,  -- 1:未付 2:部分付 3:已付 4:已退款 5:部分退款
    fulfillment_status INTEGER DEFAULT 1,  -- 1:未发 2:部分发 3:已发 4:已签收
    total_amount REAL NOT NULL,
    subtotal REAL NOT NULL,
    discount_amount REAL DEFAULT 0,
    shipping_fee REAL DEFAULT 0,
    tax_amount REAL DEFAULT 0,
    paid_amount REAL DEFAULT 0,
    refund_amount REAL DEFAULT 0,
    currency TEXT DEFAULT 'CNY',
    exchange_rate REAL,
    coupon_id INTEGER,
    coupon_amount REAL DEFAULT 0,
    points_used INTEGER DEFAULT 0,
    points_discount REAL DEFAULT 0,
    customer_note TEXT,
    internal_note TEXT,
    is_rated INTEGER DEFAULT 0,
    payment_time TEXT,
    ship_time TEXT,
    finish_time TEXT,
    cancel_time TEXT,
    cancel_reason TEXT,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_orders_code ON orders(order_code);
CREATE INDEX IF NOT EXISTS idx_orders_platform ON orders(platform, platform_order_id);
CREATE INDEX IF NOT EXISTS idx_orders_customer ON orders(customer_id);
CREATE INDEX IF NOT EXISTS idx_orders_status ON orders(order_status);
CREATE INDEX IF NOT EXISTS idx_orders_created ON orders(created_at);

-- 订单明细
CREATE TABLE IF NOT EXISTS order_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    order_id INTEGER NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
    product_id INTEGER REFERENCES products(id),
    sku_id INTEGER REFERENCES product_skus(id),
    product_name TEXT NOT NULL,
    product_code TEXT,
    sku_code TEXT,
    sku_spec TEXT,  -- JSON
    product_image TEXT,
    quantity INTEGER NOT NULL,
    unit_price REAL NOT NULL,
    subtotal REAL NOT NULL,
    discount_amount REAL DEFAULT 0,
    total_amount REAL NOT NULL,
    cost_price REAL,
    tax_rate REAL,
    tax_amount REAL,
    refund_quantity INTEGER DEFAULT 0,
    refund_amount REAL DEFAULT 0,
    created_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_order_items_order ON order_items(order_id);
CREATE INDEX IF NOT EXISTS idx_order_items_product ON order_items(product_id);

-- 订单收货地址
CREATE TABLE IF NOT EXISTS order_addresses (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    order_id INTEGER NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
    receiver_name TEXT NOT NULL,
    receiver_phone TEXT NOT NULL,
    country TEXT NOT NULL,
    country_code TEXT,
    province TEXT,
    city TEXT,
    district TEXT,
    address TEXT NOT NULL,
    postal_code TEXT,
    address_type INTEGER DEFAULT 1,
    created_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_order_addresses_order ON order_addresses(order_id);

-- ============================================================================
-- 库存管理
-- ============================================================================

CREATE TABLE IF NOT EXISTS inventory (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    sku_id INTEGER NOT NULL UNIQUE REFERENCES product_skus(id),
    total_quantity INTEGER DEFAULT 0,
    available_quantity INTEGER DEFAULT 0,
    locked_quantity INTEGER DEFAULT 0,
    damaged_quantity INTEGER DEFAULT 0,
    safety_stock INTEGER DEFAULT 10,
    max_stock INTEGER,
    warehouse_id INTEGER,
    location TEXT,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_inventory_sku ON inventory(sku_id);

-- 库存流水
CREATE TABLE IF NOT EXISTS stock_movements (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    movement_code TEXT UNIQUE NOT NULL,
    sku_id INTEGER NOT NULL REFERENCES product_skus(id),
    warehouse_id INTEGER,
    movement_type INTEGER NOT NULL,  -- 1:入库 2:出库 3:调拨 4:盘点 5:损耗 6:锁定 7:解锁
    quantity INTEGER NOT NULL,
    before_quantity INTEGER NOT NULL,
    after_quantity INTEGER NOT NULL,
    reference_type TEXT,
    reference_id INTEGER,
    reference_code TEXT,
    note TEXT,
    operator_id INTEGER,
    operator_name TEXT,
    created_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_stock_movements_sku ON stock_movements(sku_id);
CREATE INDEX IF NOT EXISTS idx_stock_movements_type ON stock_movements(movement_type);
CREATE INDEX IF NOT EXISTS idx_stock_movements_created ON stock_movements(created_at);

-- ============================================================================
-- 采购管理
-- ============================================================================

CREATE TABLE IF NOT EXISTS purchase_orders (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    order_code TEXT UNIQUE NOT NULL,
    supplier_id INTEGER NOT NULL REFERENCES suppliers(id),
    supplier_name TEXT,
    total_amount REAL NOT NULL,
    tax_amount REAL DEFAULT 0,
    paid_amount REAL DEFAULT 0,
    payment_status INTEGER DEFAULT 1,  -- 1:未付 2:部分付 3:已付
    delivery_status INTEGER DEFAULT 1,  -- 1:未发 2:部分发 3:已发
    expected_date TEXT,
    actual_date TEXT,
    status INTEGER DEFAULT 1,  -- 1:待审 2:已审 3:执行中 4:已完成 5:已取消
    approved_by INTEGER,
    approved_at TEXT,
    approval_note TEXT,
    supplier_note TEXT,
    internal_note TEXT,
    attachments TEXT DEFAULT '[]',
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_purchase_orders_code ON purchase_orders(order_code);
CREATE INDEX IF NOT EXISTS idx_purchase_orders_supplier ON purchase_orders(supplier_id);

-- 采购明细
CREATE TABLE IF NOT EXISTS purchase_order_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    order_id INTEGER NOT NULL REFERENCES purchase_orders(id) ON DELETE CASCADE,
    product_id INTEGER REFERENCES products(id),
    sku_id INTEGER REFERENCES product_skus(id),
    product_name TEXT NOT NULL,
    sku_code TEXT,
    spec_values TEXT,
    quantity INTEGER NOT NULL,
    received_qty INTEGER DEFAULT 0,
    unit_price REAL NOT NULL,
    subtotal REAL NOT NULL,
    expected_qty INTEGER,
    expected_date TEXT,
    inspected_qty INTEGER DEFAULT 0,
    qualified_qty INTEGER DEFAULT 0,
    defective_qty INTEGER DEFAULT 0,
    batch_code TEXT,
    production_date TEXT,
    expiry_date TEXT,
    created_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_purchase_items_order ON purchase_order_items(order_id);

-- ============================================================================
-- 物流管理
-- ============================================================================

-- 物流公司
CREATE TABLE IF NOT EXISTS logistics_companies (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    code TEXT UNIQUE NOT NULL,
    name TEXT NOT NULL,
    name_en TEXT,
    service_type TEXT NOT NULL,  -- express/air/sea/land
    api_code TEXT,
    api_config TEXT DEFAULT '{}',
    contact_phone TEXT,
    contact_email TEXT,
    website TEXT,
    tracking_url_template TEXT,
    status INTEGER DEFAULT 1,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);

-- 发货单
CREATE TABLE IF NOT EXISTS shipments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    shipment_code TEXT UNIQUE NOT NULL,
    order_id INTEGER NOT NULL REFERENCES orders(id),
    logistics_id INTEGER REFERENCES logistics_companies(id),
    logistics_name TEXT,
    tracking_number TEXT,
    receiver_name TEXT NOT NULL,
    receiver_phone TEXT NOT NULL,
    receiver_address TEXT NOT NULL,
    package_weight REAL,
    package_volume REAL,
    package_items TEXT NOT NULL DEFAULT '[]',
    package_count INTEGER DEFAULT 1,
    shipping_fee REAL DEFAULT 0,
    actual_shipping_fee REAL,
    estimated_arrival TEXT,
    actual_arrival TEXT,
    status INTEGER DEFAULT 1,  -- 1:已发货 2:运输中 3:已签收 4:异常 5:已退货
    shipping_note TEXT,
    ship_time TEXT,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_shipments_code ON shipments(shipment_code);
CREATE INDEX IF NOT EXISTS idx_shipments_order ON shipments(order_id);
CREATE INDEX IF NOT EXISTS idx_shipments_tracking ON shipments(tracking_number);

-- 物流轨迹
CREATE TABLE IF NOT EXISTS shipment_tracking (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    shipment_id INTEGER NOT NULL REFERENCES shipments(id) ON DELETE CASCADE,
    tracking_time TEXT NOT NULL,
    tracking_status TEXT NOT NULL,
    tracking_description TEXT NOT NULL,
    location TEXT,
    raw_data TEXT,
    created_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_tracking_shipment ON shipment_tracking(shipment_id);
CREATE INDEX IF NOT EXISTS idx_tracking_time ON shipment_tracking(tracking_time);

-- ============================================================================
-- 初始化数据
-- ============================================================================

-- 默认管理员
INSERT INTO users (username, password_hash, email, real_name, status)
VALUES ('admin', '$argon2id$v=19$m=19456,t=2,p=1$test$test', 'admin@ciciERP.com', '管理员', 1);

-- 默认角色
INSERT INTO roles (name, code, description, permissions)
VALUES
    ('超级管理员', 'super_admin', '拥有所有权限', '["*"]'),
    ('管理员', 'admin', '管理后台用户', '["users:*", "roles:*"]'),
    ('运营', 'operator', '日常运营操作', '["products:read", "products:write", "orders:*"]'),
    ('客服', 'service', '客服处理', '["orders:read", "customers:read"]');

-- 默认客户等级
INSERT INTO customer_levels (name, level, min_amount, discount_percent)
VALUES
    ('普通会员', 1, 0, 0),
    ('银卡会员', 2, 1000, 5),
    ('金卡会员', 3, 5000, 10),
    ('钻石会员', 4, 20000, 15);

-- 默认分类
INSERT INTO categories (name, slug, level, sort_order)
VALUES ('未分类', 'uncategorized', 0, 0);

-- ─── 产品成本表（产品模块扩展） ─────────────────────────────────────────────────
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
    quantity INTEGER DEFAULT 1,
    purchase_order_id INTEGER,
    is_reference INTEGER DEFAULT 0,
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
CREATE UNIQUE INDEX IF NOT EXISTS idx_product_costs_reference
    ON product_costs(product_id) WHERE is_reference = 1;

-- ─── 产品售价表（多平台定价） ──────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS product_prices (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    product_id INTEGER NOT NULL,
    platform TEXT NOT NULL DEFAULT 'website',
    sale_price_cny REAL NOT NULL,
    sale_price_usd REAL,
    exchange_rate REAL DEFAULT 7.2,
    profit_margin REAL DEFAULT 0.15,
    platform_fee_rate REAL DEFAULT 0.025,
    platform_fee REAL,
    is_reference INTEGER DEFAULT 0,
    effective_date TEXT,
    notes TEXT,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now')),
    FOREIGN KEY (product_id) REFERENCES products(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_product_prices_product ON product_prices(product_id);
CREATE INDEX IF NOT EXISTS idx_product_prices_platform ON product_prices(platform);
CREATE INDEX IF NOT EXISTS idx_product_prices_effective ON product_prices(effective_date);
CREATE UNIQUE INDEX IF NOT EXISTS idx_product_prices_reference
    ON product_prices(product_id, platform) WHERE is_reference = 1;

-- ─── 产品内容表（独立站 SEO 内容） ────────────────────────────────────────────
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
