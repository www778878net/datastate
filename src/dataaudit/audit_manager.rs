//! AuditManager - 审计日志管理器
//!
//! 职责：管理全局的AuditLogDataState单例，提供统一的审计日志记录接口

use once_cell::sync::Lazy;
use std::sync::Mutex;
use super::audit_log_state::AuditLogDataState;
use base::mylogger;

/// 全局审计日志管理器
static AUDIT_MANAGER: Lazy<Mutex<AuditManager>> = Lazy::new(|| {
    Mutex::new(AuditManager::new())
});

/// AuditManager - 审计日志管理器
pub struct AuditManager {
    audit_log_state: AuditLogDataState,
    logger: Arc<base::mylogger::MyLogger>,
}

impl AuditManager {
    /// 创建新的AuditManager实例
    fn new() -> Self {
        let audit_log_state = AuditLogDataState::new();
        let logger = mylogger!();
        
        // 初始化表
        if let Err(e) = audit_log_state.init_table() {
            logger.error(&format!("初始化审计日志表失败：{}", e));
        }
        Self { audit_log_state, logger }
    }

    /// 记录审计日志（计数方式）
    ///
    /// 如果当天已有记录，则增加计数；否则创建新记录
    pub fn log_audit(
        &self,
        tablename: &str,
        ability: &str,
        caller: &str,
    ) -> Result<(), String> {
        self.audit_log_state.log_audit(tablename, ability, caller)
    }

    /// 获取审计日志
    pub fn get_audit_logs(
        &self,
        tablename: Option<&str>,
        days: i32,
    ) -> Vec<super::audit_log_state::AuditLogRecord> {
        self.audit_log_state.get_audit_logs(tablename, days)
    }

    /// 获取指定日期范围的统计
    pub fn get_stats_by_date_range(
        &self,
        start_date: &str,
        end_date: &str,
    ) -> Vec<super::audit_log_state::AuditLogRecord> {
        self.audit_log_state.get_stats_by_date_range(start_date, end_date)
    }
}

/// 记录审计日志（全局函数）
///
/// 使用全局AuditLogDataState记录审计日志（计数方式）
pub fn log_audit(
    tablename: &str,
    ability: &str,
    caller: &str,
) -> Result<(), String> {
    let manager = AUDIT_MANAGER.lock().map_err(|e| e.to_string())?;
    manager.log_audit(tablename, ability, caller)
}

/// 获取审计日志（全局函数）
pub fn get_audit_logs(
    tablename: Option<&str>,
    days: i32,
) -> Vec<super::audit_log_state::AuditLogRecord> {
    let manager = AUDIT_MANAGER.lock().unwrap();
    manager.get_audit_logs(tablename, days)
}

/// 获取指定日期范围的统计（全局函数）
pub fn get_stats_by_date_range(
    start_date: &str,
    end_date: &str,
) -> Vec<super::audit_log_state::AuditLogRecord> {
    let manager = AUDIT_MANAGER.lock().unwrap();
    manager.get_stats_by_date_range(start_date, end_date)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_audit() {
        let result = log_audit("testtb", "getone", "TestTb");
        assert!(result.is_ok(), "记录审计日志应该成功");
    }

    #[test]
    fn test_get_audit_logs() {
        let logs = get_audit_logs(Some("testtb"), 7);
        // 不应该panic
        assert!(logs.len() >= 0);
    }
}