//! AuditLogDataState - 审计日志数据状态机
//!
//! 职责：将审计日志作为datastate管理，支持上传下载同步
//! 设计为单表状态机，每天每个调用方对每个方法只有一条记录（计数方式）

use crate::data_sync::DataSync;
use crate::state::BaseState;
use crate::sync_config::TableConfig;
use serde::{Deserialize, Serialize};

/// 审计日志表创建SQL
///
/// 设计原则：
/// - 只需要一个表，加上时间
/// - 不需要每次都添加一条记录，应该是计数次数
/// - summary 记录摘要：为什么调这个、修改了什么
pub const AUDIT_LOG_CREATE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS datastate_audit_log (
    idpk INTEGER PRIMARY KEY AUTOINCREMENT,
    tablename TEXT NOT NULL,          -- 表名
    ability TEXT NOT NULL,            -- 方法名
    caller TEXT NOT NULL,             -- 调用方
    stat_date TEXT NOT NULL,          -- 统计日期 YYYY-MM-DD
    call_count INTEGER NOT NULL DEFAULT 1,  -- 调用次数
    last_call_time TEXT NOT NULL,     -- 最后调用时间 ISO8601
    summary TEXT NOT NULL DEFAULT '', -- 摘要：为什么调这个、修改了什么
    UNIQUE(tablename, ability, caller, stat_date)
)
"#;

/// 审计日志记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogRecord {
    pub idpk: i64,
    pub tablename: String,
    pub ability: String,
    pub caller: String,
    pub stat_date: String,
    pub call_count: i64,
    pub last_call_time: String,
    pub summary: String,              // 摘要
}

/// AuditLogDataState - 审计日志数据状态机
///
/// 作为datastate管理审计日志表，支持同步功能
#[derive(Clone, Serialize, Deserialize)]
pub struct AuditLogDataState {
    /// 基础状态
    #[serde(flatten)]
    pub base: BaseState,

    /// 同步组件（包含数据库实例）
    #[serde(skip)]
    pub datasync: DataSync,
}

impl AuditLogDataState {
    /// 创建 AuditLogDataState 实例
    pub fn new() -> Self {
        Self {
            base: BaseState::new("datastate_audit_log"),
            datasync: DataSync::new("datastate_audit_log"),
        }
    }

    /// 从配置创建
    pub fn from_config(config: &TableConfig) -> Self {
        Self {
            base: BaseState::new(&config.name),
            datasync: DataSync::from_config(config),
        }
    }

    /// 初始化表
    pub fn init_table(&self) -> Result<(), String> {
        let conn = self.datasync.db.get_conn();
        let conn_guard = conn.lock().map_err(|e| e.to_string())?;

        conn_guard
            .execute(AUDIT_LOG_CREATE_SQL, [])
            .map_err(|e| format!("创建审计日志表失败：{}", e))?;

        Ok(())
    }

    /// 记录审计日志（计数方式）
    ///
    /// 如果当天已有记录，则增加计数；否则创建新记录
    /// 
    /// # Arguments
    /// * `tablename` - 表名
    /// * `ability` - 方法名
    /// * `caller` - 调用方
    /// * `summary` - 摘要：为什么调这个、修改了什么
    pub fn log_audit(
        &self,
        tablename: &str,
        ability: &str,
        caller: &str,
        summary: &str,
    ) -> Result<(), String> {
        let stat_date = chrono::Local::now().format("%Y-%m-%d").to_string();
        let last_call_time = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();

        let conn = self.datasync.db.get_conn();
        let conn_guard = conn.lock().map_err(|e| e.to_string())?;

        // 确保表存在
        conn_guard
            .execute(AUDIT_LOG_CREATE_SQL, [])
            .map_err(|e| format!("创建审计日志表失败：{}", e))?;

        // 尝试插入新记录
        let insert_sql = r#"
            INSERT INTO datastate_audit_log (tablename, ability, caller, stat_date, call_count, last_call_time, summary)
            VALUES (?, ?, ?, ?, 1, ?, ?)
        "#;

        let result = conn_guard.execute(
            insert_sql,
            rusqlite::params![tablename, ability, caller, stat_date, last_call_time, summary],
        );

        match result {
            Ok(_) => Ok(()), // 插入成功
            Err(rusqlite::Error::SqliteFailure(err, _)) if err.code == rusqlite::ErrorCode::ConstraintViolation => {
                // 违反唯一约束，说明已存在记录，更新计数和摘要
                let update_sql = r#"
                    UPDATE datastate_audit_log
                    SET call_count = call_count + 1,
                        last_call_time = ?,
                        summary = ?
                    WHERE tablename = ? AND ability = ? AND caller = ? AND stat_date = ?
                "#;

                conn_guard
                    .execute(update_sql, rusqlite::params![last_call_time, summary, tablename, ability, caller, stat_date])
                    .map_err(|e| format!("更新审计日志计数失败：{}", e))?;

                Ok(())
            }
            Err(e) => Err(format!("记录审计日志失败：{}", e)),
        }
    }

