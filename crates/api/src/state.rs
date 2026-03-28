//! 应用状态

use cicierp_db::Database;

/// 应用共享状态
#[derive(Clone)]
pub struct AppState {
    pub db: Database,
}

impl AppState {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// 获取数据库连接池
    pub fn pool(&self) -> &sqlx::SqlitePool {
        self.db.pool()
    }
}
