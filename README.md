# ciciERP

基于 Rust + SQLite + Leptos 的轻量级 ERP 系统。

## 技术栈

- **后端**: Rust (Axum)
- **数据库**: SQLite (WAL 模式)
- **前端**: Leptos (WASM) - 待实现
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
```

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
| 产品管理 | ✅ | CRUD + 全文搜索 |
| 供应商管理 | ✅ | CRUD + 产品关联 |
| 客户管理 | ✅ | CRUD |
| 订单管理 | ✅ | CRUD + 发货/取消 |
| 库存管理 | ✅ | 查询 + 更新 + 锁定 |
| 采购管理 | 🚧 | 待实现 |
| 物流管理 | 🚧 | 待实现 |
| AI 管家 | 🚧 | 待实现 |

## 许可证

MIT