    /// 获取审计日志
    pub fn get_audit_logs(
        &self,
        tablename: Option<&str>,
        days: i32,
    ) -> Vec<AuditLogRecord> {
        if let Some(table) = tablename {
            let sql = format!(
                "SELECT idpk, tablename, ability, caller, stat_date, call_count, last_call_time, summary 
                 FROM datastate_audit_log 
                 WHERE tablename = ? 
                 ORDER BY stat_date DESC LIMIT {}",
                days
            );
            match self.datasync.db.query(&sql, &[&table]) {
                Ok(results) => results
                    .iter()
                    .map(|row| AuditLogRecord {
                        idpk: row.get("idpk").and_then(|v| v.as_i64()).unwrap_or(0),
                        tablename: row.get("tablename").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        ability: row.get("ability").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        caller: row.get("caller").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        stat_date: row.get("stat_date").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        call_count: row.get("call_count").and_then(|v| v.as_i64()).unwrap_or(0),
                        last_call_time: row.get("last_call_time").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        summary: row.get("summary").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    })
                    .collect(),
                _ => Vec::new(),
            }
        } else {
            let sql = format!(
                "SELECT idpk, tablename, ability, caller, stat_date, call_count, last_call_time, summary 
                 FROM datastate_audit_log 
                 ORDER BY stat_date DESC LIMIT {}",
                days
            );
            match self.datasync.db.query(&sql, &[]) {
                Ok(results) => results
                    .iter()
                    .map(|row| AuditLogRecord {
                        idpk: row.get("idpk").and_then(|v| v.as_i64()).unwrap_or(0),
                        tablename: row.get("tablename").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        ability: row.get("ability").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        caller: row.get("caller").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        stat_date: row.get("stat_date").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        call_count: row.get("call_count").and_then(|v| v.as_i64()).unwrap_or(0),
                        last_call_time: row.get("last_call_time").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        summary: row.get("summary").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    })
                    .collect(),
                _ => Vec::new(),
            }
        }
    }

    /// 获取指定日期范围的统计
    pub fn get_stats_by_date_range(
        &self,
        start_date: &str,
        end_date: &str,
    ) -> Vec<AuditLogRecord> {
        let sql = r#"
            SELECT idpk, tablename, ability, caller, stat_date, call_count, last_call_time, summary 
            FROM datastate_audit_log 
            WHERE stat_date BETWEEN ? AND ?
            ORDER BY stat_date DESC
        "#;

        match self.datasync.db.query(sql, &[&start_date, &end_date]) {
            Ok(results) => results
                .iter()
                .map(|row| AuditLogRecord {
                    idpk: row.get("idpk").and_then(|v| v.as_i64()).unwrap_or(0),
                    tablename: row.get("tablename").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    ability: row.get("ability").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    caller: row.get("caller").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    stat_date: row.get("stat_date").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    call_count: row.get("call_count").and_then(|v| v.as_i64()).unwrap_or(0),
                    last_call_time: row.get("last_call_time").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    summary: row.get("summary").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                })
                .collect(),
            _ => Vec::new(),
        }
    }
}

impl Default for AuditLogDataState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_log_state_creation() {
        let state = AuditLogDataState::new();
        assert_eq!(state.base.name, "datastate_audit_log");
        assert_eq!(state.datasync.table_name, "datastate_audit_log");
    }

    #[test]
    fn test_audit_log_sql_validity() {
        // 验证SQL语法正确性
        let sql = AUDIT_LOG_CREATE_SQL;
        assert!(sql.contains("CREATE TABLE"));
        assert!(sql.contains("datastate_audit_log"));
        assert!(sql.contains("call_count"));
        assert!(sql.contains("last_call_time"));
        assert!(sql.contains("UNIQUE"));
    }
}