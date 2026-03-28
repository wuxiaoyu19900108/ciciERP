//! ciciERP 数据库层
//!
//! 提供 SQLite 数据库连接和查询功能

pub mod pool;
pub mod queries;

pub use pool::*;
