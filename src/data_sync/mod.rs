//! DataSync - 同步组件
//!
//! 职责：同步日志管理、状态变更日志、同步统计
//! 参考 DataAudit 组件模式设计
//!
//! 提供两个版本：
//! - DataSync: SQLite 版本（默认）
//! - DataSyncMysql: MySQL 版本

mod data_sync;
pub mod data_sync_mysql;

// SQLite 版本导出
pub use data_sync::{
    DataSync, SynclogItem, SyncStats, StateLog,
    SYNCLOG_CREATE_SQL, DATA_STATE_LOG_CREATE_SQL, DATA_SYNC_STATS_CREATE_SQL,
    SyncResult, SyncData, SyncValidationError,
    add_to_synclog, get_pending_count, get_pending_items,
    log_status_change, get_status_logs, update_sync_stats, get_sync_stats,
    // Protobuf 结构导出
    ProtoSynclogItem, ProtoSynclogBatch,
};

pub use data_sync::DataSync as DataSyncQueue;

// MySQL 版本导出
pub use data_sync_mysql::{
    DataSyncMysql, SynclogItemMysql, SyncStatsMysql, StateLogMysql,
    SyncResultMysql, SyncDataMysql,
    SYNCLOG_CREATE_SQL_MYSQL, DATA_STATE_LOG_CREATE_SQL_MYSQL, DATA_SYNC_STATS_CREATE_SQL_MYSQL,
};