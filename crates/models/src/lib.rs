//! ciciERP 数据模型
//!
//! 包含所有业务实体的定义

pub mod product;
pub mod supplier;
pub mod customer;
pub mod order;
pub mod inventory;
pub mod common;
pub mod user;
pub mod auth;
pub mod purchase;
pub mod logistics;
pub mod exchange_rate;
pub mod api_client;
pub mod webhook;
pub mod proforma_invoice;
pub mod commercial_invoice;

pub use product::*;
pub use supplier::*;
pub use customer::*;
pub use order::*;
pub use inventory::*;
pub use common::*;
pub use user::*;
pub use auth::*;
pub use purchase::*;
pub use logistics::*;
pub use exchange_rate::*;
pub use api_client::*;
pub use webhook::*;
pub use proforma_invoice::*;
pub use commercial_invoice::*;
