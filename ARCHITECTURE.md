# ciciERP 技术架构设计文档

## 文档信息
- **项目名称**: ciciERP
- **版本**: v2.0
- **文档日期**: 2026-02-27
- **架构师**: Claude (AI 架构专家)
- **技术栈**: Rust + SQLite + HTMX + Tailwind

---

## 1. 架构概述

### 1.1 设计原则

ciciERP 遵循以下核心设计原则：

| 原则 | 说明 | 具体实现 |
|-----|------|---------|
| **极简部署** | 单文件数据库，零配置 | SQLite 单文件存储 |
| **模块松耦合** | 各模块独立，通过 API 交互 | 模块间 RESTful API 通信 |
| **高性能** | 原生编译，零运行时开销 | 纯 Rust 技术栈 |
| **类型安全** | 编译期保证类型正确 | Rust 强类型系统 |
| **易于维护** | 统一语言，降低复杂度 | 前后端均为 Rust |

### 1.2 技术栈选型（v2.0 精简版）

#### 核心技术栈
```
┌─────────────────────────────────────────────────────┐
│                    ciciERP v2.1                     │
├─────────────────────────────────────────────────────┤
│  前端:     HTMX + Tailwind CSS (服务端渲染)         │
│  后端:     Axum (Rust)                              │
│  数据库:   SQLite (单文件, WAL 模式)                │
│  缓存:     内存缓存 + SQLite                        │
│  搜索:     SQLite FTS5 全文搜索                     │
│  AI:       Claude API (飞书 Bot)                    │
└─────────────────────────────────────────────────────┘
```

#### 技术栈对比

| 组件 | 旧方案 (v1.0) | 新方案 (v2.1) | 优势 |
|-----|--------------|--------------|------|
| **数据库** | PostgreSQL + Redis | SQLite | 零配置、单文件、易备份 |
| **后端** | Next.js + Rust | 纯 Rust (Axum) | 统一技术栈、高性能 |
| **前端** | Next.js (React) | HTMX + Tailwind | 零JS、服务端渲染、极省资源 |
| **缓存** | Redis | SQLite + 内存 | 简化架构、减少依赖 |
| **消息队列** | Redis Streams | SQLite + 内存 | 足够用、简化部署 |
| **全文搜索** | PostgreSQL FTS | SQLite FTS5 | 内置支持、无需额外组件 |

### 1.3 架构优势

1. **部署简单**：单个可执行文件 + 单个数据库文件
2. **资源占用极低**：无 Node.js 运行时、无 Redis、无 PostgreSQL、无 WASM
3. **备份容易**：复制 SQLite 文件即可完成备份
4. **开发效率高**：服务端渲染，无需复杂前端框架
5. **性能优异**：原生编译，零 JS 开销

---

## 2. 系统架构图

### 2.1 整体架构

```
┌─────────────────────────────────────────────────────────────────┐
│                        用户入口层                                │
├─────────────────────────────────┬───────────────────────────────┤
│         Web 前端                │        AI 飞书聊天             │
│    HTMX + Tailwind CSS         │    飞书 Bot + Claude API       │
│    (服务端渲染)                 │                                │
└───────────────┬─────────────────┴───────────────┬───────────────┘
                │                                 │
                │  HTTP/WebSocket                 │  Webhook
                │                                 │
┌───────────────▼─────────────────────────────────▼───────────────┐
│                      Rust 后端服务 (Axum)                        │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                     API 路由层                               ││
│  │  /api/v1/products  /api/v1/orders  /api/v1/inventory  ...  ││
│  └─────────────────────────────────────────────────────────────┘│
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                     模块服务层 (松耦合)                       ││
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐          ││
│  │  │ Product │ │  Order  │ │Inventory│ │Customer │          ││
│  │  │ Module  │ │ Module  │ │ Module  │ │ Module  │          ││
│  │  └────┬────┘ └────┬────┘ └────┬────┘ └────┬────┘          ││
│  │       │           │           │           │                ││
│  │  ┌────▼────┐ ┌────▼────┐ ┌────▼────┐ ┌────▼────┐          ││
│  │  │Supplier │ │Purchase │ │Logistics│ │  Sync   │          ││
│  │  │ Module  │ │ Module  │ │ Module  │ │ Module  │          ││
│  │  └─────────┘ └─────────┘ └─────────┘ └─────────┘          ││
│  └─────────────────────────────────────────────────────────────┘│
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                     数据访问层 (SQLx)                        ││
│  └─────────────────────────────────────────────────────────────┘│
└─────────────────────────────────┬───────────────────────────────┘
                                  │
                                  │ SQL
                                  ▼
                    ┌─────────────────────────┐
                    │       SQLite            │
                    │   (单文件数据库)         │
                    │   - 主数据存储           │
                    │   - FTS5 全文搜索        │
                    │   - WAL 模式并发         │
                    └─────────────────────────┘
```

