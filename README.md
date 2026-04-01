# ciciERP

基于 Rust + SQLite + Axum 的轻量级 ERP 系统。

## 技术栈

- **后端**: Rust (Axum)
- **数据库**: SQLite (WAL 模式)
- **前端**: Askama 服务端模板渲染（SSR）
- **AI 管家**: 飞书 Bot + Claude API - 待实现

## 项目结构

```
ciciERP/
├── crates/
│   ├── api/           # Axum Web 服务
│   ├── db/            # 数据库层
│   ├── models/        # 数据模型
│   └── utils/         # 工具函数
├── migrations/        # 数据库迁移脚本
├── docs/              # 文档
├── config/            # 配置文件
└── scripts/           # 脚本工具
```

## 快速开始

### 环境要求

- Rust 1.75+
- SQLite 3.40+

### 运行

```bash
# 开发模式
cargo run -p cicierp-api

# 生产模式
cargo build --release -p cicierp-api
./target/release/cicierp-api
```

服务将在 http://localhost:3000 启动。

### 环境变量

```bash
HOST=0.0.0.0
PORT=3000
CORS_ORIGINS=http://localhost:3000,https://yourdomain.com  # 生产环境必须设置
NODE_ENV=production                                         # 开启生产模式 CORS 限制
```

完整配置见 `config/default.toml`，环境变量优先级高于配置文件。

## API 文档

启动服务后访问：
- 健康检查: GET /health
- API 文档: [docs/API.md](docs/API.md)

## 开发

### 生成 API 文档

```bash
./scripts/gen_api_docs.sh
```

### 运行测试

```bash
cargo test
```

## 模块

| 模块 | 状态 | 说明 |
|-----|------|------|
| 产品管理 | ✅ | CRUD + 全文搜索 + 成本/价格/内容子模块 |
| 供应商管理 | ✅ | CRUD + 产品关联 |
| 客户管理 | ✅ | CRUD + 收货地址管理 |
| 订单管理 | ✅ | CRUD + 发货/取消/状态流转 |
| 库存管理 | ✅ | 查询 + 更新 + 锁定/解锁 + 低库存预警 |
| 采购管理 | ✅ | CRUD + 审批 + 入库收货 |
| 物流管理 | ✅ | 物流公司管理 + 发货单 + 运踪追踪 |
| 形式发票 (PI) | ✅ | CRUD + 发送/确认/转订单/取消 + Excel 导出 |
| 商业发票 (CI) | ✅ | CRUD + 发送/标记付款 + Excel 导出 |
| 汇率管理 | ✅ | 自动定时拉取 + 手动更新 + 历史记录 |
| 用户与权限 | ✅ | 用户 CRUD + 角色权限 (RBAC) + JWT 认证 |
| 对接 API | ✅ | 供 cicishop 等外部平台同步产品/库存/订单/客户 |
| Web 管理界面 | ✅ | 基于 Askama SSR 的后台管理页面 |
| AI 管家 | 🚧 | 飞书 Bot + Claude API，待实现 |
| 数据分析看板 | 🚧 | 报表与统计，待实现 |

## 许可证

MIT
