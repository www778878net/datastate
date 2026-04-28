//! AuditPermDataState - 权限表数据状态机
//!
//! 职责：将权限表作为datastate管理，支持上传下载同步
//! 设计为单表状态机，管理哪些调用方可以访问哪些表的哪些方法

use crate::data_sync::DataSync;
use crate::state::BaseState;
use crate::sync_config::TableConfig;
use serde::{Deserialize, Serialize};

/// 权限表创建SQL
pub const AUDIT_PERM_CREATE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS datastate_audit (
    idpk INTEGER PRIMARY KEY AUTOINCREMENT,
    id TEXT NOT NULL,                 -- 业务主键 UUID
    tablename TEXT NOT NULL,          -- 表名（DataState 标识）
    ability TEXT NOT NULL,            -- 方法名（如 "getone", "mlist", "*" 表示全部）
    caller TEXT NOT NULL,             -- 允许调用的微服务名
    description TEXT NOT NULL DEFAULT '', -- 功能说明
    upby TEXT NOT NULL DEFAULT '',    -- 更新人
    cid TEXT NOT NULL DEFAULT '',     -- 创建者 ID
    uid TEXT NOT NULL DEFAULT '',     -- 用户 ID
    uptime TEXT NOT NULL DEFAULT '',  -- 更新时间（同步用）
    UNIQUE(tablename, ability, caller),
    UNIQUE(id)
)
"#;

/// 权限记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditPermRecord {
    pub idpk: i64,
    pub id: String,
    pub tablename: String,
    pub ability: String,
    pub caller: String,
    pub description: String,
    pub upby: String,
    pub cid: String,
    pub uid: String,
    pub uptime: String,
}

/// AuditPermDataState - 权限表数据状态机
///
/// 作为datastate管理权限表，支持同步功能
#[derive(Clone, Serialize, Deserialize)]
pub struct AuditPermDataState {
    /// 基础状态
    #[serde(flatten)]
    pub base: BaseState,

    /// 同步组件（包含数据库实例）
    #[serde(skip)]
    pub datasync: DataSync,
}

