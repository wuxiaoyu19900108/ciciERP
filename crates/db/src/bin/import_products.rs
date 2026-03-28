//! 产品数据导入脚本
//!
//! 从 docs/product_cost_list.md 导入产品数据到 ciciERP 数据库
//!
//! 用法: cargo run --bin import_products

use anyhow::{Context, Result};
use chrono::Utc;
use cicierp_db::pool::Database;
use serde::Deserialize;
use sqlx::SqlitePool;
use std::fs;

/// 导入配置
const EXCHANGE_RATE: f64 = 6.81;
const ALIBABA_FEE_RATE: f64 = 0.025;

/// 产品数据结构（从 JSON 解析）
#[derive(Debug, Deserialize)]
struct ProductData {
    products: Vec<ProductItem>,
}

#[derive(Debug, Deserialize)]
struct ProductItem {
    model: String,
    name: String,
    cost_rmb: f64,
    cost_usd: f64,
    profit_margin_usd: Option<f64>,
    alibaba_fee_usd: Option<f64>,
    selling_price_usd: Option<f64>,
    stock_qty: Option<i64>,
    size: Option<String>,
    weight: Option<f64>,
    notes: Option<String>,
    supplier: Option<String>,
}

/// 导入结果统计
#[derive(Default)]
struct ImportStats {
    total: u32,
    created: u32,
    skipped: u32,
    failed: u32,
    errors: Vec<String>,
}

impl ImportStats {
    fn success(&mut self) {
        self.total += 1;
        self.created += 1;
    }

    fn skip(&mut self, reason: &str) {
        self.total += 1;
        self.skipped += 1;
        println!("  跳过: {}", reason);
    }

    fn fail(&mut self, product: &str, error: &str) {
        self.total += 1;
        self.failed += 1;
        self.errors.push(format!("{}: {}", product, error));
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== ciciERP 产品数据导入工具 ===\n");

    // 1. 读取数据文件
    let data_path = "docs/product_cost_list.md";
    println!("读取数据文件: {}", data_path);

    let content = fs::read_to_string(data_path)
        .context("无法读取数据文件")?;

    // 2. 提取 JSON 数据
    let products = parse_products_from_markdown(&content)?;
    println!("解析到 {} 个产品\n", products.len());

    // 3. 连接数据库并运行迁移
    let db_path = std::env::var("DATABASE_PATH")
        .unwrap_or_else(|_| "cicierp.db".to_string());
    println!("连接数据库: {}", db_path);

    let db = Database::new(&cicierp_db::pool::DatabaseConfig {
        path: db_path,
    })
    .await
    .context("数据库连接失败")?;

    // 运行迁移
    println!("运行数据库迁移...");
    db.run_migrations().await?;
    println!("迁移完成\n");

    let pool = db.pool();

    // 4. 导入产品
    let mut stats = ImportStats::default();

    for (index, item) in products.iter().enumerate() {
        println!("[{}/{}] 导入: {} ({})", index + 1, products.len(), item.name, item.model);

        match import_product(&pool, item).await {
            Ok(true) => {
                stats.success();
                println!("  成功");
            }
            Ok(false) => {
                stats.skip(&format!("产品已存在: {}", item.model));
            }
            Err(e) => {
                stats.fail(&item.name, &e.to_string());
                println!("  失败: {}", e);
            }
        }
    }

    // 5. 输出统计
    println!("\n=== 导入完成 ===");
    println!("总计: {}", stats.total);
    println!("成功: {}", stats.created);
    println!("跳过: {}", stats.skipped);
    println!("失败: {}", stats.failed);

    if !stats.errors.is_empty() {
        println!("\n失败详情:");
        for error in &stats.errors {
            println!("  - {}", error);
        }
    }

    // 6. 写入导入报告
    let report = generate_report(&stats);
    fs::write("import_report.md", &report)
        .context("写入报告失败")?;
    println!("\n导入报告已保存到: import_report.md");

    Ok(())
}

/// 从 Markdown 内容中解析产品数据
fn parse_products_from_markdown(content: &str) -> Result<Vec<ProductItem>> {
    // 查找 ```json 代码块（第二个，因为第一个是汇率配置）
    let mut json_block_start = 0;
    let mut count = 0;
    for pos in content.match_indices("```json").map(|(i, _)| i) {
        count += 1;
        if count == 2 {
            json_block_start = pos;
            break;
        }
    }

    if json_block_start == 0 {
        anyhow::bail!("找不到产品数据 JSON 代码块");
    }

    // 找到代码块内容起始（跳过 ```json 和换行）
    let content_start = content[json_block_start..]
        .find('\n')
        .map(|i| json_block_start + i + 1)
        .context("找不到 JSON 内容起始")?;

    // 找到代码块结束
    let content_end = content[content_start..]
        .find("```")
        .map(|i| content_start + i)
        .context("找不到 JSON 代码块结束")?;

    let json_content = &content[content_start..content_end];

    // 解析 JSON
    let data: ProductData = serde_json::from_str(json_content)
        .context("JSON 解析失败")?;

    Ok(data.products)
}

/// 导入单个产品
async fn import_product(pool: &SqlitePool, item: &ProductItem) -> Result<bool> {
    let now = Utc::now().to_rfc3339();

    // 1. 检查产品是否已存在（按 product_code）
    let existing: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM products WHERE product_code = ? AND deleted_at IS NULL"
    )
    .bind(&item.model)
    .fetch_optional(pool)
    .await?;

