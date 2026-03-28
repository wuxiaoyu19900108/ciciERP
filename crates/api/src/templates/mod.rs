//! HTML 模板模块
//!
//! 模板数据结构（实际渲染使用 web.rs 中的字符串拼接）

pub mod base;
pub mod auth;
pub mod dashboard;
pub mod products;
pub mod orders;
pub mod inventory;
pub mod customers;
pub mod suppliers;
pub mod purchases;

pub use base::*;
pub use auth::*;
pub use dashboard::*;
pub use products::*;
pub use orders::*;
pub use inventory::*;
pub use customers::*;
pub use suppliers::*;
pub use purchases::*;
