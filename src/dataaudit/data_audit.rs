//! DataAudit - 数据层基础组件
//!
//! 职责：审计日志记录
//! 权限检查由具体数据服务自己控制（写死在函数中）

use crate::localdb::LocalDB;
use serde::{Deserialize, Serialize};

/// data_ability_log 表 - 能力调用日志
pub const DATA_ABILITY_LOG_CREATE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS data_ability_log (
    idpk INTEGER PRIMARY KEY AUTOINCREMENT,
    ability_name TEXT NOT NULL,
    caller TEXT NOT NULL DEFAULT '',
    action TEXT NOT NULL DEFAULT '',
    input_params TEXT NOT NULL DEFAULT '',
    created_at REAL NOT NULL DEFAULT (strftime('%s','now'))
)
"#;

/// 能力调用日志记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbilityLog {
    pub idpk: i64,
    pub ability_name: String,
    pub caller: String,
    pub action: String,
    pub input_params: String,
    pub created_at: f64,
}

/// DataAudit - 数据层基础组件
///
/// 职责：审计日志记录
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DataAudit {
    /// 表名
    pub tablename: String,
    /// 是否开启审计
    pub audit_enabled: bool,
    /// 数据库连接
    #[serde(skip)]
    pub db: LocalDB,
}

impl DataAudit {
    /// 创建 DataAudit 实例（默认开启审计）
    pub fn new(tablename: &str) -> Self {
        let db = LocalDB::default();
        // 初始化审计日志表
        let _ = Self::init_tables(&db);
        
        Self {
            tablename: tablename.to_string(),
            audit_enabled: true,
            db,
        }
    }

    /// 初始化审计日志表
    pub async fn init_tables(db: &LocalDB) -> Result<(), String> {
        let conn = db.get_conn();
        let conn_guard = conn.lock().await;

        conn_guard
            .execute(DATA_ABILITY_LOG_CREATE_SQL, [])
            .map_err(|e| format!("创建审计日志表失败：{}", e))?;

        Ok(())
    }

    /// 记录审计日志
    ///
    /// # Arguments
    /// * `ability` - 方法名
    /// * `caller` - 调用方
    /// * `summary` - 操作摘要
    pub async fn check_permission(
        &self,
        ability: &str,
        caller: &str,
        summary: &str,
    ) -> Result<bool, String> {
        if !self.audit_enabled {
            return Ok(true);
        }

        let ability_full = format!("{}/{}", self.tablename, ability);
        let conn = self.db.get_conn();
        let conn_guard = conn.lock().await;

        let log_sql = "INSERT INTO data_ability_log (ability_name, caller, action, input_params) VALUES (?, ?, ?, ?)";
        conn_guard
            .execute(log_sql, rusqlite::params![ability_full, caller, ability, summary])
            .map_err(|e| format!("记录审计日志失败: {}", e))?;

        Ok(true)
    }

    /// 获取能力调用日志
    pub async fn get_ability_logs(db: &LocalDB, ability_name: &str, limit: i32) -> Vec<AbilityLog> {
        let sql = format!(
            "SELECT idpk, ability_name, caller, action, input_params, created_at FROM data_ability_log WHERE ability_name = ? ORDER BY created_at DESC LIMIT {}",
            limit
        );

        match db.query(&sql, &[&ability_name as &dyn rusqlite::ToSql]).await {
            Ok(results) => results
                .iter()
                .map(|row| AbilityLog {
                    idpk: row.get("idpk").and_then(|v: &serde_json::Value| v.as_i64()).unwrap_or(0),
                    ability_name: row.get("ability_name").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("").to_string(),
                    caller: row.get("caller").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("").to_string(),
                    action: row.get("action").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("").to_string(),
                    input_params: row.get("input_params").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("").to_string(),
                    created_at: row.get("created_at").and_then(|v| v.as_f64()).unwrap_or(0.0),
                })
                .collect(),
            _ => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_audit_new() {
        let audit = DataAudit::new("testtb");
        assert_eq!(audit.tablename, "testtb");
        assert!(audit.audit_enabled);
    }
}
