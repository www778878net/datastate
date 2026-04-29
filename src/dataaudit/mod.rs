//! DataAudit - 数据层审计组件
//!
//! 职责：审计日志记录
//! 权限检查由具体数据服务自己控制（写死在函数中）

mod data_audit;
mod audit_log_state;
mod audit_perm_state;

pub use data_audit::{DataAudit, DATA_ABILITY_LOG_CREATE_SQL, AbilityLog};

pub use audit_log_state::{AuditLogDataState, AuditLogRecord, AUDIT_LOG_CREATE_SQL};

pub use audit_perm_state::{AuditPermDataState, AuditPermRecord, AUDIT_PERM_CREATE_SQL};

pub async fn get_audit_logs(tablename: Option<&str>, days: i32) -> Vec<AuditLogRecord> {
    let audit_log_state = AuditLogDataState::new();
    audit_log_state.get_audit_logs(tablename, days).await
}

pub async fn get_stats_by_date_range(start_date: &str, end_date: &str) -> Vec<AuditLogRecord> {
    let audit_log_state = AuditLogDataState::new();
    audit_log_state.get_stats_by_date_range(start_date, end_date).await
}
