//! 共享数据结构模块
//!
//! 定义 SQLite 和 MySQL 共用的数据结构

mod sys_warn_data;
mod sys_sql_data;

pub use sys_warn_data::SysWarnData;
pub use sys_sql_data::SysSqlData;