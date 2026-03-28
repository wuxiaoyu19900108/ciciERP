#!/bin/bash
# 创建默认 API 客户端供 cicishop 使用

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
DB_PATH="${PROJECT_DIR}/data/cicierp.db"

echo "创建默认 API 客户端..."

# 检查数据库是否存在
if [ ! -f "$DB_PATH" ]; then
    echo "错误: 数据库文件不存在: $DB_PATH"
    echo "请先运行应用程序以创建数据库"
    exit 1
fi

# 生成随机 API Key 和 Secret
API_KEY="ak_$(openssl rand -hex 32)"
API_SECRET="$(openssl rand -hex 32)"

# 检查是否已存在
EXISTING=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM api_clients WHERE client_id = 'cicishop'")
if [ "$EXISTING" -gt 0 ]; then
    echo "API 客户端 'cicishop' 已存在"
    echo ""
    echo "现有客户端信息:"
    sqlite3 -header -column "$DB_PATH" "SELECT id, client_id, client_name, status FROM api_clients WHERE client_id = 'cicishop'"
    echo ""
    read -p "是否要重新生成 API Key？(y/N) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        sqlite3 "$DB_PATH" "UPDATE api_clients SET api_key = '$API_KEY', api_secret = '$API_SECRET', updated_at = datetime('now') WHERE client_id = 'cicishop'"
        echo "已更新 API Key 和 Secret"
    fi
    exit 0
fi

# 创建新的 API 客户端
sqlite3 "$DB_PATH" <<EOF
INSERT INTO api_clients (client_id, client_name, api_key, api_secret, permissions, rate_limit, status, created_at, updated_at)
VALUES ('cicishop', 'ciciShop 独立站', '$API_KEY', '$API_SECRET', '["*"]', 5000, 1, datetime('now'), datetime('now'));
EOF

echo ""
echo "✅ API 客户端创建成功！"
echo ""
echo "客户端信息:"
echo "  Client ID: cicishop"
echo "  API Key:   $API_KEY"
echo "  API Secret: $API_SECRET"
echo ""
echo "⚠️  请妥善保存 API Secret，它不会再次显示！"
echo ""
echo "使用示例:"
echo ""
echo "  # 计算 HMAC 签名"
echo "  timestamp=\$(date +%s)"
echo "  body='{\"test\": true}'"
echo "  signature=\$(echo -n \"\${API_KEY}\${timestamp}\${body}\" | openssl dgst -sha256 -hmac \"\$API_SECRET\" | sed 's/^.* //')"
echo ""
echo "  # 发送请求"
echo "  curl -X POST http://localhost:3000/api/v1/integration/products/batch \\"
echo "    -H \"Authorization: Bearer \$API_KEY\" \\"
echo "    -H \"X-Timestamp: \$timestamp\" \\"
echo "    -H \"X-Signature: sha256=\$signature\" \\"
echo "    -H \"Content-Type: application/json\" \\"
echo "    -d \"\$body\""
