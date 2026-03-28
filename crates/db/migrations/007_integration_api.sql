-- 对接 API 相关表
-- 用于支持外部系统（如 cicishop）与 ERP 的数据对接

-- API 客户端表
-- 存储对接系统的认证信息
CREATE TABLE IF NOT EXISTS api_clients (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    client_id TEXT NOT NULL UNIQUE,           -- 客户端标识（如 cicishop）
    client_name TEXT NOT NULL,                -- 客户端名称
    api_key TEXT NOT NULL UNIQUE,             -- API Key
    api_secret TEXT NOT NULL,                 -- API Secret（用于签名）
    permissions TEXT NOT NULL DEFAULT '[]',   -- 权限列表（JSON数组）
    rate_limit INTEGER NOT NULL DEFAULT 1000, -- 每小时请求限制
    status INTEGER NOT NULL DEFAULT 1,        -- 状态：1=启用, 0=禁用
    last_used_at TEXT,                        -- 最后使用时间
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Webhook 订阅表
-- 存储事件推送订阅配置
CREATE TABLE IF NOT EXISTS webhook_subscriptions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    client_id INTEGER NOT NULL,               -- 关联的 API 客户端
    event_type TEXT NOT NULL,                 -- 事件类型（如 order.shipped）
    endpoint_url TEXT NOT NULL,               -- 推送地址
    secret TEXT NOT NULL,                     -- 签名密钥
    status INTEGER NOT NULL DEFAULT 1,        -- 状态：1=启用, 0=禁用
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (client_id) REFERENCES api_clients(id) ON DELETE CASCADE
);

-- Webhook 发送记录表
-- 记录 webhook 推送历史
CREATE TABLE IF NOT EXISTS webhook_deliveries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    subscription_id INTEGER NOT NULL,         -- 关联的订阅
    event_type TEXT NOT NULL,                 -- 事件类型
    payload TEXT NOT NULL,                    -- 请求体（JSON）
    response_status INTEGER,                  -- HTTP 响应状态码
    response_body TEXT,                       -- 响应体
    attempts INTEGER NOT NULL DEFAULT 1,      -- 重试次数
    delivered_at TEXT,                        -- 发送成功时间
    error_message TEXT,                       -- 错误信息
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (subscription_id) REFERENCES webhook_subscriptions(id) ON DELETE CASCADE
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_api_clients_client_id ON api_clients(client_id);
CREATE INDEX IF NOT EXISTS idx_api_clients_api_key ON api_clients(api_key);
CREATE INDEX IF NOT EXISTS idx_webhook_subscriptions_client_id ON webhook_subscriptions(client_id);
CREATE INDEX IF NOT EXISTS idx_webhook_subscriptions_event_type ON webhook_subscriptions(event_type);
CREATE INDEX IF NOT EXISTS idx_webhook_deliveries_subscription_id ON webhook_deliveries(subscription_id);
CREATE INDEX IF NOT EXISTS idx_webhook_deliveries_created_at ON webhook_deliveries(created_at);