### 2.2 两个用户入口

```
┌─────────────────────────────────────────────────────────────────┐
│                         用户入口                                 │
├────────────────────────────────┬────────────────────────────────┤
│                                │                                │
│   ┌────────────────────┐      │      ┌────────────────────┐   │
│   │                    │      │      │                    │   │
│   │    Web 前端        │      │      │    AI 飞书聊天      │   │
│   │                    │      │      │                    │   │
│   │  ┌──────────────┐  │      │      │  ┌──────────────┐  │   │
│   │  │   Leptos     │  │      │      │  │   飞书 Bot    │  │   │
│   │  │   (WASM)     │  │      │      │  │              │  │   │
│   │  └──────────────┘  │      │      │  └──────────────┘  │   │
│   │         │          │      │      │         │          │   │
│   │  ┌──────────────┐  │      │      │  ┌──────────────┐  │   │
│   │  │  Tailwind    │  │      │      │  │  Claude API  │  │   │
│   │  │  CSS         │  │      │      │  │  (意图识别)   │  │   │
│   │  └──────────────┘  │      │      │  └──────────────┘  │   │
│   │                    │      │      │                    │   │
│   │  功能:             │      │      │  功能:             │   │
│   │  - 产品管理        │      │      │  - 自然语言查询    │   │
│   │  - 订单处理        │      │      │  - 智能预警        │   │
│   │  - 库存查看        │      │      │  - 快捷操作        │   │
│   │  - 报表分析        │      │      │  - 状态通知        │   │
│   │                    │      │      │                    │   │
│   └────────────────────┘      │      └────────────────────┘   │
│                                │                                │
└────────────────────────────────┴────────────────────────────────┘
```

### 2.3 模块间松耦合设计

```
┌─────────────────────────────────────────────────────────────────┐
│                     模块服务层 (松耦合)                          │
│                                                                  │
│  各模块通过 RESTful API 交互，耦合最小化                          │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                     Product Module                        │   │
│  │  ┌────────────────────────────────────────────────────┐  │   │
│  │  │ REST API: /api/v1/products                         │  │   │
│  │  │ - GET    /products        (列表)                   │  │   │
│  │  │ - GET    /products/:id    (详情)                   │  │   │
│  │  │ - POST   /products        (创建)                   │  │   │
│  │  │ - PUT    /products/:id    (更新)                   │  │   │
│  │  │ - DELETE /products/:id    (删除)                   │  │   │
│  │  └────────────────────────────────────────────────────┘  │   │
│  └──────────────────────────────────────────────────────────┘   │
│                              │                                   │
│                    HTTP API │ (松耦合)                           │
│                              ▼                                   │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                     Order Module                          │   │
│  │  调用 Product Module API 获取产品信息                      │   │
│  │  调用 Inventory Module API 扣减库存                        │   │
│  └──────────────────────────────────────────────────────────┘   │
│                              │                                   │
│                              ▼                                   │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                   Inventory Module                        │   │
│  │  独立服务，被其他模块调用                                   │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## 3. 项目目录结构

```
ciciERP/
├── Cargo.toml                    # Workspace 配置
├── Cargo.lock
├── README.md
├── ARCHITECTURE.md               # 本文档
├── PRODUCT_SPEC.md               # 产品规格
│
├── crates/                       # Rust crates
│   │
│   ├── api/                      # Axum Web 服务
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs           # 入口
│   │       ├── routes/           # API 路由
│   │       │   ├── mod.rs
│   │       │   ├── products.rs
│   │       │   ├── orders.rs
│   │       │   ├── inventory.rs
│   │       │   ├── customers.rs
│   │       │   ├── suppliers.rs
│   │       │   ├── purchases.rs
│   │       │   └── logistics.rs
│   │       ├── middleware/       # 中间件
│   │       │   ├── auth.rs
│   │       │   └── logging.rs
│   │       └── error.rs          # 错误处理
│   │
│   ├── frontend/                 # Leptos WASM 前端
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs           # WASM 入口
│   │       ├── app.rs            # 应用根组件
│   │       ├── components/       # UI 组件
│   │       │   ├── layout.rs
│   │       │   ├── sidebar.rs
│   │       │   ├── table.rs
│   │       │   └── form.rs
│   │       ├── pages/            # 页面
│   │       │   ├── dashboard.rs
│   │       │   ├── products.rs
│   │       │   ├── orders.rs
│   │       │   ├── inventory.rs
│   │       │   ├── customers.rs
│   │       │   └── suppliers.rs
│   │       └── api/              # API 客户端
│   │           └── client.rs
│   │
│   ├── ai-butler/                # AI 管家服务
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── feishu/           # 飞书 Bot
│   │       │   ├── mod.rs
│   │       │   ├── bot.rs
│   │       │   └── message.rs
│   │       ├── intent/           # 意图识别
│   │       │   ├── mod.rs
│   │       │   └── claude.rs
│   │       └── handlers/         # 指令处理
│   │           ├── mod.rs
│   │           ├── query.rs
│   │           └── action.rs
│   │
│   ├── db/                       # 数据库层
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── schema.rs         # 表结构定义
│   │       ├── migrations/       # 迁移脚本
│   │       │   └── 001_init.sql
│   │       └── queries/          # 查询语句
│   │           ├── products.rs
│   │           ├── orders.rs
│   │           └── inventory.rs
│   │
│   ├── models/                   # 数据模型 (共享)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── product.rs
│   │       ├── order.rs
│   │       ├── inventory.rs
│   │       ├── customer.rs
│   │       ├── supplier.rs
│   │       ├── purchase.rs
│   │       └── logistics.rs
│   │
│   └── utils/                    # 工具函数
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── cache.rs          # 内存缓存
│           ├── queue.rs          # 内存队列
│           └── search.rs         # FTS5 搜索
│
├── migrations/                   # SQL 迁移文件
│   └── 001_init.sql
│
├── static/                       # 静态资源
│   ├── css/
│   └── images/
│
├── tests/                        # 集成测试
│   ├── api_tests.rs
│   └── db_tests.rs
│
└── config/                       # 配置文件
    ├── default.toml
    └── production.toml