    if existing.is_some() {
        return Ok(false); // 产品已存在，跳过
    }

    // 2. 开始事务
    let mut tx = pool.begin().await?;

    // 3. 插入产品（只插入 products 表中存在的列）
    let result = sqlx::query(
        r#"
        INSERT INTO products (
            product_code, name, status,
            purchase_price, sale_price,
            description, created_at, updated_at,
            name_en, slug, category_id, brand_id,
            weight, volume, description_en,
            specifications, main_image, images, is_featured, is_new,
            view_count, sales_count
        ) VALUES (?, ?, 1, ?, ?, ?, ?, ?, NULL, NULL, NULL, NULL, NULL, NULL, NULL, '{}', NULL, '[]', 0, 0, 0, 0)
        "#
    )
    .bind(&item.model)
    .bind(&item.name)
    .bind(item.cost_rmb)  // purchase_price = cost_rmb
    .bind(item.cost_rmb)  // sale_price = cost_rmb（参考售价，实际售价由 product_prices 管理）
    .bind(&item.notes)    // description = notes
    .bind(&now)
    .bind(&now)
    .execute(&mut *tx)
    .await?;

    let product_id = result.last_insert_rowid();

    // 4. 插入成本记录
    sqlx::query(
        r#"
        INSERT INTO product_costs (
            product_id, supplier_id, cost_cny, cost_usd, currency,
            exchange_rate, profit_margin, platform_fee_rate, platform_fee,
            sale_price_usd, quantity, purchase_order_id, is_reference,
            effective_date, notes, created_at, updated_at
        ) VALUES (?, NULL, ?, ?, 'CNY', ?, 0, 0.025, NULL, NULL, 1, NULL, 1, NULL, ?, ?, ?)
        "#
    )
    .bind(product_id)
    .bind(item.cost_rmb)
    .bind(item.cost_usd)
    .bind(EXCHANGE_RATE)
    .bind(&item.notes)
    .bind(&now)
    .bind(&now)
    .execute(&mut *tx)
    .await?;

    // 5. 插入售价记录（如果有售价信息）
    if let Some(sale_price_usd) = item.selling_price_usd {
        let sale_price_cny = sale_price_usd * EXCHANGE_RATE;

        sqlx::query(
            r#"
            INSERT INTO product_prices (
                product_id, platform, sale_price_cny, sale_price_usd, exchange_rate,
                profit_margin, platform_fee_rate, platform_fee, is_reference,
                effective_date, notes, created_at, updated_at
            ) VALUES (?, 'alibaba', ?, ?, ?, 0, 0.025, ?, 1, NULL, '从成本清单导入', ?, ?)
            "#
        )
        .bind(product_id)
        .bind(sale_price_cny)
        .bind(sale_price_usd)
        .bind(EXCHANGE_RATE)
        .bind(item.alibaba_fee_usd)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
    }

    // 6. 提交事务
    tx.commit().await?;

    Ok(true)
}

/// 生成导入报告
fn generate_report(stats: &ImportStats) -> String {
    let now = Utc::now().format("%Y-%m-%d %H:%M:%S UTC");

    format!(
        r#"# 产品数据导入报告

**导入时间**: {}

## 导入统计

| 指标 | 数量 |
|------|------|
| 总计 | {} |
| 成功 | {} |
| 跳过 | {} |
| 失败 | {} |

## 失败详情

{}

## 说明

- 成功: 新创建的产品
- 跳过: 产品编号已存在，未重复导入
- 失败: 导入过程中发生错误

---
*报告由 import_products 工具自动生成*
"#,
        now,
        stats.total,
        stats.created,
        stats.skipped,
        stats.failed,
        if stats.errors.is_empty() {
            "无".to_string()
        } else {
            stats.errors.iter().map(|e| format!("- {}", e)).collect::<Vec<_>>().join("\n")
        }
    )
}
