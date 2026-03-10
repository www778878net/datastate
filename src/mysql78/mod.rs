//! Mysql78 - MySQL 数据库操作类
//!
//! 提供连接池管理、预处理语句缓存、重试机制、事务操作

mod mysql78;
mod sys_warn_state;
mod sys_sql_state;

pub use mysql78::{Mysql78, MysqlConfig, MysqlUpInfo, MysqlUpdateResult, MysqlInsertResult};
pub use sys_warn_state::{SysWarnMysqlState, TABLE_NAME as SYS_WARN_TABLE, CREATE_SQL as SYS_WARN_CREATE_SQL};
pub use sys_sql_state::{SysSqlMysqlState, TABLE_NAME as SYS_SQL_TABLE, CREATE_SQL as SYS_SQL_CREATE_SQL};