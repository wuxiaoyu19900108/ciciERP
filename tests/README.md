# ciciERP E2E Tests (Playwright)

## 目录结构

```
tests/
├── package.json          # 依赖配置
├── playwright.config.ts  # Playwright 配置
└── e2e/
    ├── auth.spec.ts          # 认证模块测试（API + UI）
    ├── products.spec.ts      # 产品 API 测试（CRUD）
    └── products-ui.spec.ts   # 产品 UI 测试（浏览器）
```

## 快速开始

### 1. 安装依赖

```bash
cd tests
npm install
npx playwright install chromium
```

### 2. 启动服务器

```bash
# 在项目根目录
cargo run -p cicierp-api
```

### 3. 运行测试

```bash
cd tests

# 运行所有测试
npm test

# 运行特定模块
npm run test:auth
npm run test:products
npm run test:products-ui

# 有头模式（可以看到浏览器）
npm run test:headed

# 交互式 UI 模式
npm run test:ui

# 查看测试报告
npm run test:report
```

## 测试覆盖范围

### `auth.spec.ts`（16 个用例）
- ✅ 登录成功 / 密码错误 / 用户不存在
- ✅ 未授权访问受保护接口
- ✅ 获取当前用户信息（/auth/me）
- ✅ 登出
- ✅ 无效 Token
- ✅ 登录页渲染
- ✅ 登录页错误提示
- ✅ 登录成功跳转

### `products.spec.ts`（16 个用例）
- ✅ 列表查询 + 分页
- ✅ 创建产品（必填/完整/重复编码/缺少 name）
- ✅ 获取详情（存在/不存在）
- ✅ 价格汇总 / 历史价格
- ✅ 更新产品（名称/状态/不存在）
- ✅ 搜索 + 状态筛选
- ✅ 软删除 + 删后不可见

### `products-ui.spec.ts`（14 个用例）
- ✅ 列表页加载 / 表格 / 搜索 / 新建按钮
- ✅ 未登录重定向
- ✅ 新建产品表单 / 提交 / 跳转
- ✅ 详情页 + 编辑按钮 / 返回按钮
- ✅ 编辑页预填 / 保存更新

## 环境变量

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `BASE_URL` | `http://localhost:3000` | 服务地址 |

```bash
BASE_URL=http://your-server:3000 npm test
```
