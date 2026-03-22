//! AuditLogDataState - 审计日志数据状态机
//!
//! 职责：将审计日志作为datastate管理，支持上传下载同步
//! 设计为单表状态机，每天每个调用方对每个方法只有一条记录（计数方式）

use crate::data_sync::DataSync;
use crate::state::BaseState;
use crate::sync_config::TableConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use serde_json::Value;

/// 审计日志表创建SQL
///
/// 唯一键：apisys + apimicro + apiobj + ability + caller
/// 统计：某个方法被某个调用方调用了多少次
pub const AUDIT_LOG_CREATE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS data_audit_log (
    cid TEXT NOT NULL DEFAULT '',
    apisys TEXT NOT NULL DEFAULT '',
    apimicro TEXT NOT NULL DEFAULT '',
    apiobj TEXT NOT NULL DEFAULT '',
    ability TEXT NOT NULL DEFAULT '',
    caller TEXT NOT NULL DEFAULT '',
    num INTEGER NOT NULL DEFAULT 0,
    dlong INTEGER NOT NULL DEFAULT 0,
    downlen INTEGER NOT NULL DEFAULT 0,
    id TEXT NOT NULL DEFAULT '',
    upby TEXT NOT NULL DEFAULT '',
    uptime TEXT NOT NULL DEFAULT '',
    idpk INTEGER PRIMARY KEY AUTOINCREMENT,
    UNIQUE(apisys, apimicro, apiobj, ability, caller),
    UNIQUE(id)
)
"#;

/// 审计日志记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogRecord {
    pub cid: String,
    pub apisys: String,
    pub apimicro: String,
    pub apiobj: String,
    pub ability: String,
    pub caller: String,
    pub num: i64,
    pub dlong: i64,
    pub downlen: i64,
    pub id: String,
    pub upby: String,
    pub uptime: String,
    pub idpk: i64,
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

    /// 公司ID
    pub cid: String,
    /// 操作者
    pub upby: String,
}

impl AuditLogDataState {
    /// 创建 AuditLogDataState 实例
    pub fn new() -> Self {
        let config = crate::localdb::LocalDBConfig::default();
        Self {
            base: BaseState::new("data_audit_log"),
            datasync: DataSync::new("data_audit_log"),
            cid: config.cid,
            upby: config.upby,
        }
    }

    /// 从配置创建
    pub fn from_config(config: &TableConfig) -> Self {
        let db_config = crate::localdb::LocalDBConfig::default();
        Self {
            base: BaseState::new(&config.name),
            datasync: DataSync::from_config(config),
            cid: db_config.cid,
            upby: db_config.upby,
        }
    }

