//! DataAudit - 数据层基础组件
//!
//! 职责：权限表初始化、权限检查、审计日志
//! 设计为组合组件，被具体表实现（如 TestTb）组合使用

use crate::localdb::LocalDB;
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use chrono::{Local, Utc};

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

/// data_ability_log 表 - 能力调用日志（Claude钩子管理员日志）
///
/// 记录每次能力调用：能力名称、调用者、说明、输入参数
pub const DATA_ABILITY_LOG_CREATE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS data_ability_log (
    idpk INTEGER PRIMARY KEY AUTOINCREMENT,
    ability_name TEXT NOT NULL,
    caller TEXT NOT NULL DEFAULT '',
    action TEXT NOT NULL DEFAULT '',     -- 操作说明
    input_params TEXT NOT NULL DEFAULT '', -- JSON 输入参数
    created_at REAL NOT NULL DEFAULT (strftime('%s','now'))
)
"#;

/// data_ability_daily 表 - 能力调用每日唯一值
///
/// 每天保存一次唯一值，可查一个月（31天）
pub const DATA_ABILITY_DAILY_CREATE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS data_ability_daily (
    idpk INTEGER PRIMARY KEY AUTOINCREMENT,
    ability_name TEXT NOT NULL,
    caller TEXT NOT NULL DEFAULT '',
    action TEXT NOT NULL DEFAULT '',
    input_hash TEXT NOT NULL DEFAULT '',  -- 输入参数的 UUID
    stat_date TEXT NOT NULL,               -- YYYY-MM-DD
    UNIQUE(ability_name, caller, input_hash, stat_date)
)
"#;

/// 能力权限记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbilityPerm {
    pub idpk: i64,
    pub id: String,                // 业务主键 UUID
    pub tablename: String,         // 表名（DataState 标识）
    pub ability: String,           // 方法名（如 "getone", "mlist", "*" 表示全部）
    pub caller: String,            // 允许调用的微服务名
    pub description: String,       // 功能说明
    pub upby: String,              // 更新人
    pub cid: String,               // 创建者 ID
    pub uid: String,               // 用户 ID
    pub uptime: String,            // 更新时间（同步用）
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
/// - 权限检查
/// - 审计日志记录
///
/// 使用方式：
/// ```ignore
/// // 具体表实现时组合 DataAudit
/// struct TestTb {
///     pub audit: DataAudit,
///     pub state: DataState,
/// }
///
/// impl TestTb {
///     pub fn getone(&self, id: &str, caller: &str) -> Result<Record, String> {
///         audit!(self, "getone", caller, { ... })
///     }
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub struct DataAudit {
    /// 表名（DataState 标识）
    pub tablename: String,
    /// 是否开启审计（每个表独立控制）
    pub audit_enabled: bool,
}

