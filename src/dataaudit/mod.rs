//! DataAudit - 数据层审计组件
//!
//! 职责：权限表初始化、权限检查、审计日志

mod data_audit;
mod audit_log_state;
mod audit_perm_state;

pub use data_audit::{
    DataAudit, DATASTATE_AUDIT_CREATE_SQL, DATA_ABILITY_PERM_CREATE_SQL, DATA_ABILITY_LOG_CREATE_SQL,
    DATA_ABILITY_DAILY_CREATE_SQL, AbilityPerm, AbilityLog, AbilityDaily,
    register_ability, register_ability_simple, check_ability_permission,
    get_ability_perm, list_abilities, log_ability_call, get_ability_logs,
    get_ability_daily_stats,
};

pub use audit_log_state::{
    AuditLogDataState, AuditLogRecord, AUDIT_LOG_CREATE_SQL,
};

pub use audit_perm_state::{
    AuditPermDataState, AuditPermRecord, AUDIT_PERM_CREATE_SQL,
};

pub fn get_audit_logs(tablename: Option<&str>, days: i32) -> Vec<AuditLogRecord> {
    let audit_log_state = AuditLogDataState::new();
    audit_log_state.get_audit_logs(tablename, days)
}

pub fn get_stats_by_date_range(start_date: &str, end_date: &str) -> Vec<AuditLogRecord> {
    let audit_log_state = AuditLogDataState::new();
    audit_log_state.get_stats_by_date_range(start_date, end_date)
}
