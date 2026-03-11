//! DataAudit - 数据层基础组件
//!
//! 职责：权限表初始化、权限检查、审计日志
//! 设计为组合组件，被具体表实现（如 TestTb）组合使用

use crate::localdb::LocalDB;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

/// datastate_audit 表 - 权限表
///
/// 字段：表名 + 方法名 + 调用方
/// 用于注册哪个调用方可以访问哪个表的哪个方法
pub const DATASTATE_AUDIT_CREATE_SQL: &str = r#"
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

/// 旧常量别名（兼容）
pub const DATA_ABILITY_PERM_CREATE_SQL: &str = DATASTATE_AUDIT_CREATE_SQL;

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

/// data_ability_daily 表 - 能力调用每日唯一值
pub const DATA_ABILITY_DAILY_CREATE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS data_ability_daily (
    idpk INTEGER PRIMARY KEY AUTOINCREMENT,
    ability_name TEXT NOT NULL,
    caller TEXT NOT NULL DEFAULT '',
    action TEXT NOT NULL DEFAULT '',
    input_hash TEXT NOT NULL DEFAULT '',
    stat_date TEXT NOT NULL,
    UNIQUE(ability_name, caller, input_hash, stat_date)
)
"#;

/// 能力权限记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbilityPerm {
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

/// 能力每日唯一调用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbilityDaily {
    pub idpk: i64,
    pub ability_name: String,
    pub caller: String,
    pub action: String,
    pub stat_date: String,
}

/// DataAudit - 数据层基础组件
///
/// 职责：
/// - 权限表初始化（只执行一次）
/// - 权限检查（核心方法：check_permission）
/// - 审计日志记录
///
/// 使用方式：
/// ```ignore
/// struct TestTb {
///     pub audit: DataAudit,
///     pub state: DataState,
/// }
///
/// impl TestTb {
///     pub fn getone(&self, id: &str, caller: &str, summary: &str) -> Result<Record, String> {
///         self.audit.check_permission("getone", caller, summary)?;
///         // ... 业务逻辑
///     }
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub struct DataAudit {
    /// 表名（DataState 标识）
    pub tablename: String,
    /// 是否开启审计（每个表独立控制）
    pub audit_enabled: bool,
    /// 数据库连接
    #[serde(skip)]
    pub db: LocalDB,
}

impl DataAudit {
    /// 创建 DataAudit 实例（默认开启审计）
    pub fn new(tablename: &str) -> Self {
        Self {
            tablename: tablename.to_string(),
            audit_enabled: true,
            db: LocalDB::default(),
        }
    }

    /// 设置审计开关
    pub fn set_audit_enabled(&mut self, enabled: bool) {
        self.audit_enabled = enabled;
    }

    /// 初始化权限相关表（只执行一次）
    /// 在应用启动时调用
    pub fn init_tables(db: &LocalDB) -> Result<(), String> {
        let conn = db.get_conn();
        let conn_guard = conn.lock().map_err(|e| e.to_string())?;

        conn_guard
            .execute(DATA_ABILITY_PERM_CREATE_SQL, [])
            .map_err(|e| format!("创建权限表失败：{}", e))?;

        conn_guard
            .execute(DATA_ABILITY_LOG_CREATE_SQL, [])
            .map_err(|e| format!("创建能力日志表失败：{}", e))?;

        conn_guard
            .execute(DATA_ABILITY_DAILY_CREATE_SQL, [])
            .map_err(|e| format!("创建每日统计表失败：{}", e))?;

        Ok(())
    }

    /// 注册能力权限
    pub fn register_ability(
        db: &LocalDB,
        tablename: &str,
        ability: &str,
        caller: &str,
        description: &str,
    ) -> Result<(), String> {
        let id = Uuid::new_v4().to_string();
        let sql = "INSERT OR REPLACE INTO datastate_audit (id, tablename, ability, caller, description, upby, cid, uid, uptime) VALUES (?, ?, ?, ?, ?, '', '', '', '')";

        let conn = db.get_conn();
        let conn_guard = conn.lock().map_err(|e| e.to_string())?;

        conn_guard
            .execute(sql, rusqlite::params![id, tablename, ability, caller, description])
            .map_err(|e| format!("注册能力权限失败：{}", e))?;

        Ok(())
    }

