-- 创建默认 API 客户端供 cicishop 使用
-- 运行方式: sqlite3 data/cicierp.db < scripts/create_default_api_client.sql

-- 注意：运行前请确保已经运行了迁移脚本 007_integration_api.sql

-- 生成随机 API Key 和 Secret（示例值，生产环境应使用安全的随机值）
-- API Key: ak_xxxx (32 字节 hex = 64 字符)
-- API Secret: xxxx (32 字节 hex = 64 字符)

INSERT INTO api_clients (client_id, client_name, api_key, api_secret, permissions, rate_limit, status, created_at, updated_at)
VALUES (
    'cicishop',
    'ciciShop 独立站',
    'ak_' || lower(hex(randomblob(32))),
    lower(hex(randomblob(32))),
    '["*"]',
    5000,
    1,
    datetime('now'),
    datetime('now')
);

-- 查询创建的客户端
SELECT id, client_id, client_name, api_key, api_secret, permissions, rate_limit, status
FROM api_clients
WHERE client_id = 'cicishop';
