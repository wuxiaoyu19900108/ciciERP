# 币种板块和月度分析报告开发计划

## 概述

为 ciciERP 订单模块添加币种筛选和月度分析报告功能。

## 当前数据库状态

### orders 表相关字段
- `currency`: TEXT - 币种（USD/CNY）
- `exchange_rate`: REAL - 汇率
- `platform`: TEXT - 平台（ali_import, import, manual）
- `total_amount`: REAL - 订单总金额
- `created_at`: TEXT - 创建时间

### order_items 表相关字段
- `cost_price`: REAL - 成本价
- `total_amount`: REAL - 订单项金额
- `quantity`: INTEGER - 数量

### 当前数据分布
- USD 订单：60 条（阿里国际站）
- CNY 订单：69 条（速卖通）
- 总订单：129 条

---

## 任务1：订单币种筛选

### 1.1 后端修改

#### 文件：`crates/api/src/routes/web.rs`

1. 修改 `OrdersQuery` 结构体，添加 currency 参数：
```rust
pub struct OrdersQuery {
    pub page: Option<u32>,
    pub status: Option<i64>,
    pub currency: Option<String>,  // 新增
}
```

2. 修改 `orders_page` 函数，传递 currency 参数给查询：
```rust
let currency = query.currency.clone();
```

#### 文件：`crates/db/src/queries/orders.rs`

1. 修改 `list` 方法签名，添加 currency 参数：
```rust
pub async fn list(
    &self,
    page: u32,
    page_size: u32,
    order_status: Option<i64>,
    payment_status: Option<i64>,
    customer_id: Option<i64>,
    platform: Option<&str>,
    date_from: Option<&str>,
    date_to: Option<&str>,
    keyword: Option<&str>,
    currency: Option<&str>,  // 新增
) -> AppResult<PagedResult<OrderListItem>>
```

2. 在查询中添加 currency 过滤条件：
```rust
if let Some(c) = currency {
    list_query.push(" AND o.currency = ");
    list_query.push_bind(c);
}
```

### 1.2 前端修改

#### 文件：`crates/api/templates/orders.html`

添加币种 Tab 切换 UI（参考产品列表页面的 Tab 样式）：

```html
<!-- 币种切换 Tab -->
<div class="mb-4 flex gap-2">
    <a href="/orders?currency=USD{% if query.status %}&status={{ query.status }}{% endif %}"
       class="px-4 py-2 rounded-lg text-sm font-medium {% if query.currency == 'USD' or query.currency == None %}bg-blue-600 text-white{% else %}bg-gray-100 text-gray-600 hover:bg-gray-200{% endif %}">
        美金订单 (USD)
    </a>
    <a href="/orders?currency=CNY{% if query.status %}&status={{ query.status }}{% endif %}"
       class="px-4 py-2 rounded-lg text-sm font-medium {% if query.currency == 'CNY' %}bg-blue-600 text-white{% else %}bg-gray-100 text-gray-600 hover:bg-gray-200{% endif %}">
        人民币订单 (CNY)
    </a>
</div>
```

### 1.3 创建订单默认币种

#### 文件：`crates/api/templates/order_new.html`

1. 添加币种选择下拉框：
```html
<div class="mb-4">
    <label class="block text-sm font-medium text-gray-700 mb-1">结算币种</label>
    <select name="currency" class="w-full px-3 py-2 border rounded-lg">
        <option value="USD" selected>美金 (USD)</option>
        <option value="CNY">人民币 (CNY)</option>
    </select>
</div>
```

---

## 任务2：月度分析报告页面

### 2.1 后端 API

#### 文件：`crates/api/src/routes/web.rs`

添加新路由：
```rust
.route("/analytics", get(analytics_page))
```

添加 `analytics_page` 函数：
```rust
pub async fn analytics_page(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Query(query): Query<AnalyticsQuery>,
) -> Html<String>
```

### 2.2 数据查询

#### 文件：`crates/db/src/queries/orders.rs`

添加分析方法：
```rust
pub struct MonthlyStats {
    pub month: String,
    pub currency: String,
    pub order_count: i64,
    pub total_sales: f64,
    pub total_cost: f64,
    pub total_profit: f64,
    pub profit_rate: f64,
}

pub struct ProductRanking {
    pub product_name: String,
    pub order_count: i64,
    pub total_quantity: i64,
    pub total_sales: f64,
}

pub struct PlatformStats {
    pub platform: String,
    pub order_count: i64,
    pub total_sales: f64,
    pub total_profit: f64,
}

pub async fn get_monthly_stats(&self, year: i32, month: Option<i32>) -> AppResult<Vec<MonthlyStats>>
pub async fn get_product_ranking(&self, limit: i32, currency: Option<&str>) -> AppResult<Vec<ProductRanking>>
pub async fn get_platform_stats(&self) -> AppResult<Vec<PlatformStats>>
```

### 2.3 前端页面

#### 文件：`crates/api/templates/analytics.html`

创建分析报告页面，包含：

1. **月份选择器**
   - 年份下拉框（2025, 2026）
   - 月份下拉框（1-12）

2. **概览卡片**
   - USD 销售额
   - CNY 销售额
   - 总利润
   - 订单数量

3. **产品销量排行表**
   - Top 10 产品
   - 显示：产品名、订单数、数量、销售额

4. **平台分布**
   - ali_import（阿里国际站）
   - import（速卖通）
   - 显示：订单数、销售额、利润

5. **图表（使用 Chart.js）**
   - 月度销售趋势图
   - 币种分布饼图

### 2.4 导航菜单

#### 文件：`crates/api/templates/layout.html`

在侧边栏添加"数据分析"菜单项：
```html
<a href="/analytics" class="flex items-center px-4 py-3 text-gray-600 hover:bg-gray-100">
    <span class="mr-3">📊</span>
    数据分析
</a>
```

---

## 实现顺序

1. **第一步**：后端 - 修改 OrdersQuery 和 list 方法，添加 currency 筛选
2. **第二步**：前端 - 订单列表页面添加币种 Tab
3. **第三步**：后端 - 创建分析数据查询方法
4. **第四步**：后端 - 创建 analytics_page 路由
5. **第五步**：前端 - 创建分析报告页面
6. **第六步**：前端 - 添加导航菜单项
7. **第七步**：测试验证

---

## 测试验证

### 订单币种筛选测试
1. 访问 /orders，默认显示所有订单
2. 点击"美金订单"Tab，URL 变为 /orders?currency=USD
3. 点击"人民币订单"Tab，URL 变为 /orders?currency=CNY
4. 验证列表只显示对应币种的订单

### 分析报告测试
1. 访问 /analytics 页面
2. 验证数据正确显示
3. 切换月份，验证数据更新
4. 检查图表是否正常渲染

---

## 文件清单

需要修改的文件：
- `crates/api/src/routes/web.rs` - 添加路由和页面处理
- `crates/db/src/queries/orders.rs` - 添加查询方法
- `crates/api/templates/orders.html` - 币种 Tab
- `crates/api/templates/order_new.html` - 币种选择
- `crates/api/templates/analytics.html` - 新建分析页面
- `crates/api/templates/layout.html` - 导航菜单

---

## 注意事项

1. 保持现有功能不变
2. currency 为空或 null 时视为 USD（向后兼容）
3. 利润计算：total_amount - (cost_price * quantity)
4. 利润率计算：profit / total_amount * 100
5. 使用现有样式风格，保持 UI 一致
