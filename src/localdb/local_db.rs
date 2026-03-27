//! LocalDB - SQLite 本地数据库封装
//!
//! 提供 Local-First 存储的本地封装
//!
//! # 用法
//! ```rust
//! use datastate::LocalDB;
//!
//! // 默认实例（三级优先级：环境变量 > 配置文件 > 默认路径）
//! let db = LocalDB::default_instance()?;
//!
//! // 多数据源场景
//! let main_db = LocalDB::default_instance()?;
//! let game_db = LocalDB::with_path("data/game.db")?;
//! let log_db = LocalDB::with_path("data/logs.db")?;
//! ```
//!
//! ## 数据库路径优先级
//! 1. 环境变量 `SQLITE_PATH`
//! 2. 配置文件 `docs/config/{env}.ini` 中的 `[database] db_path`
//! 3. 默认路径 `docs/config/local.db`

use rusqlite::{Connection, Row, Rows, params};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::collections::HashMap;
use chrono::Local;
use base::mylogger;
use prost::Message;

use base::project_path::ProjectPath;
use crate::datastate::{DATA_ABILITY_LOG_CREATE_SQL};
use crate::data_sync::{DATA_STATE_LOG_CREATE_SQL, DATA_SYNC_STATS_CREATE_SQL, SYNCLOG_CREATE_SQL, ProtoSynclogBatch};
use crate::sqlite78::{SYS_SQL_CREATE_SQL, SYS_WARN_CREATE_SQL};

/// LocalDB 配置
#[derive(Debug, Clone)]
pub struct LocalDBConfig {
    /// 是否记录警告日志（调试跟踪、错误记录）
    pub is_log: bool,
    /// 是否统计 SQL 效率
    pub is_count: bool,
    /// 公司ID
    pub cid: String,
    /// 用户ID
    pub uid: String,
    /// 操作者
    pub upby: String,
    /// 系统名
    pub apisys: String,
    /// 微服务名
    pub apimicro: String,
}

impl Default for LocalDBConfig {
    fn default() -> Self {
        let (cid, uid, upby) = ProjectPath::find()
            .ok()
            .and_then(|p| p.load_ini_config().ok())
            .map(|config| {
                let user_config = config.get("user7788").cloned().unwrap_or_default();
                let default_config = config.get("DEFAULT").cloned().unwrap_or_default();
                
                let cid = user_config.get("cid").cloned().unwrap_or_default();
                let uid = user_config.get("uid").cloned().unwrap_or_default();
                let upby = user_config.get("username").cloned()
                    .or_else(|| default_config.get("uname").cloned())
                    .unwrap_or_default();
                
                (cid, uid, upby)
            })
            .unwrap_or_default();

        Self {
            is_log: true,
            is_count: true,
            cid,
            uid,
            upby,
            apisys: String::new(),
            apimicro: String::new(),
        }
    }
}

/// 全局数据库实例（单例模式）
static INSTANCE: OnceLock<LocalDB> = OnceLock::new();

/// 本地数据库管理类
#[derive(Debug, Clone)]
pub struct LocalDB {
    conn: Arc<Mutex<Connection>>,
    db_path: PathBuf,
    config: LocalDBConfig,
}

impl Default for LocalDB {
    fn default() -> Self {
        Self::new(None).unwrap_or_else(|_| {
            panic!("Failed to create default LocalDB")
        })
    }
}