impl DataAudit {
    /// 创建 DataAudit 实例（默认开启审计）
    pub fn new(tablename: &str) -> Self {
        Self {
            tablename: tablename.to_string(),
            audit_enabled: true,
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

        // 创建能力权限表
        conn_guard
            .execute(DATA_ABILITY_PERM_CREATE_SQL, [])
            .map_err(|e| format!("创建权限表失败：{}", e))?;

        // 创建能力调用日志表
        conn_guard
            .execute(DATA_ABILITY_LOG_CREATE_SQL, [])
            .map_err(|e| format!("创建能力日志表失败：{}", e))?;

        // 创建每日唯一值表
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
        // 生成业务主键 id: UUID
        let id = Uuid::new_v4().to_string();

        let sql = "INSERT OR REPLACE INTO datastate_audit (id, tablename, ability, caller, description, upby, cid, uid, uptime) VALUES (?, ?, ?, ?, ?, '', '', '', '')";

        let conn = db.get_conn();
        let conn_guard = conn.lock().map_err(|e| e.to_string())?;

        conn_guard
            .execute(
                sql,
                rusqlite::params![
                    id,
                    tablename,
                    ability,
                    caller,
                    description
                ],
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
    ///
    /// 只有在 audit_enabled=true 时才会检查权限
    /// 权限检查通过后会记录审计日志
    ///
    /// # Arguments
    /// * `db` - LocalDB 实例
    /// * `ability` - 方法名
    /// * `caller` - 调用方的包名（必须是调用者自己包的 PKG_NAME）
    ///
    /// # Returns
    /// * `Ok(true)` - 有权限或审计关闭
    /// * `Err(msg)` - 无权限或未注册（仅在审计开启时）
    pub fn check_permission(
        &self,
        db: &LocalDB,
        ability: &str,
        caller: &str,
    ) -> Result<bool, String> {
        if !self.audit_enabled {
            return Ok(true);
        }

        if &self.tablename == caller {
            let _ = self.log_action_with_count(ability, caller, "");
            return Ok(true);
        }

        let conn = db.get_conn();
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
                let _ = self.log_action_with_count(ability, caller, "");
                Ok(true)
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                Err(format!(
                    "[{}] 无权调用 [{}/{}]",
                    caller, &self.tablename, ability
                ))
            }
            Err(e) => Err(format!("查询权限失败：{}", e)),
        }
    }

    /// 获取能力权限信息
    pub fn get_ability_perm(db: &LocalDB, tablename: &str, ability: &str) -> Option<AbilityPerm> {
        let sql = "SELECT idpk, id, tablename, ability, caller, description, upby, cid, uid, uptime FROM datastate_audit WHERE tablename = ? AND ability = ?";

        match db.query(sql, &[&tablename as &dyn rusqlite::ToSql, &ability as &dyn rusqlite::ToSql]) {
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

    /// 带权限检查的执行包装
    ///
    /// 核心方法：AI 实现具体表时，只需把业务逻辑包装成闭包传入
    /// 自动完成：权限检查 + 审计日志记录
    ///
    /// # Arguments
    /// * `db` - 数据库连接
    /// * `ability` - 能力名（如 "getone", "save"）
    /// * `caller` - 调用者类名
    /// * `action` - 业务逻辑闭包
    ///
    /// # Returns
    /// 闭包的返回结果，或权限检查失败错误
    pub fn do_action<F, T>(
        &self,
        db: &LocalDB,
        ability: &str,
        caller: &str,
        action: F,
    ) -> Result<T, String>
    where
        F: FnOnce() -> Result<T, String>,
    {
        // 1. 权限检查
        self.check_permission(db, ability, caller)?;

        // 2. 记录审计日志（不论审计开关是否开启）
        let _ = self.log_action(db, ability, caller, "");

        // 3. 执行实际业务逻辑
        action()
    }

    /// 带输入参数的权限检查执行包装
    pub fn do_action_with_input<F, T>(
        &self,
        db: &LocalDB,
        ability: &str,
        caller: &str,
        input_params: &str,
        action: F,
    ) -> Result<T, String>
    where
        F: FnOnce() -> Result<T, String>,
    {
        // 1. 权限检查
        self.check_permission(db, ability, caller)?;

        // 2. 记录审计日志（带输入参数）
        let _ = self.log_action(db, ability, caller, input_params);

        // 3. 执行实际业务逻辑
        action()
    }

    /// 带权限检查的执行包装（计数方式）
    ///
    /// 使用新的计数方式记录审计日志，每天每个调用方对每个方法只有一条记录
    ///
    /// # Arguments
    /// * `db` - 数据库连接
    /// * `ability` - 能力名（如 "getone", "save"）
    /// * `caller` - 调用者类名
    /// * `action` - 业务逻辑闭包
    ///
    /// # Returns
    /// 闭包的返回结果，或权限检查失败错误
    pub fn do_action_with_count<F, T>(
        &self,
        db: &LocalDB,
        ability: &str,
        caller: &str,
        summary: &str,
        action: F,
    ) -> Result<T, String>
    where
        F: FnOnce() -> Result<T, String>,
    {
        // 1. 权限检查
        self.check_permission(db, ability, caller)?;

        // 2. 记录审计日志（计数方式，不论审计开关是否开启）
        self.log_action_with_count(ability, caller, summary)?;

        // 3. 执行实际业务逻辑
        action()
    }

    /// 带输入参数的权限检查执行包装（计数方式）
    pub fn do_action_with_input_count<F, T>(
        &self,
        db: &LocalDB,
        ability: &str,
        caller: &str,
        summary: &str,
        _input_params: &str,
        action: F,
    ) -> Result<T, String>
    where
        F: FnOnce() -> Result<T, String>,
    {
        // 1. 权限检查
        self.check_permission(db, ability, caller)?;

        // 2. 记录审计日志（计数方式，带摘要）
        self.log_action_with_count(ability, caller, summary)?;

        // 3. 执行实际业务逻辑
        action()
    }

    /// 记录能力调用日志
    fn log_action(
        &self,
        db: &LocalDB,
        ability: &str,
        caller: &str,
        input_params: &str,
    ) -> Result<(), String> {
        let ability_full = format!("{}/{}", self.tablename, ability);
        let conn = db.get_conn();
        let conn_guard = conn.lock().map_err(|e| e.to_string())?;

        // 生成ISO8601格式UTC时间戳 (精确到毫秒)
        let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();

        // 在input_params JSON中添加timestamp字段
        let input_with_timestamp: serde_json::Value = if input_params.is_empty() {
            serde_json::json!({ "timestamp": timestamp })
        } else {
            match serde_json::from_str::<serde_json::Value>(input_params) {
                Ok(mut v) => {
                    if let Some(obj) = v.as_object_mut() {
                        obj.insert("timestamp".to_string(), serde_json::Value::String(timestamp));
                    }
                    v
                }
                Err(_) => serde_json::json!({ "timestamp": timestamp, "raw": input_params }),
            }
        };
        let input_json = serde_json::to_string(&input_with_timestamp).unwrap_or_default();

        // 1. 记录每次调用
        let log_sql = "INSERT INTO data_ability_log (ability_name, caller, action, input_params) VALUES (?, ?, ?, ?)";
        conn_guard
            .execute(
                log_sql,
                rusqlite::params![ability_full, caller, ability, input_json],
            )
            .map_err(|e| format!("记录能力日志失败: {}", e))?;

        // 2. 保存每日唯一值 (去重)
        let input_hash = Uuid::new_v4().to_string();
        let today = Local::now().format("%Y-%m-%d").to_string();

        let daily_sql = "INSERT OR IGNORE INTO data_ability_daily (ability_name, caller, action, input_hash, stat_date) VALUES (?, ?, ?, ?, ?)";
        conn_guard
            .execute(
                daily_sql,
                rusqlite::params![ability_full, caller, ability, input_hash, today],
            )
            .map_err(|e| format!("保存每日唯一值失败: {}", e))?;

        Ok(())
    }

/// 记录能力调用日志（计数方式）
    ///
    /// 使用新的计数方式，每天每个调用方对每个方法只有一条记录
    /// 如果当天已有记录，则增加计数；否则创建新记录
    fn log_action_with_count(
        &self,
        ability: &str,
        caller: &str,
        summary: &str,
    ) -> Result<(), String> {
        // 直接调用 AuditLogDataState 记录日志
        let audit_log_state = super::AuditLogDataState::new();
        audit_log_state.log_audit(&self.tablename, ability, caller, summary)
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

    /// 获取每日唯一调用统计 (可查一个月)
    pub fn get_ability_daily_stats(db: &LocalDB, ability_name: &str, days: i32) -> Vec<AbilityDaily> {
        let sql = format!(
            "SELECT idpk, ability_name, caller, action, stat_date FROM data_ability_daily WHERE ability_name = ? ORDER BY stat_date DESC LIMIT {}",
            days
        );

        match db.query(&sql, &[&ability_name as &dyn rusqlite::ToSql]) {
            Ok(results) => results
                .iter()
                .map(|row| AbilityDaily {
                    idpk: row.get("idpk").and_then(|v| v.as_i64()).unwrap_or(0),
                    ability_name: row.get("ability_name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    caller: row.get("caller").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    action: row.get("action").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    stat_date: row.get("stat_date").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    /// 当前时间戳
    #[allow(dead_code)]
    fn current_time() -> f64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64()
    }
}

// ========== 独立函数（供 DataState 重新导出使用） ==========

// ========== 能力层权限函数 ==========

/// 注册能力权限
pub fn register_ability(
    db: &LocalDB,
    tablename: &str,
    ability: &str,
    caller: &str,
    description: &str,
    upby: &str,
    cid: &str,
    uid: &str,
) -> Result<(), String> {
    // 生成业务主键 id: UUID
    let id = Uuid::new_v4().to_string();

    let sql = "INSERT OR REPLACE INTO datastate_audit (id, tablename, ability, caller, description, upby, cid, uid, uptime) VALUES (?, ?, ?, ?, ?, ?, ?, ?, '')";

    let conn = db.get_conn();
    let conn_guard = conn.lock().map_err(|e| e.to_string())?;

    conn_guard.execute(
        sql,
        rusqlite::params![id, tablename, ability, caller, description, upby, cid, uid],
    ).map_err(|e| format!("注册能力权限失败：{}", e))?;

    Ok(())
}

/// 注册能力权限（简化版）
pub fn register_ability_simple(
    db: &LocalDB,
    tablename: &str,
    ability: &str,
    caller: &str,
    description: &str,
) -> Result<(), String> {
    register_ability(db, tablename, ability, caller, description, "", "", "")
}

/// 验证能力权限
pub fn check_ability_permission(db: &LocalDB, tablename: &str, ability: &str, caller: &str, audit_enabled: bool) -> Result<bool, String> {
    // 审计关闭时，直接放行
    if !audit_enabled {
        return Ok(true);
    }

    // 本表内部调用 → 自动放行
    if tablename == caller {
        return Ok(true);
    }

    // 支持 ability=* 通配符
    let sql = "SELECT 1 FROM datastate_audit WHERE tablename = ? AND caller = ? AND (ability = ? OR ability = '*')";
    let conn = db.get_conn();
    let conn_guard = conn.lock().map_err(|e| e.to_string())?;

    let result: Result<i32, _> = conn_guard.query_row(
        sql,
        rusqlite::params![tablename, caller, ability],
        |_| Ok(1),
    );

    match result {
        Ok(_) => Ok(true), // 有权限
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            Err(format!("[{}] 无权访问 [{}/{}]", caller, tablename, ability))
        }
        Err(e) => Err(format!("查询权限失败：{}", e)),
    }
}

/// 获取能力权限信息
pub fn get_ability_perm(db: &LocalDB, tablename: &str, ability: &str) -> Option<AbilityPerm> {
    let sql = "SELECT idpk, id, tablename, ability, caller, description, upby, cid, uid, uptime FROM datastate_audit WHERE tablename = ? AND ability = ?";

    match db.query(sql, &[&tablename as &dyn rusqlite::ToSql, &ability as &dyn rusqlite::ToSql]) {
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

/// 记录能力调用日志
pub fn log_ability_call(
    db: &LocalDB,
    tablename: &str,
    ability: &str,
    caller: &str,
    action: &str,
    input_params: &serde_json::Value,
) -> Result<(), String> {
    let created_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64();

    // 生成ISO8601格式UTC时间戳 (精确到毫秒)
    let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();

    // 在input_params中添加timestamp字段
    let mut params_with_timestamp = input_params.clone();
    if let Some(obj) = params_with_timestamp.as_object_mut() {
        obj.insert("timestamp".to_string(), serde_json::Value::String(timestamp));
    }

    let input_json = serde_json::to_string(&params_with_timestamp).unwrap_or_default();

    let log_sql = "INSERT INTO data_ability_log (ability_name, caller, action, input_params, created_at) VALUES (?, ?, ?, ?, ?)";
    let conn = db.get_conn();
    let conn_guard = conn.lock().map_err(|e| e.to_string())?;

    let ability_full = format!("{}/{}", tablename, ability);
    conn_guard.execute(
        log_sql,
        rusqlite::params![ability_full, caller, action, input_json, created_at],
    ).map_err(|e| format!("记录能力调用日志失败: {}", e))?;

    // 保存每日唯一值
    let input_hash = Uuid::new_v4().to_string();
    let today = Local::now().format("%Y-%m-%d").to_string();

    let daily_sql = "INSERT OR IGNORE INTO data_ability_daily (ability_name, caller, action, input_hash, stat_date) VALUES (?, ?, ?, ?, ?)";
    conn_guard.execute(
        daily_sql,
        rusqlite::params![ability_full, caller, action, input_hash, today],
    ).map_err(|e| format!("保存每日唯一值失败: {}", e))?;

    Ok(())
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

/// 获取每日唯一调用统计
pub fn get_ability_daily_stats(db: &LocalDB, ability_name: &str, days: i32) -> Vec<AbilityDaily> {
    let sql = format!(
        "SELECT idpk, ability_name, caller, action, stat_date FROM data_ability_daily WHERE ability_name = ? ORDER BY stat_date DESC LIMIT {}",
        days
    );

    match db.query(&sql, &[&ability_name as &dyn rusqlite::ToSql]) {
        Ok(results) => results
            .iter()
            .map(|row| AbilityDaily {
                idpk: row.get("idpk").and_then(|v| v.as_i64()).unwrap_or(0),
                ability_name: row.get("ability_name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                caller: row.get("caller").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                action: row.get("action").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                stat_date: row.get("stat_date").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// 带能力权限检查的宏 - 简化版
///
/// # 使用方式
/// ```ignore
/// impl TestTb {
///     fn getone(&self, id: &str) -> Result<Record, String> {
///         // 自动获取 caller（调用方类型名）
///         // 自动使用 self.db
///         audit!(self, "getone", || {
///             // 业务逻辑
///             Ok(record)
///         })
///     }
///     
///     // 跨服务调用（指定 caller）
///     fn cross_call(&self, id: &str, caller: &str) -> Result<Record, String> {
///         audit!(self, "getone", caller, || {
///             Ok(record)
///         })
///     }
/// }
/// ```
#[macro_export]
macro_rules! audit {
    // 不带 caller：audit!(self, "ability", { ... }) - 自动获取 caller
    ($self:ident, $ability:literal, $body:block) => {{
        let caller = std::any::type_name::<$self>()
            .rsplit("::")
            .next()
            .unwrap_or("");
        $self.audit.do_action(&$self.db, $ability, caller, || $body)
    }};
    // 带 caller：audit!(self, "ability", caller, { ... })
    ($self:ident, $ability:literal, $caller:expr, $body:block) => {{
        $self.audit.do_action(&$self.db, $ability, $caller, || $body)
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ability_perm_structure() {
        let perm = AbilityPerm {
            idpk: 1,
            id: "testtb_getone".to_string(),
            tablename: "testtb".to_string(),
            ability: "getone".to_string(),
            caller: "ClassA,ClassB".to_string(),
            description: "测试能力".to_string(),
            upby: "admin".to_string(),
            cid: "".to_string(),
            uid: "".to_string(),
            uptime: "".to_string(),
        };

        assert_eq!(perm.tablename, "testtb");
        assert_eq!(perm.ability, "getone");
    }

    #[test]
    fn test_data_audit_new() {
        let audit = DataAudit::new("testtb");
        assert_eq!(audit.tablename, "testtb");
    }
}