```

---

## 4. 数据库设计

### 4.1 SQLite 配置

```sql
-- 启用 WAL 模式（并发读写）
PRAGMA journal_mode = WAL;

-- 设置外键约束
PRAGMA foreign_keys = ON;

-- 设置缓存大小（负数表示 KB）
PRAGMA cache_size = -64000;  -- 64MB

-- 设置同步模式
PRAGMA synchronous = NORMAL;
```

### 4.2 核心表结构

#### 4.2.1 产品管理

```sql
-- 产品主表
CREATE TABLE products (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    product_code TEXT UNIQUE NOT NULL,
    name TEXT NOT NULL,
    name_en TEXT,
    slug TEXT UNIQUE,
    category_id INTEGER REFERENCES categories(id),
    brand_id INTEGER REFERENCES brands(id),

    -- 价格
    purchase_price REAL NOT NULL,
    sale_price REAL NOT NULL,
    compare_price REAL,

    -- 物理属性
    weight REAL,
    volume REAL,

    -- 描述
    description TEXT,
    description_en TEXT,
    specifications TEXT,  -- JSON 格式

    -- 媒体
    main_image TEXT,
    images TEXT,          -- JSON 数组

    -- 状态
    status INTEGER DEFAULT 1,  -- 1:上架 2:下架 3:草稿
    is_featured INTEGER DEFAULT 0,
    is_new INTEGER DEFAULT 0,

    -- 统计
    view_count INTEGER DEFAULT 0,
    sales_count INTEGER DEFAULT 0,

    -- 时间戳
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now')),
    deleted_at TEXT
);

CREATE INDEX idx_products_code ON products(product_code);
CREATE INDEX idx_products_category ON products(category_id);
CREATE INDEX idx_products_status ON products(status);

-- 全文搜索虚拟表
CREATE VIRTUAL TABLE products_fts USING fts5(
    name,
    description,
    content='products',
    content_rowid='id'
);

-- 触发器：保持 FTS 索引同步
CREATE TRIGGER products_ai AFTER INSERT ON products BEGIN
    INSERT INTO products_fts(rowid, name, description)
    VALUES (new.id, new.name, new.description);
END;

CREATE TRIGGER products_ad AFTER DELETE ON products BEGIN
    INSERT INTO products_fts(products_fts, rowid, name, description)
    VALUES('delete', old.id, old.name, old.description);
END;

CREATE TRIGGER products_au AFTER UPDATE ON products BEGIN
    INSERT INTO products_fts(products_fts, rowid, name, description)
    VALUES('delete', old.id, old.name, old.description);
    INSERT INTO products_fts(rowid, name, description)
    VALUES (new.id, new.name, new.description);
