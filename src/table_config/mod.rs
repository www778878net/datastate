//! TableConfig - 表配置模块
//!
//! - TableSet: 表配置集合
//! - UidCid: 数据隔离类型

mod table_set;

pub use table_set::{TableSet, TableConfigJson, UidCid, TableConfigManager};
