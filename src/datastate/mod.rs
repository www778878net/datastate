//! DataState - 数据状态机模块
//!
//! 单表状态机，管理数据库表的同步状态
//!
//! 提供两个版本：
//! - DataState: SQLite 版本（默认，本地优先）
//! - DataStateMysql: MySQL 版本（服务器端）

mod data_state;
mod data_state_mysql;
mod testtb;

// 引入 data_sync 模块（从父级模块导入）
pub use crate::data_sync;

// SQLite 版本导出
pub use data_state::DataState;
pub use testtb::{TestTb, TestTbRecord, TESTTB_CREATE_SQL};

// MySQL 版本导出
pub use data_state_mysql::DataStateMysql;

// 从 data_sync 重新导出（SQLite 版本）
pub use crate::data_sync::{
    DataSync, SynclogItem, SyncStats, StateLog, SyncData, SyncResult,
    SYNCLOG_CREATE_SQL, DATA_STATE_LOG_CREATE_SQL, DATA_SYNC_STATS_CREATE_SQL,
    add_to_synclog, get_pending_count, get_pending_items,
    log_status_change, get_status_logs, update_sync_stats, get_sync_stats,
    // Protobuf 结构
    ProtoSynclogItem, ProtoSynclogBatch,
};

// MySQL 版本重新导出
pub use crate::data_sync::{
    DataSyncMysql, SynclogItemMysql, SyncStatsMysql, StateLogMysql,
    SyncResultMysql, SyncDataMysql,
    SYNCLOG_CREATE_SQL_MYSQL, DATA_STATE_LOG_CREATE_SQL_MYSQL, DATA_SYNC_STATS_CREATE_SQL_MYSQL,
};

// 审计相关（从 dataaudit 重新导出）
pub use crate::dataaudit::{
    DataAudit, DATA_ABILITY_LOG_CREATE_SQL, AbilityLog,
    AuditLogDataState, AuditLogRecord, AUDIT_LOG_CREATE_SQL,
    AuditPermDataState, AuditPermRecord, AUDIT_PERM_CREATE_SQL,
};