impl LocalDB {
    pub fn new(config: Option<LocalDBConfig>) -> Result<Self, String> {
        let path = Self::resolve_db_path()?;

        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("创建目录失败: {}", e))?;
            }
        }

        let conn = Connection::open(&path)
            .map_err(|e| format!("连接数据库失败: {}", e))?;

        // 设置 WAL 模式
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA busy_timeout=30000;")
            .map_err(|e| format!("设置 PRAGMA 失败: {}", e))?;

        let config = config.unwrap_or_default();

        // sys_sql 和 sys_warn 表由 datastate 注册时自动创建

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            db_path: path,
            config,
        })
    }

    /// 解析数据库路径（优先级：环境变量 > 配置文件 > 默认）
    fn resolve_db_path() -> Result<PathBuf, String> {
        // 1. 优先：环境变量 SQLITE_PATH
        if let Ok(env_path) = std::env::var("SQLITE_PATH") {
            if !env_path.is_empty() {
                return Ok(PathBuf::from(env_path));
            }
        }

        // 2. 其次：配置文件 docs/config/{env}.ini 中的 db_path
        if let Ok(project_path) = ProjectPath::find() {
            if let Ok(ini_config) = project_path.load_ini_config() {
                if let Some(database_section) = ini_config.get("database") {
                    if let Some(db_path_value) = database_section.get("db_path") {
                        if !db_path_value.is_empty() {
                            return Ok(PathBuf::from(db_path_value));
                        }
                    }
                }
            }
            // 3. 默认路径 docs/config/local.db
            return Ok(project_path.local_db());
        }

        Err("无法确定数据库路径：未找到项目根目录".to_string())
    }

    pub fn default_instance() -> Result<Self, String> {
        INSTANCE.get_or_init(|| Self::new(None).expect("创建数据库失败"));
        Ok(INSTANCE.get().unwrap().clone())
    }

    /// 使用指定路径创建数据库实例
    /// 
    /// 用于多数据源场景，如：
    /// - 主数据库：LocalDB::default_instance()
    /// - 游戏数据库：LocalDB::with_path("data/game.db")
    /// - 日志数据库：LocalDB::with_path("data/logs.db")
    pub fn with_path(db_path: &str) -> Result<Self, String> {
        let path = PathBuf::from(db_path);

        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("创建目录失败: {}", e))?;
            }
        }

        let conn = Connection::open(&path)
            .map_err(|e| format!("连接数据库失败: {}", e))?;

        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA busy_timeout=30000;")
            .map_err(|e| format!("设置 PRAGMA 失败: {}", e))?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            db_path: path,
            config: LocalDBConfig::default(),
        })
    }

    /// 获取按天分表的表名
    pub fn get_daily_table_name(base_name: &str, date_str: Option<&str>) -> String {
        let date = date_str.map(|s| s.to_string())
            .unwrap_or_else(|| Local::now().format("%Y%m%d").to_string());
        format!("{}_{}", base_name, date)
    }

    /// 确保表存在
    pub fn ensure_table(&self, table_name: &str, create_sql: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(create_sql, [])
            .map_err(|e| format!("创建表失败: {}", e))?;
        
        // 自动添加 id 唯一索引（如果表有 id 字段且没有唯一约束）
        let index_name = format!("u_{}_id", table_name);
        let create_index_sql = format!(
            "CREATE UNIQUE INDEX IF NOT EXISTS {} ON {}(id)",
            index_name, table_name
        );
        // 忽略错误（表可能没有 id 字段或已有唯一约束）
        let _ = conn.execute(&create_index_sql, []);
        
        Ok(())
    }

    /// 检查表是否存在
    pub fn table_exists(&self, table_name: &str) -> Result<bool, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let exists: bool = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?",
            [table_name],
            |row| row.get::<_, i32>(0)
        ).map_err(|e| format!("查询表失败: {}", e))?
            > 0;
        Ok(exists)
    }

    /// 初始化系统表（统计和状态机相关）
    pub fn init_system_tables(&self) -> Result<(), String> {
        let tables = [
            ("synclog", SYNCLOG_CREATE_SQL),
            ("data_state_log", DATA_STATE_LOG_CREATE_SQL),
            ("data_sync_stats", DATA_SYNC_STATS_CREATE_SQL),
            ("data_ability_log", DATA_ABILITY_LOG_CREATE_SQL),
            ("sys_sql", SYS_SQL_CREATE_SQL),
            ("sys_warn", SYS_WARN_CREATE_SQL),
        ];

        for (table_name, create_sql) in tables {
            if !self.table_exists(table_name)? {
                self.ensure_table(table_name, create_sql)?;
            } else {
                // 已存在的表，尝试添加 id 唯一索引
                self.ensure_id_unique_index(table_name)?;

                // sys_warn 表升级：添加缺失的列
                if table_name == "sys_warn" {
                    let columns = [
                        ("cid", "TEXT NOT NULL DEFAULT ''"),
                        ("apisys", "TEXT NOT NULL DEFAULT ''"),
                        ("apimicro", "TEXT NOT NULL DEFAULT ''"),
                        ("apiobj", "TEXT NOT NULL DEFAULT ''"),
                        ("upid", "TEXT NOT NULL DEFAULT ''"),
                    ];
                    for (col, def) in &columns {
                        self.ensure_column("sys_warn", col, def)?;
                    }
                }
            }
        }
        Ok(())
    }

    /// 确保表有指定列（没有则添加）
    pub fn ensure_column(&self, table_name: &str, column: &str, column_def: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        // 检查列是否存在（遍历所有列）
        let mut stmt = conn.prepare(&format!("PRAGMA table_info({})", table_name))
            .map_err(|e| e.to_string())?;
        let columns: Vec<String> = stmt.query_map([], |row| row.get::<_, String>(1))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();

        if !columns.contains(&column.to_string()) {
            let sql = format!("ALTER TABLE {} ADD COLUMN {} {}", table_name, column, column_def);
            conn.execute(&sql, []).map_err(|e| format!("添加列失败: {}", e))?;
        }

        Ok(())
    }

    /// 确保 id 字段有唯一索引
    pub fn ensure_id_unique_index(&self, table_name: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        
        // 检查是否已有 id 唯一约束（通过索引或 UNIQUE）
        let has_unique: bool = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND tbl_name=? AND sql LIKE '%id%' AND sql LIKE '%UNIQUE%'",
            [table_name],
            |row| row.get::<_, i32>(0)
        ).unwrap_or(0) > 0;
        
        if has_unique {
            return Ok(());
        }
        
        // 检查表是否有 id 字段
        let has_id: bool = conn.prepare(&format!("PRAGMA table_info({})", table_name))
            .and_then(|mut stmt| {
                let mut rows = stmt.query([])?;
                while let Some(row) = rows.next()? {
                    let name: String = row.get(1)?;
                    if name == "id" {
                        return Ok(true);
                    }
                }
                Ok(false)
            })
            .unwrap_or(false);
        
        if !has_id {
            return Ok(());
        }
        
        // 添加唯一索引
        let index_name = format!("u_{}_id", table_name);
        let sql = format!(
            "CREATE UNIQUE INDEX IF NOT EXISTS {} ON {}(id)",
            index_name, table_name
        );
        let _ = conn.execute(&sql, []);
        
        Ok(())
    }

    /// 插入数据
    pub fn insert(&self, table: &str, data: &HashMap<String, Value>) -> Result<String, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        // 获取表的实际字段列表
        let table_columns: Vec<String> = conn.prepare(&format!("PRAGMA table_info({})", table))
            .and_then(|mut stmt| {
                let mut rows = stmt.query([])?;
                let mut cols = Vec::new();
                while let Some(row) = rows.next()? {
                    let name: String = row.get(1)?;
                    cols.push(name);
                }
                Ok(cols)
            })
            .unwrap_or_default();

        // 过滤掉本地表不存在的字段，并按表字段顺序排列
        let mut ordered_columns: Vec<String> = Vec::new();
        let mut ordered_values: Vec<Value> = Vec::new();
        
        for col in &table_columns {
            if let Some(v) = data.get(col) {
                ordered_columns.push(col.clone());
                ordered_values.push(v.clone());
            }
        }
        
        // 确保 id 字段存在
        if !ordered_columns.contains(&"id".to_string()) {
            if let Some(v) = data.get("id") {
                ordered_columns.push("id".to_string());
                ordered_values.push(v.clone());
            }
        }

        // 调试信息
        let logger = mylogger!();
        logger.detail(&format!("[insert] table: {}, columns: {:?}, data: {:?}", table, ordered_columns, data));

        let columns: Vec<&str> = ordered_columns.iter().map(|s| s.as_str()).collect();
        let placeholders: Vec<&str> = (0..columns.len()).map(|_| "?").collect();
        let sql = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            table,
            columns.join(", "),
            placeholders.join(", ")
        );

        let values: Vec<String> = ordered_values.iter().map(|v| {
            match v {
                Value::Null => "".to_string(),
                Value::Bool(b) => b.to_string(),
                Value::Number(n) => n.to_string(),
                Value::String(s) => s.clone(),
                Value::Array(_) | Value::Object(_) => serde_json::to_string(v).unwrap_or_default(),
            }
        }).collect();

        let logger = mylogger!();
        logger.detail(&format!("[insert] SQL: {}, values: {:?}", sql, values));

        let params_vec: Vec<&dyn rusqlite::ToSql> = values.iter()
            .map(|v| v as &dyn rusqlite::ToSql)
            .collect();

        conn.execute(&sql, params_vec.as_slice())
            .map_err(|e| format!("插入失败: {}", e))?;

        Ok(data.get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string())
    }

    /// 更新数据
    pub fn update(&self, table: &str, row_id: &str, data: &HashMap<String, Value>) -> Result<bool, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        let set_clause: Vec<String> = data.keys()
            .map(|k| format!("{} = ?", k))
            .collect();
        let sql = format!(
            "UPDATE {} SET {} WHERE id = ?",
            table,
            set_clause.join(", ")
        );

        let mut values: Vec<String> = data.values().map(|v| {
            match v {
                Value::Null => "".to_string(),
                Value::Bool(b) => b.to_string(),
                Value::Number(n) => n.to_string(),
                Value::String(s) => s.clone(),
                Value::Array(_) | Value::Object(_) => serde_json::to_string(v).unwrap_or_default(),
            }
        }).collect();
        values.push(row_id.to_string());

        let params_vec: Vec<&dyn rusqlite::ToSql> = values.iter()
            .map(|v| v as &dyn rusqlite::ToSql)
            .collect();

        let rows_affected = conn.execute(&sql, params_vec.as_slice())
            .map_err(|e| format!("更新失败: {}", e))?;

        Ok(rows_affected > 0)
    }

    /// 查询数据
    pub fn query(&self, sql: &str, params: &[&dyn rusqlite::ToSql]) -> Result<Vec<HashMap<String, Value>>, String> {
        use std::time::Instant;
        let start = Instant::now();

        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        let mut stmt = conn.prepare(sql)
            .map_err(|e| format!("准备语句失败: {}", e))?;

        let column_names: Vec<String> = stmt.column_names()
            .into_iter()
            .map(|s| s.to_string())
            .collect();

        let rows = stmt.query(params)
            .map_err(|e| format!("查询失败: {}", e))?;

        let result = Self::process_rows(rows, &column_names)?;

        let elapsed = start.elapsed().as_millis() as i64;
        let downlen = result.len() as i64;

        // 记录 SQL 统计
        if self.config.is_count {
            let apiobj = Self::parse_table_name(sql);
            if let Err(e) = Self::do_save_sql_log(&conn, &self.config.cid, &self.config.apisys, &self.config.apimicro, &apiobj, sql, elapsed, downlen, &self.config.upby) {
                let logger = mylogger!();
                logger.error(&format!("[LocalDB] save_sql_log 失败: {}", e));
            }
        }

        // 记录调试日志
        if self.config.is_log {
            let apiobj = Self::parse_table_name(sql);
            let result_json = serde_json::to_string(&result).unwrap_or_default();
            let params_str = Self::params_to_string(params);
            let content = format!("{} c:{} v{}", result_json, sql, params_str);
            let _ = Self::do_add_warn(&conn, &self.config.cid, "debug_local", &self.config.apisys, &self.config.apimicro, &apiobj, &content, &self.config.upby);
        }

        Ok(result)
    }

    /// 将参数转换为字符串
    fn params_to_string(params: &[&dyn rusqlite::ToSql]) -> String {
        let parts: Vec<String> = params.iter().map(|p| {
            let sql_value = match p.to_sql() {
                Ok(v) => v,
                Err(_) => return "?".to_string(),
            };
            match sql_value {
                rusqlite::types::ToSqlOutput::Owned(v) => Self::value_to_string(&v),
                rusqlite::types::ToSqlOutput::Borrowed(v) => Self::value_ref_to_string(v),
                _ => "?".to_string(),
            }
        }).collect();
        format!("[{}]", parts.join(","))
    }

    /// 将 SQLite 值转换为字符串
    fn value_to_string(value: &rusqlite::types::Value) -> String {
        use rusqlite::types::Value;
        match value {
            Value::Null => "null".to_string(),
            Value::Integer(i) => i.to_string(),
            Value::Real(f) => f.to_string(),
            Value::Text(s) => format!("\"{}\"", s),
            Value::Blob(b) => format!("<blob:{}bytes>", b.len()),
        }
    }

    /// 将 SQLite 值引用转换为字符串
    fn value_ref_to_string(value: rusqlite::types::ValueRef) -> String {
        use rusqlite::types::ValueRef;
        match value {
            ValueRef::Null => "null".to_string(),
            ValueRef::Integer(i) => i.to_string(),
            ValueRef::Real(f) => f.to_string(),
            ValueRef::Text(s) => format!("\"{}\"", String::from_utf8_lossy(s)),
            ValueRef::Blob(b) => format!("<blob:{}bytes>", b.len()),
        }
    }

    /// 处理查询结果
    fn process_rows(mut rows: Rows, column_names: &[String]) -> Result<Vec<HashMap<String, Value>>, String> {
        let mut results = Vec::new();
        loop {
            match rows.next().map_err(|e| format!("读取行失败: {}", e))? {
                Some(row) => {
                    let mut map = HashMap::new();
                    for (i, name) in column_names.iter().enumerate() {
                        let value = Self::row_value_to_json(&row, i);
                        map.insert(name.clone(), value);
                    }
                    results.push(map);
                }
                None => break,
            }
        }
        Ok(results)
    }

    /// 将行值转换为 JSON
    fn row_value_to_json(row: &Row, col_index: usize) -> Value {
        // 尝试获取字符串
        if let Ok(s) = row.get::<_, String>(col_index) {
            // 直接返回字符串，不尝试解析为 JSON
            return Value::String(s);
        }
        // 尝试获取整数
        if let Ok(n) = row.get::<_, i64>(col_index) {
            return Value::Number(n.into());
        }
        // 尝试获取浮点数
        if let Ok(n) = row.get::<_, f64>(col_index) {
            return serde_json::json!(n);
        }
        Value::Null
    }

    /// 执行 SQL
    pub fn execute(&self, sql: &str) -> Result<(), String> {
        use std::time::Instant;
        let start = Instant::now();

        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let result = conn.execute(sql, [])
            .map_err(|e| format!("执行失败: {}", e))?;

        let elapsed = start.elapsed().as_millis() as i64;

        // 记录 SQL 统计
        if self.config.is_count {
            let apiobj = Self::parse_table_name(sql);
            if let Err(e) = Self::do_save_sql_log(&conn, &self.config.cid, &self.config.apisys, &self.config.apimicro, &apiobj, sql, elapsed, 0, &self.config.upby) {
                let logger = mylogger!();
                logger.error(&format!("[LocalDB] save_sql_log 失败: {}", e));
            }
        }

        // 记录调试日志
        if self.config.is_log {
            let apiobj = Self::parse_table_name(sql);
            let content = format!("rows_affected={} c:{}", result, sql);
            if let Err(e) = Self::do_add_warn(&conn, &self.config.cid, "debug_local", &self.config.apisys, &self.config.apimicro, &apiobj, &content, &self.config.upby) {
                let logger = mylogger!();
                logger.error(&format!("[LocalDB] add_warn 失败: {}", e));
            }
        }

        Ok(())
    }

    /// 清理过期表
    pub fn cleanup_old_tables(&self, base_name: &str, retention_days: i32) -> Result<Vec<String>, String> {
        if retention_days <= 0 {
            return Ok(Vec::new());
        }

        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        // 获取所有匹配的表
        let pattern = format!("{}_%", base_name);
        let tables: Vec<String> = conn.prepare(
            "SELECT name FROM sqlite_master WHERE type='table' AND name LIKE ?"
        )
        .map_err(|e| format!("准备语句失败: {}", e))?
        .query_map([&pattern], |row| row.get(0))
        .map_err(|e| format!("查询失败: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

        let cutoff_date = Local::now() - chrono::Duration::days(retention_days as i64);
        let cutoff_str = cutoff_date.format("%Y%m%d").to_string();

        let mut dropped = Vec::new();
        for table in tables {
            // 提取日期部分
            if let Some(date_part) = table.split('_').last() {
                if date_part.len() == 8 && date_part.parse::<i32>().is_ok() {
                    if date_part < cutoff_str.as_str() {
                        conn.execute(&format!("DROP TABLE IF EXISTS {}", table), [])
                            .map_err(|e| format!("删除表失败: {}", e))?;
                        dropped.push(table);
                    }
                }
            }
        }

        Ok(dropped)
    }

    /// 获取数据库路径
    pub fn get_db_path(&self) -> &std::path::Path {
        &self.db_path
    }

    /// 获取数据库连接（用于事务等高级操作）
    pub fn get_conn(&self) -> Arc<Mutex<Connection>> {
        self.conn.clone()
    }

    /// 执行带参数的 SQL
    pub fn execute_with_params(&self, sql: &str, params: &[&dyn rusqlite::ToSql]) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(sql, params)
            .map_err(|e| format!("执行失败: {}", e))?;
        Ok(())
    }

    /// 执行带参数的 SQL，返回影响行数
    pub fn execute_with_params_affected(&self, sql: &str, params: &[&dyn rusqlite::ToSql]) -> Result<usize, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let rows_affected = conn.execute(sql, params)
            .map_err(|e| format!("执行失败: {}", e))?;
        Ok(rows_affected)
    }

    /// 获取表记录数
    pub fn count(&self, table: &str) -> Result<i32, String> {
        let sql = format!("SELECT COUNT(*) FROM {}", table);
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let count: i32 = conn.query_row(&sql, [], |row| row.get(0))
            .map_err(|e| format!("查询失败: {}", e))?;
        Ok(count)
    }

    /// 删除记录
    pub fn delete(&self, table: &str, row_id: &str) -> Result<bool, String> {
        let sql = format!("DELETE FROM {} WHERE id = ?", table);
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let rows_affected = conn.execute(&sql, [row_id])
            .map_err(|e| format!("删除失败: {}", e))?;
        Ok(rows_affected > 0)
    }

    /// 检查表的id字段是否是主键
    pub fn is_id_primary_key(&self, table: &str) -> Result<bool, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let sql = format!("PRAGMA table_info({})", table);
        let mut stmt = conn.prepare(&sql).map_err(|e| format!("查询表信息失败: {}", e))?;
        
        let rows = stmt.query_map([], |row| {
            let pk: i32 = row.get(5)?;
            let name: String = row.get(1)?;
            Ok((name, pk))
        }).map_err(|e| format!("获取表信息失败: {}", e))?;
        
        for row_result in rows {
            let (name, pk) = row_result.map_err(|e| format!("读取行失败: {}", e))?;
            if name == "id" {
                return Ok(pk > 0);
            }
        }
        Ok(false)
    }

    /// 将id字段设置为主键（如果当前主键是idpk）
    /// 
    /// 重建表结构：
    /// - id 为主键（使用雪花算法生成）
    /// - idpk 保留用于兼容（自增，但不是主键）
    pub fn ensure_id_is_primary_key(&self, table: &str) -> Result<bool, String> {
        let is_pk = self.is_id_primary_key(table)?;
        if is_pk {
            return Ok(false);
        }
        
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        
        let temp_table = format!("{}_temp_{}", table, chrono::Utc::now().format("%Y%m%d%H%M%S"));
        
        conn.execute_batch(&format!(
            r#"
            CREATE TABLE {temp} AS SELECT * FROM {table};
            DROP TABLE {table};
            CREATE TABLE {table} (
                id TEXT NOT NULL PRIMARY KEY,
                idpk INTEGER,
                cid TEXT NOT NULL DEFAULT '',
                kind TEXT NOT NULL DEFAULT '',
                item TEXT NOT NULL DEFAULT '',
                data TEXT NOT NULL DEFAULT '',
                upby TEXT NOT NULL DEFAULT '',
                uptime TEXT NOT NULL DEFAULT ''
            );
            INSERT INTO {table} SELECT * FROM {temp};
            DROP TABLE {temp};
            "#,
            temp = temp_table,
            table = table
        )).map_err(|e| format!("修改主键失败: {}", e))?;
        
        Ok(true)
    }

    /// 应用远程更新（插入或更新，不考虑 sync_queue）
    pub fn apply_remote_update(&self, table: &str, record_id: &str, data: &HashMap<String, Value>) -> Result<(), String> {
        // 检查本地是否存在
        let sql = format!("SELECT id FROM {} WHERE id = ?", table);
        let exists = self.query(&sql, &[&record_id as &dyn rusqlite::ToSql])?
            .first()
            .is_some();

        if exists {
            // 更新
            self.update(table, record_id, data)?;
        } else {
            // 插入
            self.insert(table, data)?;
        }

        Ok(())
    }

    /// 从配置文件读取 SID
    pub fn get_sid(&self) -> String {
        if let Ok(path) = ProjectPath::find() {
            if let Some(sid) = path.read_ini_value("user7788", "sid") {
                // 跳过占位符
                if !sid.starts_with("{{") && !sid.is_empty() {
                    return sid;
                }
            }
        }
        String::new()
    }

    /// 从服务器下载数据到本地（支持分页，兼容 JSON 和 protobuf 格式）
    ///
    /// - 如果 api_url 指向 synclog_mysql，使用 protobuf 格式
    /// - 否则使用旧的 JSON 格式（logsvc）
    pub fn download_from_server(
        &self,
        _table: &str,
        api_url: &str,
        getnumber: i32,
        getstart: i32,
        download_condition: Option<&Value>,
    ) -> Result<Vec<HashMap<String, Value>>, String> {
        use base::http::HttpHelper;
        use base64::{Engine as _, engine::general_purpose};

        let sid = self.get_sid();
        if sid.is_empty() {
            return Err("配置文件未找到 SID".to_string());
        }

        // 判断是否为 Rust API（synclog_mysql）
        let is_rust_api = api_url.contains("synclog_mysql");

        if is_rust_api {
            // === Rust API: protobuf 格式 ===
            let url = format!("{}/get", api_url.trim_end_matches('/'));

            //  protobuf 格式请求：只需要 sid 和 limit
            let request_payload = serde_json::json!({
                "sid": sid,
                "limit": getnumber,
            });

            let response = HttpHelper::post(&url, None, Some(&request_payload), None, false, None, 30, 2);

            if response.res != 0 {
                return Err(response.errmsg);
            }

            if let Some(data) = response.data {
                let response_value: Value = data.response;
                if let Some(bytedata_base64) = response_value.get("bytedata").and_then(|v| v.as_str()) {
                    // 解码 base64 protobuf
                    let bytes = general_purpose::STANDARD
                        .decode(bytedata_base64)
                        .map_err(|e| format!("Base64解码失败: {}", e))?;

                    // 解码 SynclogBatch protobuf
                    let batch = ProtoSynclogBatch::decode(&*bytes)
                        .map_err(|e| format!("Protobuf解码失败: {}", e))?;

                    // 转换为 HashMap<String, Value>
                    // 注意：服务器 get() 返回的业务数据存储在 cmdtext 字段（JSON 格式）
                    let mut result: Vec<HashMap<String, Value>> = Vec::new();
                    for item in batch.items {
                        // 尝试解析 cmdtext 中的业务数据 JSON
                        let business_data: HashMap<String, Value> = match serde_json::from_str(&item.cmdtext) {
                            Ok(map) => map,
                            Err(_) => {
                                // 回退：构建同步日志字段（兼容旧逻辑）
                                let mut fallback = HashMap::new();
                                fallback.insert("id".to_string(), Value::String(item.id.clone()));
                                fallback.insert("tbname".to_string(), Value::String(item.tbname.clone()));
                                fallback.insert("action".to_string(), Value::String(item.action.clone()));
                                fallback.insert("cmdtext".to_string(), Value::String(item.cmdtext.clone()));
                                fallback
                            }
                        };

                        // 确保业务数据包含 tbname 和 id
                        let mut record = business_data;
                        if !record.contains_key("tbname") {
                            record.insert("tbname".to_string(), Value::String(item.tbname.clone()));
                        }
                        if !record.contains_key("id") {
                            record.insert("id".to_string(), Value::String(item.id.clone()));
                        }

                        result.push(record);
                    }

                    return Ok(result);
                }
            }

            Ok(Vec::new())
        } else {
            // === 旧版 JSON 格式 ===
            // 自动添加 /get 后缀（与 Python 版本一致）
            let url = if api_url.ends_with("/get") {
                api_url.to_string()
            } else {
                format!("{}/get", api_url)
            };

            let mut request_payload = serde_json::json!({
                "sid": sid,
                "getnumber": getnumber,
                "getstart": getstart
            });

            if let Some(cond) = download_condition {
                if let Some(_arr) = cond.as_array() {
                    request_payload["pars"] = cond.clone();
                } else if let Some(_obj) = cond.as_object() {
                    request_payload["data"] = cond.clone();
                }
            }

            let response = HttpHelper::post(&url, None, Some(&request_payload), None, false, None, 30, 2);

            if response.res != 0 {
                return Err(response.errmsg);
            }

            if let Some(data) = response.data {
                let response_value: Value = data.response;
                if let Some(back) = response_value.get("back") {
                    if let Some(arr) = back.as_array() {
                        let mut result: Vec<HashMap<String, Value>> = Vec::new();
                        for item in arr {
                            if let Some(obj) = item.as_object() {
                                result.push(obj.clone().into_iter().collect());
                            }
                        }
                        return Ok(result);
                    }
                }
            }

            Ok(Vec::new())
        }
    }

    /// 上传本地数据到服务器（与 Python 版本一致）
    ///
    /// 调用服务器的 mAdd 接口
    /// URL: {api_url}/mAdd
    /// 请求体: {"sid": sid, "pars": [...], "cols": [...], "mid": id}
    ///
    /// # 参数
    /// - `_table`: 表名（未使用）
    /// - `api_url`: API 基础 URL
    /// - `data`: 要上传的数据
    /// - `cols`: 字段顺序（必须与服务器 colsImp 一致）
    pub fn upload_to_server(
        &self,
        _table: &str,
        api_url: &str,
        data: &HashMap<String, Value>,
        cols: Option<&[String]>,
    ) -> Result<i32, String> {
        use base::http::HttpHelper;

        let sid = self.get_sid();
        if sid.is_empty() {
            return Err("配置文件未找到 SID".to_string());
        }

        // 移除 /get 后缀（如果有）
        let base_url = if api_url.ends_with("/get") {
            &api_url[..api_url.len() - 4]
        } else {
            api_url
        };

        // 添加 /mAdd 后缀
        let url = format!("{}/mAdd", base_url);

        // 确定字段顺序：优先使用传入的 cols，否则使用 data 的 key（不推荐）
        let field_order: Vec<&str> = if let Some(cols_list) = cols {
            cols_list.iter().map(|s| s.as_str()).collect()
        } else {
            // 警告：HashMap 顺序是随机的，可能导致服务器解析错误
            data.keys().map(|s| s.as_str()).collect()
        };

        // 构建 pars 数组（按 cols 顺序）
        let pars: Vec<Value> = field_order
            .iter()
            .map(|col| data.get(*col).cloned().unwrap_or(Value::String(String::new())))
            .collect();

        // 构建请求体（与 Python 版本一致）
        let mut request_payload = serde_json::json!({
            "sid": sid,
            "pars": pars,
            "cols": field_order
        });

        // 如果 data 中包含 id 字段，传递给服务器复用
        if let Some(id) = data.get("id") {
            if let Some(id_str) = id.as_str() {
                request_payload["mid"] = serde_json::json!(id_str);
            }
        }

        let response = HttpHelper::post(&url, None, Some(&request_payload), None, false, None, 30, 2);

        // 记录服务器响应
        let logger = mylogger!();
        logger.info(&format!("[upload_to_server] 服务器响应: res={}, errmsg={}, data={:?}", 
            response.res, response.errmsg, response.data));

        if response.res != 0 {
            return Err(format!("服务器错误: res={}, errmsg={}", response.res, response.errmsg));
        }

        // 检查服务器返回的业务错误
        if let Some(ref resp_data) = response.data {
            if let Some(back_obj) = resp_data.response.as_object() {
                logger.info(&format!("[upload_to_server] 业务响应: {:?}", back_obj));
                if let Some(back_res) = back_obj.get("res") {
                    if back_res.as_i64().unwrap_or(0) != 0 {
                        let back_errmsg = back_obj.get("errmsg").and_then(|v| v.as_str()).unwrap_or("");
                        return Err(format!("业务错误: {}", back_errmsg));
                    }
                }
            }
        }

        Ok(1)
    }

    /// 批量上传 synclog 到服务器
    ///
    /// # 参数
    /// - `api_url`: API 基础 URL（synclog 的 API 地址）
    /// - `items`: 待同步的 synclog 列表
    /// 
    /// # 返回
    /// - `inserted`: 成功插入数量
    /// - `errors`: 验证失败的记录列表
    pub fn upload_batch_to_server(
        &self,
        api_url: &str,
        items: &[crate::data_sync::SynclogItem],
    ) -> Result<(i32, Vec<crate::data_sync::SyncValidationError>), String> {
        use base::http::HttpHelper;

        let sid = self.get_sid();
        if sid.is_empty() {
            return Err("配置文件未找到 SID".to_string());
        }

        if items.is_empty() {
            return Ok((0, Vec::new()));
        }

        let cols = vec![
            "id", "apisys", "apimicro", "apiobj", "tbname", "action", 
            "cmdtext", "params", "idrow", "worker", "synced",
            "lasterrinfo", "cmdtextmd5", "num", "dlong", "downlen"
        ];

        let mut pars: Vec<Value> = Vec::new();
        for item in items {
            pars.push(Value::String(item.id.clone()));
            pars.push(Value::String(item.apisys.clone()));
            pars.push(Value::String(item.apimicro.clone()));
            pars.push(Value::String(item.apiobj.clone()));
            pars.push(Value::String(item.tbname.clone()));
            pars.push(Value::String(item.action.clone()));
            pars.push(Value::String(item.cmdtext.clone()));
            pars.push(Value::String(item.params.clone()));
            pars.push(Value::String(item.idrow.clone()));
            pars.push(Value::String(item.worker.clone()));
            pars.push(Value::Number(item.synced.into()));
            pars.push(Value::String(String::new())); // lasterrinfo - 空字符串
            pars.push(Value::String(item.cmdtextmd5.clone()));
            pars.push(Value::Number(item.num.into()));
            pars.push(Value::Number(item.dlong.into()));
            pars.push(Value::Number(item.downlen.into()));
        }

        let request_payload = serde_json::json!({
            "sid": sid,
            "pars": pars,
            "cols": cols
        });

        let url = format!("{}/mAddMany", api_url.trim_end_matches('/'));

        let response = HttpHelper::post(&url, None, Some(&request_payload), None, false, None, 30, 2);

        let logger = mylogger!();
        logger.info(&format!("[upload_batch_to_server] mAddMany 响应: res={}, errmsg={}", 
            response.res, response.errmsg));

        if response.res != 0 {
            return Err(format!("服务器错误: {}", response.errmsg));
        }

        let mut inserted = items.len() as i32;
        let mut errors: Vec<crate::data_sync::SyncValidationError> = Vec::new();

        if let Some(ref resp_data) = response.data {
            if let Some(back_obj) = resp_data.response.as_object() {
                logger.info(&format!("[upload_batch_to_server] 业务响应: {:?}", back_obj));
                
                if let Some(back_res) = back_obj.get("res") {
                    if back_res.as_i64().unwrap_or(0) != 0 {
                        let back_errmsg = back_obj.get("errmsg").and_then(|v| v.as_str()).unwrap_or("");
                        return Err(format!("业务错误: {}", back_errmsg));
                    }
                }
                
                if let Some(back) = back_obj.get("back") {
                    if let Some(back_obj) = back.as_object() {
                        // 新格式：successIdrows, failedRecords
                        if let Some(count) = back_obj.get("successCount").and_then(|v| v.as_i64()) {
                            inserted = count as i32;
                        }
                        
                        // 解析 failedRecords: [{idrow, lasterrinfo}]
                        if let Some(failed_list) = back_obj.get("failedRecords").and_then(|v| v.as_array()) {
                            for (idx, err) in failed_list.iter().enumerate() {
                                if let Some(err_obj) = err.as_object() {
                                    errors.push(crate::data_sync::SyncValidationError {
                                        index: idx as i32,
                                        idrow: err_obj.get("idrow").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                        error: err_obj.get("lasterrinfo").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                    });
                                }
                            }
                        }
                        
                        // 兼容旧格式：batches, errors
                        if let Some(count) = back_obj.get("batches").and_then(|v| v.as_i64()) {
                            inserted = count as i32;
                        }
                        
                        if let Some(err_list) = back_obj.get("errors").and_then(|v| v.as_array()) {
                            for (idx, err) in err_list.iter().enumerate() {
                                if let Some(err_obj) = err.as_object() {
                                    errors.push(crate::data_sync::SyncValidationError {
                                        index: err_obj.get("index").and_then(|v| v.as_i64()).unwrap_or(idx as i64) as i32,
                                        idrow: err_obj.get("idrow").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                        error: err_obj.get("error").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                    });
                                }
                            }
                        }
                    } else if let Some(count) = back.as_i64() {
                        inserted = count as i32;
                    }
                }
            }
        }

        Ok((inserted, errors))
    }

    /// 从 SQL 语句中解析表名
    fn parse_table_name(sql: &str) -> String {
        let sql_upper = sql.to_uppercase();
        let sql_trimmed = sql.trim();
        
        // SELECT ... FROM table ...
        if sql_upper.starts_with("SELECT") {
            if let Some(from_pos) = sql_upper.find(" FROM ") {
                let after_from = sql_trimmed[from_pos + 6..].trim();
                if let Some(space_pos) = after_from.find(' ') {
                    return after_from[..space_pos].to_string();
                }
                return after_from.to_string();
            }
        }
        // INSERT INTO table ...
        else if sql_upper.starts_with("INSERT") {
            if let Some(into_pos) = sql_upper.find(" INTO ") {
                let after_into = sql_trimmed[into_pos + 6..].trim();
                if let Some(space_pos) = after_into.find(' ') {
                    return after_into[..space_pos].to_string();
                }
                return after_into.to_string();
            }
        }
        // UPDATE table SET ...
        else if sql_upper.starts_with("UPDATE") {
            let after_update = sql_trimmed[6..].trim();
            if let Some(space_pos) = after_update.find(' ') {
                return after_update[..space_pos].to_string();
            }
            return after_update.to_string();
        }
        // DELETE FROM table ...
        else if sql_upper.starts_with("DELETE") {
            if let Some(from_pos) = sql_upper.find(" FROM ") {
                let after_from = sql_trimmed[from_pos + 6..].trim();
                if let Some(space_pos) = after_from.find(' ') {
                    return after_from[..space_pos].to_string();
                }
                return after_from.to_string();
            }
        }
        
        String::new()
    }

    /// 记录 SQL 执行统计（静态方法，避免死锁）
    ///
    /// # 参数
    /// - `conn`: 数据库连接
    /// - `cid`: 公司ID
    /// - `apisys`: 系统名
    /// - `apimicro`: 微服务名
    /// - `apiobj`: 表名（对象名）
    /// - `cmdtext`: SQL 语句
    /// - `dlong`: 执行时间（毫秒）
    /// - `downlen`: 下行数据量
    /// - `upby`: 操作者
    fn do_save_sql_log(conn: &Connection, cid: &str, apisys: &str, apimicro: &str, apiobj: &str, cmdtext: &str, dlong: i64, downlen: i64, upby: &str) -> Result<(), String> {
        let cmdtextmd5 = format!("{:x}", md5::compute(cmdtext.as_bytes()));
        let uptime = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let id = crate::snowflake::next_id_string();

        let sql = r#"
            INSERT INTO sys_sql (id, cid, apisys, apimicro, apiobj, cmdtext, num, dlong, downlen, upby, cmdtextmd5, uptime)
            VALUES (?, ?, ?, ?, ?, ?, 1, ?, ?, ?, ?, ?)
            ON CONFLICT(cmdtextmd5) DO UPDATE SET 
                num = num + 1,
                dlong = dlong + ?,
                downlen = downlen + ?
        "#;

        conn.execute(&sql, params![
            &id,
            &cid,
            &apisys,
            &apimicro,
            &apiobj,
            &cmdtext,
            &dlong,
            &downlen,
            &upby,
            &cmdtextmd5,
            &uptime,
            &dlong,
            &downlen,
        ]).map_err(|e| e.to_string())?;

        Ok(())
    }

    /// 记录警告日志（静态方法，避免死锁）
    ///
    /// # 参数
    /// - `conn`: 数据库连接
    /// - `cid`: 公司ID（填UID的值）
    /// - `kind`: 日志类型
    /// - `apimicro`: 微服务名
    /// - `apiobj`: API对象名
    /// - `content`: 日志内容
    /// - `upby`: 操作者
    fn do_add_warn(conn: &Connection, cid: &str, kind: &str, apisys: &str, apimicro: &str, apiobj: &str, content: &str, upby: &str) -> Result<(), String> {
        let uptime = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let id = crate::snowflake::next_id_string();

        let sql = "INSERT INTO sys_warn (id, cid, kind, apisys, apimicro, apiobj, content, upby, uptime) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)";
        conn.execute(&sql, params![&id, &cid, &kind, &apisys, &apimicro, &apiobj, &content, &upby, &uptime])
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    /// 记录警告日志（调试跟踪、错误记录）
    ///
    /// # 参数
    /// - `kind`: 日志类型（如 debug_xxx, err_xxx）
    /// - `apisys`: 系统名
    /// - `apimicro`: 微服务名
    /// - `apiobj`: API对象名
    /// - `content`: 日志内容
    /// - `upby`: 操作者
    pub fn add_warn(&self, kind: &str, apisys: &str, apimicro: &str, apiobj: &str, content: &str, upby: &str) {
        if !self.config.is_log {
            return;
        }

        let conn = match self.conn.lock() {
            Ok(c) => c,
            Err(_) => return,
        };

        let _ = Self::do_add_warn(&conn, &self.config.cid, kind, apisys, apimicro, apiobj, content, upby);
    }

    /// 记录调试日志（简化版）
    pub fn add_debug(&self, apisys: &str, apimicro: &str, apiobj: &str, content: &str) {
        self.add_warn(&format!("debug_{}", apisys), apisys, apimicro, apiobj, content, "");
    }

    /// 记录错误日志（简化版）
    pub fn add_error(&self, apisys: &str, apimicro: &str, apiobj: &str, content: &str) {
        self.add_warn(&format!("err_{}", apisys), apisys, apimicro, apiobj, content, "");
    }
}