END;
```

#### 4.2.2 SKU 表

```sql
CREATE TABLE product_skus (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    product_id INTEGER NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    sku_code TEXT UNIQUE NOT NULL,

    -- 规格值 (JSON)
    spec_values TEXT NOT NULL DEFAULT '{}',

    -- 价格
    sale_price REAL NOT NULL,
    cost_price REAL NOT NULL,
    compare_price REAL,

    -- 库存
    stock_quantity INTEGER NOT NULL DEFAULT 0,
    available_quantity INTEGER NOT NULL DEFAULT 0,
    locked_quantity INTEGER NOT NULL DEFAULT 0,
    safety_stock INTEGER DEFAULT 10,

    -- 媒体
    sku_image TEXT,

    -- 条码
    barcode TEXT,
    qr_code TEXT,

    -- 状态
    status INTEGER DEFAULT 1,

    -- 时间戳
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX idx_skus_product ON product_skus(product_id);
CREATE INDEX idx_skus_code ON product_skus(sku_code);
CREATE INDEX idx_skus_barcode ON product_skus(barcode);
```

#### 4.2.3 产品-供应商多对多关系

```sql
-- 供应商表
CREATE TABLE suppliers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    supplier_code TEXT UNIQUE NOT NULL,
    name TEXT NOT NULL,
    name_en TEXT,

    -- 联系信息
    contact_person TEXT,
    contact_phone TEXT,
    contact_email TEXT,
    address TEXT,

    -- 企业信息
    credit_code TEXT,
    tax_id TEXT,

    -- 评级
    rating_level TEXT DEFAULT 'C',
    rating_score REAL DEFAULT 3.5,

    -- 付款条件
    payment_terms INTEGER DEFAULT 30,

    -- 统计
    total_orders INTEGER DEFAULT 0,
    total_amount REAL DEFAULT 0,

    -- 状态
    status INTEGER DEFAULT 1,

    -- 时间戳
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now')),
    deleted_at TEXT
);

-- 产品-供应商关联表（多对多）
CREATE TABLE product_suppliers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    product_id INTEGER NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    supplier_id INTEGER NOT NULL REFERENCES suppliers(id) ON DELETE CASCADE,

    -- 供应商特定信息
    supplier_sku TEXT,              -- 供应商的 SKU
    purchase_price REAL,            -- 采购价
    min_order_qty INTEGER DEFAULT 1,
    lead_time INTEGER,              -- 交货周期（天）
    is_primary INTEGER DEFAULT 0,   -- 是否首选供应商

    -- 时间戳
    created_at TEXT DEFAULT (datetime('now')),

    UNIQUE(product_id, supplier_id)
);

CREATE INDEX idx_product_suppliers_product ON product_suppliers(product_id);
CREATE INDEX idx_product_suppliers_supplier ON product_suppliers(supplier_id);
```

#### 4.2.4 客户管理

```sql
CREATE TABLE customers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    customer_code TEXT UNIQUE NOT NULL,

    -- 基本信息
    name TEXT NOT NULL,
    mobile TEXT UNIQUE,
    email TEXT,
    gender INTEGER,
    birthday TEXT,
    avatar TEXT,

    -- 等级和积分
    level_id INTEGER REFERENCES customer_levels(id),
    points INTEGER DEFAULT 0,

    -- 统计
    total_orders INTEGER DEFAULT 0,
    total_amount REAL DEFAULT 0,

    -- 标签 (JSON 数组)
    tags TEXT DEFAULT '[]',

    -- 来源
    source TEXT NOT NULL,
    external_id TEXT,
    external_platform TEXT,

    -- 状态
    status INTEGER DEFAULT 1,

    -- 最后活跃
    last_login_at TEXT,
    last_order_at TEXT,

    -- 时间戳
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now')),
    deleted_at TEXT
);

