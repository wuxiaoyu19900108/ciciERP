-- 客户跟进日期
ALTER TABLE customers ADD COLUMN next_followup_date TEXT;
ALTER TABLE customers ADD COLUMN followup_notes TEXT;

-- 订单利润快照
ALTER TABLE order_items ADD COLUMN platform_fee_rate REAL DEFAULT 0;
ALTER TABLE order_items ADD COLUMN platform_fee REAL DEFAULT 0;
ALTER TABLE order_items ADD COLUMN gross_profit REAL DEFAULT 0;
ALTER TABLE order_items ADD COLUMN net_profit REAL DEFAULT 0;

ALTER TABLE orders ADD COLUMN total_gross_profit REAL DEFAULT 0;
ALTER TABLE orders ADD COLUMN total_net_profit REAL DEFAULT 0;

-- 操作日志表
CREATE TABLE IF NOT EXISTS operation_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER,
    username TEXT,
    action TEXT NOT NULL,
    module TEXT NOT NULL,
    target_id INTEGER,
    target_code TEXT,
    description TEXT,
    ip_address TEXT,
    created_at TEXT DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_operation_logs_module ON operation_logs(module, created_at);
CREATE INDEX IF NOT EXISTS idx_operation_logs_user ON operation_logs(user_id, created_at);

-- 回填历史订单利润快照（用现有 cost_price 估算，平台费暂按0）
UPDATE order_items SET
    gross_profit = total_amount - COALESCE(cost_price * quantity, 0),
    net_profit = total_amount - COALESCE(cost_price * quantity, 0)
WHERE gross_profit = 0 OR gross_profit IS NULL;

UPDATE orders SET
    total_gross_profit = COALESCE((
        SELECT SUM(gross_profit) FROM order_items WHERE order_id = orders.id
    ), 0),
    total_net_profit = COALESCE((
        SELECT SUM(net_profit) FROM order_items WHERE order_id = orders.id
    ), 0)
WHERE total_gross_profit = 0 OR total_gross_profit IS NULL;