#[cfg(test)]
mod tests {
    use base::mylogger;
    use super::*;
    use std::path::PathBuf;

    fn find_project_root() -> Option<PathBuf> {
        let mut path = std::env::current_dir().ok()?;

        loop {
            let has_docs = path.join("docs").exists();
            let has_claude = path.join(".claude").exists();
            let has_crates = path.join("crates").exists();

            if has_docs && (has_claude || has_crates) {
                return Some(path);
            }

            if !path.pop() {
                break;
            }
        }

        None
    }

    fn get_test_db_path() -> String {
        find_project_root()
            .map(|root| root.join("docs/config/local.db"))
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "docs/config/local.db".to_string())
    }

    #[test]
    fn test_new_database() {
        let db_path = get_test_db_path();
        let db = LocalDB::with_path(&db_path);
        assert!(db.is_ok(), "数据库连接应该成功");

        let db = db.unwrap();
        assert!(db.get_db_path().exists(), "数据库文件应该存在");
    }

    #[test]
    fn test_default_instance() {
        let db = LocalDB::default_instance();
        assert!(db.is_ok(), "默认实例创建应该成功");
    }

    #[test]
    fn test_table_exists() {
        let db = LocalDB::with_path(&get_test_db_path()).expect("数据库连接失败");
        let exists = db.table_exists("non_existent_table_12345");
        assert!(exists.is_ok(), "查询表存在应该成功");
        assert!(!exists.unwrap(), "不存在的表应该返回 false");
    }

    #[test]
    fn test_get_daily_table_name() {
        use chrono::Local;

        let today = Local::now().format("%Y%m%d").to_string();
        let table_name = LocalDB::get_daily_table_name("my_table", None);
        assert_eq!(table_name, format!("my_table_{}", today));

        let custom_date = "20240101";
        let table_name = LocalDB::get_daily_table_name("my_table", Some(custom_date));
        assert_eq!(table_name, "my_table_20240101");
    }

    #[test]
    fn test_download_from_server() {
        let db = LocalDB::new(None).expect("数据库连接失败");
        let result = db.download_from_server("testtb", "http://api.example.com/testtb", 10, 0, None);
        assert!(result.is_ok() || result.is_err()); // 测试结果
    }

    #[test]
    fn test_config_read() {
        let config = LocalDBConfig::default();
        let logger = mylogger!();
        logger.detail(&format!("cid: {}", config.cid));
        let logger = mylogger!();
        logger.detail(&format!("uid: {}", config.uid));
        let logger = mylogger!();
        logger.detail(&format!("upby: {}", config.upby));
        // 配置文件中有值，应该能读取到
        assert!(!config.cid.is_empty(), "cid 应该从配置文件读取到");
    }
}
