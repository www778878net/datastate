//! DataState - 数据状态机模块
//!
//! 单表状态机，管理数据库表的同步状态

mod data_state;
mod testtb;

// 引入 data_sync 模块（从父级模块导入）
pub use crate::data_sync;

pub use data_state::DataState;
pub use testtb::{TestTb, TestTbRecord, TESTTB_CREATE_SQL};

// 从 data_sync 重新导出
pub use crate::data_sync::{
    DataSync, SyncQueueItem, SyncStats, StateLog, SyncData, SyncResult,
    SYNC_QUEUE_CREATE_SQL, DATA_STATE_LOG_CREATE_SQL, DATA_SYNC_STATS_CREATE_SQL,
    add_to_sync_queue, get_pending_count, get_pending_items,
    log_status_change, get_status_logs, update_sync_stats, get_sync_stats,
};

// 权限相关方法（从 dataaudit 重新导出，方便使用）
// 仅保留能力层权限（单一表模型）
pub use crate::dataaudit::{
    // 能力权限（微服务对微服务，精确到函数级别）
    register_ability, register_ability_simple, check_ability_permission,
    get_ability_perm, list_abilities, log_ability_call, get_ability_logs,
    get_ability_daily_stats, DATASTATE_AUDIT_CREATE_SQL, DATA_ABILITY_PERM_CREATE_SQL, DATA_ABILITY_LOG_CREATE_SQL,
    DATA_ABILITY_DAILY_CREATE_SQL, AbilityPerm, AbilityLog, AbilityDaily,
};

// 新的DataState组件（审计日志和权限表作为datastate管理）
pub use crate::dataaudit::{
    AuditLogDataState, AuditLogRecord, AUDIT_LOG_CREATE_SQL,
    AuditPermDataState, AuditPermRecord, AUDIT_PERM_CREATE_SQL,
};