impl AuditPermDataState {
    /// 创建 AuditPermDataState 实例
    pub fn new() -> Self {
        Self {
            base: BaseState::new("datastate_audit"),
            datasync: DataSync::new("datastate_audit"),
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
    pub async fn init_table(&self) -> Result<(), String> {
        let conn = self.datasync.db.get_conn();
        let conn_guard = conn.lock().await;

        conn_guard
            .execute(AUDIT_PERM_CREATE_SQL, [])
            .map_err(|e| format!("创建权限表失败：{}", e))?;

        Ok(())
    }

    /// 注册能力权限
    pub async fn register_ability(
        &self,
        tablename: &str,
        ability: &str,
        caller: &str,
        description: &str,
    ) -> Result<(), String> {
        // 生成业务主键 id: 雪花算法
        let id = crate::snowflake::next_id_string();

        let sql = "REPLACE INTO datastate_audit (id, tablename, ability, caller, description, upby, cid, uid, uptime) VALUES (?, ?, ?, ?, ?, '', '', '', '')";

        let conn = self.datasync.db.get_conn();
        let conn_guard = conn.lock().await;

        conn_guard
            .execute(
                sql,
                rusqlite::params![id, tablename, ability, caller, description],
            )
            .map_err(|e| format!("注册能力权限失败：{}", e))?;

        Ok(())
    }

    /// 验证能力权限
    ///
    /// 检查流程：
    /// 1. 本表内部调用 → 自动放行
    /// 2. 能力层权限检查（支持 ability=* 通配符） → 有权限则放行
    /// 3. 拒绝
    pub async fn check_permission(
        &self,
        tablename: &str,
        ability: &str,
        caller: &str,
        audit_enabled: bool,
    ) -> Result<bool, String> {
        // 审计关闭时，不检查权限，直接返回
        if !audit_enabled {
            return Ok(true);
        }

        // 1. 本表内部调用 → 自动放行
        if tablename == caller {
            return Ok(true);
        }

        let conn = self.datasync.db.get_conn();
        let conn_guard = conn.lock().await;

        // 2. 能力层权限检查（支持 ability=* 通配符）
        let sql = "SELECT 1 FROM datastate_audit
                   WHERE tablename = ? AND caller = ?
                   AND (ability = ? OR ability = '*')";

        let result: Result<i32, _> = conn_guard.query_row(
            sql,
            rusqlite::params![tablename, caller, ability],
            |_| Ok(1),
        );

        match result {
            Ok(_) => Ok(true), // 有权限
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                Err(format!(
                    "[{}] 无权调用 [{}/{}]",
                    caller, tablename, ability
                ))
            }
            Err(e) => Err(format!("查询权限失败：{}", e)),
        }
    }

    /// 获取权限记录
    pub async fn get_permission(
        &self,
        tablename: &str,
        ability: &str,
    ) -> Option<AuditPermRecord> {
        let sql = "SELECT idpk, id, tablename, ability, caller, description, upby, cid, uid, uptime FROM datastate_audit WHERE tablename = ? AND ability = ?";

        match self.datasync.db.query(
            sql,
            &[&tablename as &dyn rusqlite::ToSql, &ability as &dyn rusqlite::ToSql],
        ).await {
            Ok(results) => {
                let rows: Vec<AuditPermRecord> = results
                    .iter()
                    .map(|row| AuditPermRecord {
                        idpk: row.get("idpk").and_then(|v: &serde_json::Value| v.as_i64()).unwrap_or(0),
                        id: row.get("id").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("").to_string(),
                        tablename: row.get("tablename").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("").to_string(),
                        ability: row.get("ability").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("").to_string(),
                        caller: row.get("caller").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("").to_string(),
                        description: row.get("description").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("").to_string(),
                        upby: row.get("upby").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("").to_string(),
                        cid: row.get("cid").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("").to_string(),
                        uid: row.get("uid").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("").to_string(),
                        uptime: row.get("uptime").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("").to_string(),
                    })
                    .collect();
                rows.into_iter().next()
            }
            _ => None,
        }
    }

    /// 列出所有权限
    pub async fn list_permissions(&self) -> Vec<AuditPermRecord> {
        let sql = "SELECT idpk, id, tablename, ability, caller, description, upby, cid, uid, uptime FROM datastate_audit ORDER BY idpk DESC";

        match self.datasync.db.query(sql, &[]).await {
            Ok(results) => results
                .iter()
                .map(|row| AuditPermRecord {
                    idpk: row.get("idpk").and_then(|v: &serde_json::Value| v.as_i64()).unwrap_or(0),
                    id: row.get("id").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("").to_string(),
                    tablename: row.get("tablename").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("").to_string(),
                    ability: row.get("ability").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("").to_string(),
                    caller: row.get("caller").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("").to_string(),
                    description: row.get("description").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("").to_string(),
                    upby: row.get("upby").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("").to_string(),
                    cid: row.get("cid").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("").to_string(),
                    uid: row.get("uid").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("").to_string(),
                    uptime: row.get("uptime").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("").to_string(),
                })
                .collect(),
            _ => Vec::new(),
        }
    }
}

impl Default for AuditPermDataState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_perm_state_creation() {
        let state = AuditPermDataState::new();
        assert_eq!(state.base.name, "datastate_audit");
        assert_eq!(state.datasync.table_name, "datastate_audit");
    }

    #[test]
    fn test_audit_perm_sql_validity() {
        // 验证SQL语法正确性
        let sql = AUDIT_PERM_CREATE_SQL;
        assert!(sql.contains("CREATE TABLE"));
        assert!(sql.contains("datastate_audit"));
        assert!(sql.contains("tablename"));
        assert!(sql.contains("ability"));
        assert!(sql.contains("caller"));
        assert!(sql.contains("UNIQUE"));
    }
}