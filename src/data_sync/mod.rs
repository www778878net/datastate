//! DataSync - 同步组件
//!
//! 职责：同步队列管理、状态变更日志、同步统计
//! 参考 DataAudit 组件模式设计

mod data_sync;

pub use data_sync::{
    DataSync, SyncQueueItem, SyncStats, StateLog,
    SYNC_QUEUE_CREATE_SQL, DATA_STATE_LOG_CREATE_SQL, DATA_SYNC_STATS_CREATE_SQL,
    SyncResult, SyncData,
    // 独立函数
    add_to_sync_queue, get_pending_count, get_pending_items,
    log_status_change, get_status_logs, update_sync_stats, get_sync_stats,
};

// 别名：DataSyncQueue = DataSync
pub use data_sync::DataSync as DataSyncQueue;