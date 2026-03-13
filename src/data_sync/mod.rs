//! DataSync - 同步组件
//!
//! 职责：同步日志管理、状态变更日志、同步统计
//! 参考 DataAudit 组件模式设计

mod data_sync;

pub use data_sync::{
    DataSync, SynclogItem, SyncStats, StateLog,
    SYNCLOG_CREATE_SQL, DATA_STATE_LOG_CREATE_SQL, DATA_SYNC_STATS_CREATE_SQL,
    SyncResult, SyncData, SyncValidationError,
    add_to_synclog, get_pending_count, get_pending_items,
    log_status_change, get_status_logs, update_sync_stats, get_sync_stats,
};

pub use data_sync::DataSync as DataSyncQueue;