CREATE INDEX idx_customers_code ON customers(customer_code);
CREATE INDEX idx_customers_mobile ON customers(mobile);
CREATE INDEX idx_customers_external ON customers(external_platform, external_id);
```

#### 4.2.5 订单管理

```sql
-- 订单主表
CREATE TABLE orders (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    order_code TEXT UNIQUE NOT NULL,

    -- 平台信息
    platform TEXT NOT NULL,
    platform_order_id TEXT,

    -- 客户信息
    customer_id INTEGER REFERENCES customers(id),
    customer_name TEXT,
    customer_mobile TEXT,

    -- 订单状态
    order_type INTEGER DEFAULT 1,
    order_status INTEGER DEFAULT 1,
    payment_status INTEGER DEFAULT 1,
    fulfillment_status INTEGER DEFAULT 1,

    -- 金额
    total_amount REAL NOT NULL,
    discount_amount REAL DEFAULT 0,
    shipping_fee REAL DEFAULT 0,
    paid_amount REAL DEFAULT 0,

    -- 币种
    currency TEXT DEFAULT 'CNY',

    -- 备注
    customer_note TEXT,
    internal_note TEXT,

    -- 时间戳
    payment_time TEXT,
    ship_time TEXT,
    finish_time TEXT,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX idx_orders_code ON orders(order_code);
CREATE INDEX idx_orders_platform ON orders(platform, platform_order_id);
CREATE INDEX idx_orders_customer ON orders(customer_id);
CREATE INDEX idx_orders_status ON orders(order_status);
CREATE INDEX idx_orders_created ON orders(created_at);

-- 订单明细表
CREATE TABLE order_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    order_id INTEGER NOT NULL REFERENCES orders(id) ON DELETE CASCADE,

    -- 产品信息
    product_id INTEGER REFERENCES products(id),
    sku_id INTEGER REFERENCES product_skus(id),
    product_name TEXT NOT NULL,
    product_code TEXT,
    sku_code TEXT,
    sku_spec TEXT,           -- JSON

    -- 图片快照
    product_image TEXT,

    -- 数量和价格
    quantity INTEGER NOT NULL,
    unit_price REAL NOT NULL,
    subtotal REAL NOT NULL,
    discount_amount REAL DEFAULT 0,
    total_amount REAL NOT NULL,

    -- 成本
    cost_price REAL,

    -- 时间戳
    created_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX idx_order_items_order ON order_items(order_id);
CREATE INDEX idx_order_items_product ON order_items(product_id);

-- 订单收货地址表
CREATE TABLE order_addresses (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    order_id INTEGER NOT NULL REFERENCES orders(id) ON DELETE CASCADE,

    -- 收货人
    receiver_name TEXT NOT NULL,
    receiver_phone TEXT NOT NULL,

    -- 地址
    country TEXT NOT NULL,
    province TEXT,
    city TEXT,
    district TEXT,
    address TEXT NOT NULL,
    postal_code TEXT,

    -- 时间戳
    created_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX idx_order_addresses_order ON order_addresses(order_id);
```

#### 4.2.6 库存管理

```sql
-- 库存主表
CREATE TABLE inventory (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    sku_id INTEGER NOT NULL UNIQUE REFERENCES product_skus(id),

    -- 数量
    total_quantity INTEGER NOT NULL DEFAULT 0,
    available_quantity INTEGER NOT NULL DEFAULT 0,
    locked_quantity INTEGER NOT NULL DEFAULT 0,
    damaged_quantity INTEGER NOT NULL DEFAULT 0,

    -- 安全库存
    safety_stock INTEGER DEFAULT 10,
    max_stock INTEGER,

    -- 时间戳
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX idx_inventory_sku ON inventory(sku_id);

-- 库存流水表
CREATE TABLE stock_movements (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    movement_code TEXT UNIQUE NOT NULL,

    -- SKU
    sku_id INTEGER NOT NULL REFERENCES product_skus(id),

    -- 变动信息
    movement_type INTEGER NOT NULL,  -- 1:入库 2:出库 3:调拨 4:盘点 5:损耗
    quantity INTEGER NOT NULL,
    before_quantity INTEGER NOT NULL,
    after_quantity INTEGER NOT NULL,

    -- 关联单据
    reference_type TEXT,
    reference_id INTEGER,
    reference_code TEXT,

    -- 备注
    note TEXT,

    -- 操作人
    operator_id INTEGER,
    operator_name TEXT,

    -- 时间戳
    created_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX idx_stock_movements_sku ON stock_movements(sku_id);
CREATE INDEX idx_stock_movements_type ON stock_movements(movement_type);
CREATE INDEX idx_stock_movements_created ON stock_movements(created_at);
```

### 4.3 数据关系图

```
┌─────────────┐         ┌─────────────┐         ┌─────────────┐
│  customers  │         │   products  │         │  suppliers  │
│             │         │             │         │             │
└──────┬──────┘         └──────┬──────┘         └──────┬──────┘
       │                       │                       │
       │                       │                       │
       │                ┌──────┴──────┐                │
       │                │             │                │
       │                ▼             ▼                │
       │         ┌──────────────────────┐              │
       │         │  product_suppliers   │◀────────────┘
       │         │    (多对多关联)       │
       │         └──────────────────────┘
       │                       │
       │                       │
       │                ┌──────┴──────┐
       │                │             │
       │                ▼             ▼
       │         ┌─────────────┐ ┌─────────────┐
       │         │product_skus │ │ inventory   │
       │         └─────────────┘ └─────────────┘
       │                │
       │                │
┌──────┴──────┐  ┌──────┴──────┐
│   orders    │  │order_items  │
│             │◀─│             │
└──────┬──────┘  └─────────────┘
       │
       │
┌──────┴──────┐  ┌─────────────┐
│ shipments   │  │order_addresses│
│             │  │             │
└─────────────┘  └─────────────┘
```

---

## 5. API 设计

### 5.1 API 规范

#### 统一响应格式

```rust
// 成功响应
pub struct ApiResponse<T> {
    pub code: u16,
    pub message: String,
    pub data: T,
    pub timestamp: i64,
}

// 错误响应
pub struct ApiError {
    pub code: u16,
    pub message: String,
    pub errors: Option<Vec<FieldError>>,
}

pub struct FieldError {
    pub field: String,
    pub message: String,
}
```

#### 分页响应

```rust
pub struct PagedResponse<T> {
    pub items: Vec<T>,
    pub pagination: Pagination,
}

pub struct Pagination {
    pub page: u32,
    pub page_size: u32,
    pub total: u64,
    pub total_pages: u32,
}
```

### 5.2 核心 API 列表

#### 产品模块

```rust
// 路由: /api/v1/products
Router::new()
    .route("/products", get(list_products))
    .route("/products/:id", get(get_product))
    .route("/products", post(create_product))
    .route("/products/:id", put(update_product))
    .route("/products/:id", delete(delete_product))
    .route("/products/search", get(search_products))  // FTS5 搜索
    .route("/products/batch", post(batch_products))
```

#### 订单模块

```rust
// 路由: /api/v1/orders
Router::new()
    .route("/orders", get(list_orders))
    .route("/orders/:id", get(get_order))
    .route("/orders", post(create_order))
    .route("/orders/:id", put(update_order))
    .route("/orders/:id/cancel", post(cancel_order))
    .route("/orders/:id/ship", post(ship_order))
    .route("/orders/:id/refund", post(refund_order))
```

#### 库存模块

```rust
// 路由: /api/v1/inventory
Router::new()
    .route("/inventory", get(list_inventory))
    .route("/inventory/:sku_id", get(get_inventory))
    .route("/inventory/:sku_id", put(update_inventory))
    .route("/inventory/lock", post(lock_inventory))
    .route("/inventory/unlock", post(unlock_inventory))
    .route("/inventory/alerts", get(get_alerts))  // 库存预警
    .route("/inventory/movements", get(list_movements))
```

#### 客户模块

```rust
// 路由: /api/v1/customers
Router::new()
    .route("/customers", get(list_customers))
    .route("/customers/:id", get(get_customer))
    .route("/customers", post(create_customer))
    .route("/customers/:id", put(update_customer))
    .route("/customers/:id/orders", get(get_customer_orders))
```

#### 供应商模块

```rust
// 路由: /api/v1/suppliers
Router::new()
    .route("/suppliers", get(list_suppliers))
    .route("/suppliers/:id", get(get_supplier))
    .route("/suppliers", post(create_supplier))
    .route("/suppliers/:id", put(update_supplier))
    .route("/suppliers/:id/products", get(get_supplier_products))
```

### 5.3 模块间 API 调用

```rust
// 订单模块调用库存模块示例
impl OrderService {
    pub async fn create_order(&self, order: CreateOrderRequest) -> Result<Order> {
        // 1. 调用库存模块 API 锁定库存
        for item in &order.items {
            let lock_req = LockInventoryRequest {
                sku_id: item.sku_id,
                quantity: item.quantity,
                order_id: None,
            };
            self.inventory_client.lock(lock_req).await?;
        }

        // 2. 创建订单
        let order = self.db.create_order(order).await?;

        // 3. 调用库存模块 API 扣减库存
        for item in &order.items {
            self.inventory_client.deduct(DeductRequest {
                sku_id: item.sku_id,
                quantity: item.quantity,
                order_id: order.id,
            }).await?;
        }

        Ok(order)
    }
}
```

---

## 6. AI 管家设计

### 6.1 架构

```
┌─────────────────────────────────────────────────────────────┐
│                      飞书客户端                              │
└─────────────────────────┬───────────────────────────────────┘
                          │
                          │ 用户消息
                          ▼
┌─────────────────────────────────────────────────────────────┐
│                    AI 管家服务 (Rust)                        │
│  ┌───────────────────────────────────────────────────────┐  │
│  │                   飞书 Bot 模块                        │  │
│  │  - 接收消息                                            │  │
│  │  - 发送消息                                            │  │
│  │  - 卡片交互                                            │  │
│  └───────────────────────────────────────────────────────┘  │
│                          │                                   │
│                          ▼                                   │
│  ┌───────────────────────────────────────────────────────┐  │
│  │                 意图识别模块 (Claude)                  │  │
│  │  - 自然语言理解                                        │  │
│  │  - 实体提取                                            │  │
│  │  - 意图分类                                            │  │
│  └───────────────────────────────────────────────────────┘  │
│                          │                                   │
│                          ▼                                   │
│  ┌───────────────────────────────────────────────────────┐  │
│  │                    指令处理模块                        │  │
│  │  - 查询类指令                                          │  │
│  │  - 操作类指令                                          │  │
│  │  - 分析类指令                                          │  │
│  └───────────────────────────────────────────────────────┘  │
│                          │                                   │
│                          │ HTTP API                          │
│                          ▼                                   │
└──────────────────────────┼───────────────────────────────────┘
                           │
                           ▼
              ┌─────────────────────────┐
              │   ciciERP API 服务      │
              │   (Axum)                │
              └─────────────────────────┘
```

### 6.2 意图类型

```rust
pub enum IntentType {
    // 查询类
    QueryOrder,        // 查询订单
    QueryInventory,    // 查询库存
    QueryCustomer,     // 查询客户
    QueryProduct,      // 查询产品
    QueryReport,       // 查询报表

    // 操作类
    CreateOrder,       // 创建订单
    ShipOrder,         // 订单发货
    UpdateInventory,   // 更新库存

    // 分析类
    AnalyzeSales,      // 销售分析
    AnalyzeInventory,  // 库存分析

    // 系统类
    SystemStatus,      // 系统状态
    ExportData,        // 导出数据

    // 未知
    Unknown,
}

pub struct Intent {
    pub intent_type: IntentType,
    pub confidence: f32,
    pub entities: HashMap<String, Value>,
    pub need_clarification: bool,
    pub clarification_question: Option<String>,
}
```

### 6.3 Claude 提示词

```rust
const SYSTEM_PROMPT: &str = r#"
你是 ciciERP 系统的 AI 管家，通过飞书为用户提供服务。

## 你的能力
1. 订单管理：查询、创建、发货
2. 库存管理：查询、更新、预警
3. 客户管理：查询、分析
4. 数据分析：生成报表、趋势分析

## 响应格式 (JSON)
{
  "intent_type": "QueryOrder",
  "confidence": 0.95,
  "entities": {
    "order_id": "ORD001",
    "date": "2026-02-27"
  },
  "need_clarification": false
}

## 注意事项
- 敏感操作需要二次确认
- 如果意图不明确，设置 need_clarification 为 true
"#;
```

---

## 7. 前端设计 (Leptos)

### 7.1 技术栈

```toml
[dependencies]
leptos = { version = "0.6", features = ["csr"] }
leptos_router = "0.6"
leptos_meta = "0.6"
gloo-net = "0.5"          # HTTP 客户端
web-sys = "0.3"           # Web API
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

### 7.2 组件结构

```rust
// app.rs
#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <div class="flex h-screen">
                <Sidebar />
                <main class="flex-1 overflow-auto">
                    <Routes>
                        <Route path="/" view=Dashboard />
                        <Route path="/products" view=Products />
                        <Route path="/orders" view=Orders />
                        <Route path="/inventory" view=Inventory />
                        <Route path="/customers" view=Customers />
                        <Route path="/suppliers" view=Suppliers />
                    </Routes>
                </main>
            </div>
        </Router>
    }
}

// pages/products.rs
#[component]
pub fn Products() -> impl IntoView {
    let products = create_resource(|| (), |_| async { get_products().await });

    view! {
        <div class="p-6">
            <h1 class="text-2xl font-bold mb-4">"产品管理"</h1>
            <Suspense fallback=|| view! { <p>"加载中..."</p> }>
                {move || products.get().map(|data| view! {
                    <ProductTable products=data />
                })}
            </Suspense>
        </div>
    }
}
```

### 7.3 API 客户端

```rust
// api/client.rs
pub async fn get_products() -> Result<Vec<Product>, Error> {
    gloo_net::http::Request::get("/api/v1/products")
        .send()
        .await?
        .json()
        .await
}

pub async fn create_product(product: CreateProductRequest) -> Result<Product, Error> {
    gloo_net::http::Request::post("/api/v1/products")
        .json(&product)?
        .send()
        .await?
        .json()
        .await
}
```

---

## 8. 缓存和队列设计

### 8.1 内存缓存

```rust
// utils/cache.rs
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{Duration, Instant};

pub struct MemoryCache<T> {
    data: Arc<RwLock<HashMap<String, CacheEntry<T>>>>,
    default_ttl: Duration,
}

struct CacheEntry<T> {
    value: T,
    expires_at: Instant,
}

impl<T: Clone + Send + Sync + 'static> MemoryCache<T> {
    pub fn new(default_ttl: Duration) -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
            default_ttl,
        }
    }

    pub async fn get(&self, key: &str) -> Option<T> {
        let data = self.data.read().await;
        data.get(key)
            .filter(|entry| entry.expires_at > Instant::now())
            .map(|entry| entry.value.clone())
    }

    pub async fn set(&self, key: String, value: T) {
        self.set_with_ttl(key, value, self.default_ttl).await;
    }

    pub async fn set_with_ttl(&self, key: String, value: T, ttl: Duration) {
        let mut data = self.data.write().await;
        data.insert(key, CacheEntry {
            value,
            expires_at: Instant::now() + ttl,
        });
    }
}
```

### 8.2 内存队列

```rust
// utils/queue.rs
use tokio::sync::mpsc;

pub struct TaskQueue<T> {
    sender: mpsc::Sender<T>,
    receiver: mpsc::Receiver<T>,
}

impl<T: Send + 'static> TaskQueue<T> {
    pub fn new(buffer_size: usize) -> Self {
        let (sender, receiver) = mpsc::channel(buffer_size);
        Self { sender, receiver }
    }

    pub async fn enqueue(&self, task: T) -> Result<(), mpsc::error::SendError<T>> {
        self.sender.send(task).await
    }

    pub async fn dequeue(&mut self) -> Option<T> {
        self.receiver.recv().await
    }
}
```

---

## 9. 部署方案

### 9.1 编译

```bash
# 编译后端 (Axum)
cargo build --release -p cicierp-api

# 编译前端 (Leptos WASM)
cargo leptos build --release -p cicierp-frontend

# 编译 AI 管家
cargo build --release -p cicierp-ai-butler
```

### 9.2 运行

```bash
# 启动 API 服务
./cicierp-api --config config/production.toml

# API 服务会同时托管静态文件
# 访问 http://localhost:3000
```

### 9.3 配置文件

```toml
# config/default.toml
[server]
host = "0.0.0.0"
port = 3000

[database]
path = "./data/cicierp.db"

[cache]
default_ttl_seconds = 300

[ai]
claude_api_key = ""
feishu_app_id = ""
feishu_app_secret = ""
```

### 9.4 备份策略

```bash
# 备份脚本 (backup.sh)
#!/bin/bash
DATE=$(date +%Y%m%d_%H%M%S)
cp ./data/cicierp.db ./backups/cicierp_$DATE.db

# 保留最近 7 天的备份
find ./backups -name "cicierp_*.db" -mtime +7 -delete
```

---

## 10. 开发路线图

### 10.1 分阶段开发计划

#### 第一阶段：基础框架（2周）
- [x] 项目结构初始化
- [ ] 数据库 Schema 设计和迁移
- [ ] Axum API 框架搭建
- [ ] Leptos 前端框架搭建
- [ ] 基础认证中间件

#### 第二阶段：核心模块（4周）
- [ ] 产品模块（CRUD + 搜索）
- [ ] 客户模块（CRUD）
- [ ] 订单模块（基础流程）
- [ ] 库存模块（基础操作）

#### 第三阶段：业务完善（3周）
- [ ] 供应商模块（多对多）
- [ ] 采购模块
- [ ] 物流模块
- [ ] 报表统计

#### 第四阶段：AI 管家（2周）
- [ ] 飞书 Bot 集成
- [ ] Claude 意图识别
- [ ] 指令执行引擎
- [ ] 智能预警

#### 第五阶段：优化上线（1周）
- [ ] 性能优化
- [ ] 测试覆盖
- [ ] 文档完善
- [ ] 部署上线

### 10.2 技术依赖

```toml
# Cargo.toml (workspace)
[workspace]
members = ["crates/*"]
resolver = "2"

[workspace.dependencies]
# Web 框架
axum = "0.7"
tower = "0.4"
tower-http = "0.5"

# 数据库
sqlx = { version = "0.7", features = ["runtime-tokio", "sqlite"] }

# 异步运行时
tokio = { version = "1.35", features = ["full"] }

# 序列化
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# 前端
leptos = "0.6"
leptos_router = "0.6"

# 工具
anyhow = "1.0"
thiserror = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.6", features = ["v4", "serde"] }
```

---

## 11. 总结

ciciERP v2.0 采用精简的纯 Rust 技术栈，具有以下优势：

### 架构优势

| 特点 | 说明 |
|-----|------|
| **极简部署** | 单可执行文件 + 单数据库文件 |
| **统一技术栈** | 前后端均为 Rust，类型共享 |
| **高性能** | 原生编译，零运行时开销 |
| **松耦合** | 模块间通过 API 交互，易于维护 |
| **易于备份** | 复制 SQLite 文件即可 |

### 技术对比

| 对比项 | v1.0 | v2.0 |
|-------|------|------|
| **部署复杂度** | 高（多个组件） | 低（单一可执行文件） |
| **资源占用** | 高（Node.js + PostgreSQL + Redis） | 低（纯 Rust + SQLite） |
| **开发效率** | 中（多语言） | 高（统一 Rust） |
| **运维成本** | 高 | 低 |
| **备份难度** | 中（需数据库导出） | 低（复制文件） |

### 当前状态

- **设计阶段**：✅ 已完成
- **实现阶段**：⏳ 等待开发启动
- **预计工期**：10-12 周

---

**文档结束**

**更新记录**:
- 2026-02-26: v1.0 初始版本，Next.js + PostgreSQL 架构
- 2026-02-27: v2.0 大重构，纯 Rust + SQLite + Leptos 架构
