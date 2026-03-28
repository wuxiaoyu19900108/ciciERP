-- 汇率表
-- 存储货币汇率历史记录

CREATE TABLE IF NOT EXISTS exchange_rates (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    from_currency TEXT NOT NULL,    -- 源货币：USD
    to_currency TEXT NOT NULL,      -- 目标货币：CNY
    rate REAL NOT NULL,             -- 汇率值（1 USD = ? CNY）
    source TEXT DEFAULT 'api',      -- 来源：api/manual
    effective_date TEXT NOT NULL,   -- 生效日期 YYYY-MM-DD
    created_at TEXT DEFAULT (datetime('now')),
    UNIQUE(from_currency, to_currency, effective_date)
);

-- 创建索引加速查询
CREATE INDEX IF NOT EXISTS idx_exchange_rates_currencies ON exchange_rates(from_currency, to_currency);
CREATE INDEX IF NOT EXISTS idx_exchange_rates_date ON exchange_rates(effective_date);