    /// 验证能力权限（核心方法）
    ///
    /// 检查流程：
    /// 1. 审计关闭 → 自动放行
    /// 2. 本表内部调用 → 自动放行
    /// 3. 能力层权限检查（支持 ability=* 通配符） → 有权限则放行
    /// 4. 拒绝
    ///
    /// 权限检查通过后会记录审计日志
    ///
    /// # Arguments
    /// * `ability` - 方法名
    /// * `caller` - 调用方的包名
    /// * `summary` - 操作摘要（记录到审计日志）
    ///
    /// # Returns
    /// * `Ok(true)` - 有权限或审计关闭
    /// * `Err(msg)` - 无权限或未注册
    pub fn check_permission(
        &self,
        ability: &str,
        caller: &str,
        summary: &str,
    ) -> Result<bool, String> {
        if !self.audit_enabled {
            return Ok(true);
        }

        if &self.tablename == caller {
            let _ = Self::log_audit(&self.db, &self.tablename, ability, caller, summary);
            return Ok(true);
        }

        let conn = self.db.get_conn();
        let conn_guard = conn.lock().map_err(|e| e.to_string())?;

        let sql = "SELECT 1 FROM datastate_audit 
                   WHERE tablename = ? AND caller = ? 
                   AND (ability = ? OR ability = '*')";
        
        let result: Result<i32, _> = conn_guard.query_row(
            sql,
            rusqlite::params![&self.tablename, caller, ability],
            |_| Ok(1),
        );

        match result {
            Ok(_) => {
                let _ = Self::log_audit(&self.db, &self.tablename, ability, caller, summary);
                Ok(true)
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                Err(format!("[{}] 无权调用 [{}/{}]", caller, &self.tablename, ability))
            }
            Err(e) => Err(format!("查询权限失败：{}", e)),
        }
    }

    /// 记录审计日志（私有方法）
    fn log_audit(
        db: &LocalDB,
        tablename: &str,
        ability: &str,
        caller: &str,
        summary: &str,
    ) -> Result<(), String> {
        let ability_full = format!("{}/{}", tablename, ability);
        let conn = db.get_conn();
        let conn_guard = conn.lock().map_err(|e| e.to_string())?;

        let log_sql = "INSERT INTO data_ability_log (ability_name, caller, action, input_params) VALUES (?, ?, ?, ?)";
        conn_guard
            .execute(log_sql, rusqlite::params![ability_full, caller, ability, summary])
            .map_err(|e| format!("记录审计日志失败: {}", e))?;

        Ok(())
    }

    /// 获取能力权限信息
    pub fn get_ability_perm(db: &LocalDB, tablename: &str, ability: &str) -> Option<AbilityPerm> {
        let sql = "SELECT idpk, id, tablename, ability, caller, description, upby, cid, uid, uptime FROM datastate_audit WHERE tablename = ? AND ability = ?";

        match db.query(sql, &[&tablename as &dyn rusqlite::ToSql, &ability]) {
            Ok(results) => {
                let rows: Vec<AbilityPerm> = results
                    .iter()
                    .map(|row| AbilityPerm {
                        idpk: row.get("idpk").and_then(|v| v.as_i64()).unwrap_or(0),
                        id: row.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        tablename: row.get("tablename").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        ability: row.get("ability").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        caller: row.get("caller").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        description: row.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        upby: row.get("upby").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        cid: row.get("cid").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        uid: row.get("uid").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        uptime: row.get("uptime").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    })
                    .collect();
                rows.into_iter().next()
            }
            _ => None,
        }
    }

    /// 列出所有已注册的能力
    pub fn list_abilities(db: &LocalDB) -> Vec<AbilityPerm> {
        let sql = "SELECT idpk, id, tablename, ability, caller, description, upby, cid, uid, uptime FROM datastate_audit ORDER BY idpk DESC";

        match db.query(sql, &[]) {
            Ok(results) => results
                .iter()
                .map(|row| AbilityPerm {
                    idpk: row.get("idpk").and_then(|v| v.as_i64()).unwrap_or(0),
                    id: row.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    tablename: row.get("tablename").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    ability: row.get("ability").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    caller: row.get("caller").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    description: row.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    upby: row.get("upby").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    cid: row.get("cid").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    uid: row.get("uid").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    uptime: row.get("uptime").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    /// 获取能力调用日志
    pub fn get_ability_logs(db: &LocalDB, ability_name: &str, limit: i32) -> Vec<AbilityLog> {
        let sql = format!(
            "SELECT idpk, ability_name, caller, action, input_params, created_at FROM data_ability_log WHERE ability_name = ? ORDER BY created_at DESC LIMIT {}",
            limit
        );

        match db.query(&sql, &[&ability_name as &dyn rusqlite::ToSql]) {
            Ok(results) => results
                .iter()
                .map(|row| AbilityLog {
                    idpk: row.get("idpk").and_then(|v| v.as_i64()).unwrap_or(0),
                    ability_name: row.get("ability_name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    caller: row.get("caller").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    action: row.get("action").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    input_params: row.get("input_params").and_then(|v| v.as_str()).unwrap_or("").to_string(),
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
