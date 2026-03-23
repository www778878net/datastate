//! Sqlite78 - SQLite 数据库操作类
//!
//! 提供 Local-First 存储的本地数据库操作能力

mod sqlite78;
mod sys_warn_state;
mod sys_sql_state;

pub use sqlite78::{Sqlite78, UpdateResult, InsertResult, WarnHandler};
pub use sys_warn_state::{SysWarnSqliteState, TABLE_NAME as SYS_WARN_TABLE, CREATE_SQL as SYS_WARN_CREATE_SQL};
pub use sys_sql_state::{SysSqlSqliteState, TABLE_NAME as SYS_SQL_TABLE, CREATE_SQL as SYS_SQL_CREATE_SQL};