    /// 设置公司ID和操作者
    pub fn set_context(&mut self, cid: &str, upby: &str) {
        self.cid = cid.to_string();
        self.upby = upby.to_string();
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

    /// 记录审计日志
    ///
    /// 唯一键：apisys + apimicro + apiobj + ability + caller
    /// 统计：某个方法被某个调用方调用了多少次
    /// cid 和 upby 从配置读取
    pub fn log_audit(
        &self,
        apisys: &str,
        apimicro: &str,
        apiobj: &str,
        ability: &str,
        caller: &str,
        elapsed_ms: i64,
    ) -> Result<(), String> {
        let uptime = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
        let id = crate::snowflake::next_id_string();

        let conn = self.datasync.db.get_conn();
        let conn_guard = conn.lock().map_err(|e| e.to_string())?;

        // 确保表存在
        conn_guard
            .execute(AUDIT_LOG_CREATE_SQL, [])
            .map_err(|e| format!("创建审计日志表失败：{}", e))?;

        // 尝试插入新记录
        let insert_sql = r#"
            INSERT INTO data_audit_log 
                (id, cid, apisys, apimicro, apiobj, ability, caller, num, dlong, downlen, upby, uptime)
            VALUES (?, ?, ?, ?, ?, ?, ?, 1, ?, 0, ?, ?)
        "#;

        let insert_result = conn_guard.execute(
            insert_sql,
            rusqlite::params![id, self.cid, apisys, apimicro, apiobj, ability, caller, elapsed_ms, self.upby, uptime],
        );

        match insert_result {
            Ok(_) => Ok(()), // 插入成功
            Err(rusqlite::Error::SqliteFailure(err, _)) if err.code == rusqlite::ErrorCode::ConstraintViolation => {
                // 违反唯一约束，更新计数
                let update_sql = r#"
                    UPDATE data_audit_log
                    SET num = num + 1,
                        dlong = dlong + ?,
                        uptime = ?
                    WHERE apisys = ? AND apimicro = ? AND apiobj = ? AND ability = ? AND caller = ?
                "#;

                conn_guard
                    .execute(update_sql, rusqlite::params![elapsed_ms, uptime, apisys, apimicro, apiobj, ability, caller])
                    .map_err(|e| format!("更新审计日志计数失败：{}", e))?;

                Ok(())
            }
            Err(e) => Err(format!("记录审计日志失败：{}", e)),
        }
    }

    /// 获取审计日志
    pub fn get_audit_logs(
        &self,
        apiobj: Option<&str>,
        days: i32,
    ) -> Vec<AuditLogRecord> {
        let fields = "cid, apisys, apimicro, apiobj, ability, caller, num, dlong, downlen, id, upby, uptime, idpk";
        
        let map_row = |row: &HashMap<String, Value>| AuditLogRecord {
            cid: row.get("cid").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            apisys: row.get("apisys").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            apimicro: row.get("apimicro").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            apiobj: row.get("apiobj").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            ability: row.get("ability").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            caller: row.get("caller").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            num: row.get("num").and_then(|v| v.as_i64()).unwrap_or(0),
            dlong: row.get("dlong").and_then(|v| v.as_i64()).unwrap_or(0),
            downlen: row.get("downlen").and_then(|v| v.as_i64()).unwrap_or(0),
            id: row.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            upby: row.get("upby").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            uptime: row.get("uptime").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            idpk: row.get("idpk").and_then(|v| v.as_i64()).unwrap_or(0),
        };

        if let Some(obj) = apiobj {
            let sql = format!(
                "SELECT {} FROM data_audit_log WHERE apiobj = ? ORDER BY uptime DESC LIMIT {}",
                fields, days
            );
            match self.datasync.db.query(&sql, &[&obj]) {
                Ok(results) => results.iter().map(map_row).collect(),
                _ => Vec::new(),
            }
        } else {
            let sql = format!(
                "SELECT {} FROM data_audit_log ORDER BY uptime DESC LIMIT {}",
                fields, days
            );
            match self.datasync.db.query(&sql, &[]) {
                Ok(results) => results.iter().map(map_row).collect(),
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
        let fields = "cid, apisys, apimicro, apiobj, ability, caller, num, dlong, downlen, id, upby, uptime, idpk";
        let sql = format!(
            "SELECT {} FROM data_audit_log WHERE uptime BETWEEN ? AND ? ORDER BY uptime DESC",
            fields
        );

        match self.datasync.db.query(&sql, &[&start_date, &end_date]) {
            Ok(results) => results
                .iter()
                .map(|row| AuditLogRecord {
                    cid: row.get("cid").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    apisys: row.get("apisys").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    apimicro: row.get("apimicro").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    apiobj: row.get("apiobj").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    ability: row.get("ability").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    caller: row.get("caller").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    num: row.get("num").and_then(|v| v.as_i64()).unwrap_or(0),
                    dlong: row.get("dlong").and_then(|v| v.as_i64()).unwrap_or(0),
                    downlen: row.get("downlen").and_then(|v| v.as_i64()).unwrap_or(0),
                    id: row.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    upby: row.get("upby").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    uptime: row.get("uptime").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    idpk: row.get("idpk").and_then(|v| v.as_i64()).unwrap_or(0),
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
    use base::mylogger::mylogger;

    #[test]
    fn test_audit_log_state_creation() {
        let logger = mylogger!();
        let state = AuditLogDataState::new();
        assert_eq!(state.base.name, "data_audit_log");
        assert_eq!(state.datasync.table_name, "data_audit_log");
    }

    #[test]
    fn test_audit_log_sql_validity() {
        let logger = mylogger!();
        let sql = AUDIT_LOG_CREATE_SQL;
        assert!(sql.contains("CREATE TABLE"));
        assert!(sql.contains("data_audit_log"));
        assert!(sql.contains("ability"));
        assert!(sql.contains("caller"));
        assert!(sql.contains("num"));
        assert!(sql.contains("dlong"));
        assert!(sql.contains("UNIQUE"));
    }

    #[test]
    fn test_log_audit() {
        let logger = mylogger!();

        let state = AuditLogDataState::new();

        // 第一次记录
        let result = state.log_audit(
            "local",
            "datastate",
            "testtb",
            "msave",
            "inventory",
            150,
        );
        if result.is_err() {
            logger.error(&format!("第一次记录失败: {:?}", result));
        }
        assert!(result.is_ok(), "第一次记录失败: {:?}", result);

        // 第二次记录（相同唯一键，应该更新计数）
        let result = state.log_audit(
            "local",
            "datastate",
            "testtb",
            "msave",
            "inventory",
            200,
        );
        if result.is_err() {
            logger.error(&format!("第二次记录失败: {:?}", result));
        }
        assert!(result.is_ok(), "第二次记录失败: {:?}", result);

        // 验证日志记录
        let logs = state.get_audit_logs(Some("testtb"), 10);
        if logs.is_empty() {
            logger.error("日志记录为空");
        }
        assert!(!logs.is_empty(), "日志记录为空");

        let log = &logs[0];
        assert_eq!(log.apiobj, "testtb");
        assert_eq!(log.apisys, "local");
        assert_eq!(log.apimicro, "datastate");
        assert_eq!(log.ability, "msave");
        assert_eq!(log.caller, "inventory");
        assert_eq!(log.num, 2); // 两次调用
        assert_eq!(log.dlong, 350); // 150 + 200
    }
}