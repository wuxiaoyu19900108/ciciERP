//! 数据库连接池管理

use anyhow::{Context, Result};
use md5::{Md5, Digest};
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use std::path::Path;
use tracing::info;

/// 解析 SQL 文件，正确处理 BEGIN...END 块
fn parse_sql_statements(sql: &str) -> Vec<String> {
    let mut statements = Vec::new();
    let mut current = String::new();
    let mut in_begin_end = false;

    for line in sql.lines() {
        let trimmed = line.trim();

        // 跳过注释行
        if trimmed.starts_with("--") || trimmed.is_empty() {
            continue;
        }

        // 检测 BEGIN...END 块（用于 TRIGGER 等）
        if trimmed.contains("BEGIN") && !in_begin_end {
            in_begin_end = true;
        }

        current.push_str(line);
        current.push('\n');

        // 在 BEGIN...END 块内，检测 END; 结束
        if in_begin_end {
            if trimmed == "END;" || trimmed.ends_with("END;") {
                in_begin_end = false;
                let stmt = current.trim().to_string();
                if !stmt.is_empty() {
                    statements.push(stmt);
                }
                current.clear();
            }
        } else if trimmed.ends_with(';') {
            // 普通语句，分号结尾
            let stmt = current.trim().to_string();
            if !stmt.is_empty() {
                statements.push(stmt);
            }
            current.clear();
        }
    }

    // 处理最后一个语句（可能没有分号结尾）
    let stmt = current.trim().to_string();
    if !stmt.is_empty() {
        statements.push(stmt);
    }

    statements
}

/// 数据库配置
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub path: String,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            path: "./data/cicierp.db".to_string(),
        }
    }
}

/// 数据库连接池
#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
}

/// 迁移记录
#[derive(Debug, sqlx::FromRow)]
struct MigrationRecord {
    id: i64,
    name: String,
    executed_at: String,
}

impl Database {
    /// 创建数据库连接池
    pub async fn new(config: &DatabaseConfig) -> Result<Self> {
        // 确保数据目录存在
        if let Some(parent) = Path::new(&config.path).parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .context("Failed to create database directory")?;
        }

        let db_url = format!("sqlite:{}?mode=rwc", config.path);
        info!("Connecting to database: {}", db_url);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&db_url)
            .await
            .context("Failed to connect to database")?;

        // 设置 SQLite PRAGMA
        sqlx::query("PRAGMA journal_mode = WAL")
            .execute(&pool)
            .await?;
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await?;
        sqlx::query("PRAGMA cache_size = -64000")
            .execute(&pool)
            .await?;
        sqlx::query("PRAGMA synchronous = NORMAL")
            .execute(&pool)
            .await?;

        info!("Database connected successfully");
        Ok(Self { pool })
    }

    /// 从现有连接池创建
    pub fn from_pool(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// 获取连接池引用
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// 确保迁移追踪表存在
    async fn ensure_migrations_table(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS _migrations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                checksum TEXT NOT NULL,
                executed_at TEXT NOT NULL DEFAULT (datetime('now'))
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .context("Failed to create migrations table")?;
        Ok(())
    }

    /// 检查迁移是否已执行
    async fn is_migration_executed(&self, name: &str) -> Result<bool> {
        let record: Option<MigrationRecord> = sqlx::query_as(
            "SELECT id, name, executed_at FROM _migrations WHERE name = ?"
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record.is_some())
    }

    /// 记录迁移执行
    async fn record_migration(&self, name: &str, checksum: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO _migrations (name, checksum) VALUES (?, ?)"
        )
        .bind(name)
        .bind(checksum)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// 运行迁移脚本（带版本追踪）
    pub async fn run_migrations(&self) -> Result<()> {
        info!("Running database migrations...");

        // 确保迁移表存在
        self.ensure_migrations_table().await?;

        // 定义迁移列表（按顺序执行）
        let migrations = vec![
            ("001_init", include_str!("../migrations/001_init.sql")),
            ("002_product_costs_content", include_str!("../migrations/002_product_costs_content.sql")),
            ("003_product_price_refactor", include_str!("../migrations/003_product_price_refactor.sql")),
            ("004_customer_notes", include_str!("../migrations/004_customer_notes.sql")),
            ("005_purchase_multi_supplier", include_str!("../migrations/005_purchase_multi_supplier.sql")),
            ("006_exchange_rates", include_str!("../migrations/006_exchange_rates.sql")),
            ("007_integration_api", include_str!("../migrations/007_integration_api.sql")),
            ("008_pi_ci_flow", include_str!("../migrations/008_pi_ci_flow.sql")),
            ("009_order_terms", include_str!("../migrations/009_order_terms.sql")),
            ("010_product_model", include_str!("../migrations/010_product_model.sql")),
        ];

        for (name, sql) in migrations {
            // 检查是否已执行
            if self.is_migration_executed(name).await? {
                info!("Migration {} already executed, skipping", name);
                continue;
            }

            info!("Executing migration: {}", name);

            // 计算校验和
            let mut hasher = Md5::new();
            hasher.update(sql);
            let checksum = format!("{:x}", hasher.finalize());

            // 解析并执行每个语句
            // 需要正确处理 BEGIN...END 块（如 TRIGGER）
            let statements = parse_sql_statements(sql);

            for statement in statements {
                if statement.is_empty() {
                    continue;
                }
                sqlx::query(&statement)
                    .execute(&self.pool)
                    .await
                    .context(format!("Failed to execute migration {}: {}", name, &statement[..50.min(statement.len())]))?;
            }

            // 记录迁移
            self.record_migration(name, &checksum).await?;
            info!("Migration {} completed", name);
        }

        info!("Database migrations completed");
        Ok(())
    }

    /// 检查数据库健康状态
    pub async fn health_check(&self) -> Result<bool> {
        let result: (i64,) = sqlx::query_as("SELECT 1")
            .fetch_one(&self.pool)
            .await?;
        Ok(result.0 == 1)
    }
}
