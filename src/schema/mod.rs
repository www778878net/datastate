//! Schema - 数据表结构定义
//!
//! - BaseSchema: 基础表结构
//! - CidSchema: 公司级数据隔离
//! - UidSchema: 用户级数据隔离

mod base_schema;

pub use base_schema::{BaseSchema, CidSchema, UidSchema, SchemaType};
