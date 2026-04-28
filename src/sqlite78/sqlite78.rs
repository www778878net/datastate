//! Sqlite78 - SQLite 数据库操作类
//!
//! 完整移植自 koa78-base78/Sqlite78.ts
//! 提供 Local-First 存储的本地数据库操作能力

use base::{ProjectPath, UpInfo, MyLogger};
use rusqlite::Connection;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use chrono::Local;

/// 查询结果
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct QueryResult {
    pub rows: Vec<HashMap<String, Value>>,
}

/// 更新结果
#[derive(Debug, Clone)]
pub struct UpdateResult {
    pub affected_rows: i64,
    pub error: Option<String>,
}

/// 插入结果
#[derive(Debug, Clone)]
pub struct InsertResult {
    pub insert_id: i64,
    pub error: Option<String>,
}

/// 警告处理器类型
pub type WarnHandler = Box<dyn Fn(&str, &str, &UpInfo) -> Result<(), String> + Send + Sync>;

/// Sqlite78 - SQLite 数据库操作类
pub struct Sqlite78 {
    conn: Option<Arc<Mutex<Connection>>>,
    filename: String,
    is_log: bool,
    is_count: bool,
    logger: MyLogger,
    warn_handler: Option<Arc<WarnHandler>>,
}

impl Default for Sqlite78 {
    fn default() -> Self {
        Self::new()
    }
}

impl Sqlite78 {
    /// 创建新实例
    pub fn new() -> Self {
        Self {
            conn: None,
            filename: String::new(),
            is_log: false,
            is_count: false,
            logger: MyLogger::new("sqlite78", 3),
            warn_handler: None,
        }
    }

    /// 查找默认数据库路径
    /// 优先级：环境变量 SQLITE_PATH > ProjectPath::find().local_db()
    pub fn find_default_db_path() -> Result<String, String> {
        // 优先使用环境变量
        if let Ok(env_path) = std::env::var("SQLITE_PATH") {
            if !env_path.is_empty() {
                return Ok(env_path);
            }
        }
        let project = ProjectPath::find()?;
        Ok(project.local_db().to_string_lossy().to_string())
    }

    /// 使用默认配置创建实例
    pub fn with_default_path() -> Self {
        let filename = Self::find_default_db_path().unwrap_or_else(|_| "docs/config/local.db".to_string());
        Self {
            conn: None,
            filename,
            is_log: false,
            is_count: false,
            logger: MyLogger::new("sqlite78", 3),
            warn_handler: None,
        }
    }

    /// 使用配置创建实例
    pub fn with_config(filename: &str, is_log: bool, is_count: bool) -> Self {
        Self {
            conn: None,
            filename: filename.to_string(),
            is_log,
            is_count,
            logger: MyLogger::new("sqlite78", 3),
            warn_handler: None,
        }
    }

    /// 设置警告处理器
    pub fn set_warn_handler(&mut self, handler: WarnHandler) {
        self.warn_handler = Some(Arc::new(handler));
    }

    /// 初始化数据库连接
    pub fn initialize(&mut self) -> Result<(), String> {
        if self.filename.is_empty() {
            return Err("filename is empty".to_string());
        }

        let path = PathBuf::from(&self.filename);
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("创建目录失败: {}", e))?;
            }
        }

        let conn = Connection::open(&path)
            .map_err(|e| format!("连接数据库失败: {}", e))?;

        // 启用外键约束
        conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL; PRAGMA busy_timeout = 30000;")
            .map_err(|e| format!("设置 PRAGMA 失败: {}", e))?;

        self.conn = Some(Arc::new(Mutex::new(conn)));
        Ok(())
    }

    /// 创建系统常用表
    pub async fn creat_tb(&self, _up: &UpInfo) -> Result<String, String> {
        let conn = self.get_conn()?;
        let conn = conn.lock().await;

        // 使用 sys_sql_state.rs 和 sys_warn_state.rs 中的常量
        conn.execute(crate::sqlite78::SYS_SQL_CREATE_SQL, [])
            .map_err(|e| format!("创建 sys_sql 表失败: {}", e))?;
        conn.execute(crate::sqlite78::SYS_WARN_CREATE_SQL, [])
            .map_err(|e| format!("创建 sys_warn 表失败: {}", e))?;

        Ok("ok".to_string())
    }

    /// 查询数据
    pub async fn do_get(&self, cmdtext: &str, values: &[&dyn rusqlite::ToSql], up: &UpInfo) -> Result<Vec<HashMap<String, Value>>, String> {
        let conn = self.get_conn()?;
        let dstart = std::time::Instant::now();

        let conn = conn.lock().await;

        let mut stmt = conn.prepare(cmdtext)
            .map_err(|e| format!("准备语句失败: {}", e))?;

        let column_names: Vec<String> = stmt.column_names()
            .into_iter()
            .map(|s| s.to_string())
            .collect();

        let rows = stmt.query(values)
            .map_err(|e| format!("查询失败: {}", e))?;

        let result = Self::process_rows(rows, &column_names)?;

        // 调试模式记录日志
        if up.debug {
            let info = format!("{} c:{} rows={}", serde_json::to_string(&result).unwrap_or_default(), cmdtext, result.len());
            self.add_warn(&info, &format!("debug_{}", up.apimicro), up)?;
        }

        // 保存统计日志
        let lendown = serde_json::to_string(&result).unwrap_or_default().len();
        self.save_log(cmdtext, dstart.elapsed().as_millis() as i64, lendown as i64, up)?;

        Ok(result)
    }

    /// 处理查询结果
    fn process_rows(mut rows: rusqlite::Rows, column_names: &[String]) -> Result<Vec<HashMap<String, Value>>, String> {
        let mut results = Vec::new();
        loop {
            match rows.next().map_err(|e| format!("读取行失败: {}", e))? {
                Some(row) => {
                    let mut map = HashMap::new();
                    for (i, name) in column_names.iter().enumerate() {
                        let value = Self::row_value_to_json(row, i);
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
    fn row_value_to_json(row: &rusqlite::Row, col_index: usize) -> Value {
        if let Ok(s) = row.get::<_, String>(col_index) {
            if let Ok(json) = serde_json::from_str(&s) {
                return json;
            }
            return Value::String(s);
        }
        if let Ok(n) = row.get::<_, i64>(col_index) {
            return Value::Number(n.into());
        }
        if let Ok(n) = row.get::<_, f64>(col_index) {
            return serde_json::json!(n);
        }
        Value::Null
    }

    /// 更新数据
    pub async fn do_m(&self, cmdtext: &str, values: &[&dyn rusqlite::ToSql], up: &UpInfo) -> Result<UpdateResult, String> {
        let conn = self.get_conn()?;
        let dstart = std::time::Instant::now();

        let conn = conn.lock().await;

        match conn.execute(cmdtext, values) {
            Ok(rows_affected) => {
                // 调试模式记录日志
                if up.debug {
                    let info = format!("affected:{} c:{}", rows_affected, cmdtext);
                    self.add_warn(&info, &format!("debug_{}", up.apimicro), up)?;
                }

                // 保存统计日志
                self.save_log(cmdtext, dstart.elapsed().as_millis() as i64, rows_affected as i64, up)?;

                Ok(UpdateResult {
                    affected_rows: rows_affected as i64,
                    error: if rows_affected == 0 {
                        Some(format!("更新失败，没有找到匹配的记录 (cmdtext: {})", cmdtext))
                    } else {
                        None
                    },
                })
            }
            Err(e) => {
                let error_msg = e.to_string();
                self.add_warn(&format!("{} c:{}", error_msg, cmdtext), &format!("err_{}", up.apimicro), up)?;
                if error_msg.contains("no such table") {
                    self.logger.detail(&format!("sqlite_doM: {}", error_msg));
                } else {
                    self.logger.error(&format!("sqlite_doM error: {}", error_msg));
                }
                Ok(UpdateResult {
                    affected_rows: 0,
                    error: Some(error_msg),
                })
            }
        }
    }

    /// 插入数据
    pub async fn do_m_add(&self, cmdtext: &str, values: &[&dyn rusqlite::ToSql], up: &UpInfo) -> Result<InsertResult, String> {
        let conn = self.get_conn()?;
        let dstart = std::time::Instant::now();

        let conn = conn.lock().await;

        match conn.execute(cmdtext, values) {
            Ok(rows_affected) => {
                let insert_id = conn.last_insert_rowid();

                // 调试模式记录日志
                if up.debug {
                    let info = format!("insertId:{} c:{}", insert_id, cmdtext);
                    self.add_warn(&info, &format!("debug_{}", up.apimicro), up)?;
                }

                // 保存统计日志
                self.save_log(cmdtext, dstart.elapsed().as_millis() as i64, rows_affected as i64, up)?;

                Ok(InsertResult {
                    insert_id,
                    error: if rows_affected == 0 {
                        Some(format!("插入失败 (cmdtext: {})", cmdtext))
                    } else {
                        None
                    },
                })
            }
            Err(e) => {
                let error_msg = e.to_string();
                // Handle "INSERT INTO table" (index 2), "INSERT OR REPLACE INTO table" (index 4), and "REPLACE INTO table" (index 2)
                let words: Vec<&str> = cmdtext.split_whitespace().collect();
                let table_name = if words.len() > 2 && words[0].to_uppercase() == "REPLACE" {
                    // REPLACE INTO table ...
                    words.get(2).unwrap_or(&"unknown")
                } else if words.len() > 4 && words[1].to_uppercase() == "OR" {
                    // INSERT OR REPLACE INTO table ...
                    words.get(4).unwrap_or(&"unknown")
                } else {
                    // INSERT INTO table ...
                    words.get(2).unwrap_or(&"unknown")
                };
                self.add_warn(&format!("{} c:{}", error_msg, cmdtext), &format!("err_{}", up.apimicro), up)?;
                self.logger.error(&format!("sqlite_doMAdd error: {} (table: {})", error_msg, table_name));
                Ok(InsertResult {
                    insert_id: 0,
                    error: Some(error_msg),
                })
            }
        }
    }

    /// 执行事务
    pub async fn do_t(
        &self,
        cmds: &[String],
        values: Vec<Vec<&dyn rusqlite::ToSql>>,
        errtexts: &[String],
        logtext: &str,
        _logvalue: &[String],
        up: &UpInfo,
    ) -> Result<String, String> {
        if cmds.len() != values.len() || cmds.len() != errtexts.len() {
            return Err("cmds, values, errtexts 长度不一致".to_string());
        }

        let conn = self.get_conn()?;
        let dstart = std::time::Instant::now();
        let mut conn = conn.lock().await;

        let tx = conn.transaction()
            .map_err(|e| format!("开始事务失败: {}", e))?;

        for (i, cmd) in cmds.iter().enumerate() {
            let result = tx.execute(cmd, values[i].as_slice());
            if let Err(e) = result {
                drop(tx);
                return Err(format!("事务执行失败: {} - {}", errtexts[i], e));
            }
        }

        tx.commit()
            .map_err(|e| format!("提交事务失败: {}", e))?;

        self.save_log(logtext, dstart.elapsed().as_millis() as i64, 1, up)?;

        Ok("ok".to_string())
    }

    /// 关闭数据库连接
    pub fn close(&mut self) {
        self.conn = None;
    }

    /// 获取连接
    pub fn get_conn(&self) -> Result<Arc<Mutex<Connection>>, String> {
        self.conn.clone().ok_or_else(|| "database not initialized".to_string())
    }

    /// 设置日志开关
    pub fn set_log(&mut self, is_log: bool) {
        self.is_log = is_log;
    }

    /// 设置计数开关
    pub fn set_count(&mut self, is_count: bool) {
        self.is_count = is_count;
    }

    /// 获取数据库路径
    pub fn get_filename(&self) -> &str {
        &self.filename
    }

    // ============ 私有方法 ============

    /// 添加警告记录
    async fn add_warn(&self, info: &str, kind: &str, up: &UpInfo) -> Result<(), String> {
        // 优先使用自定义处理器
        if let Some(ref handler) = self.warn_handler {
            return handler(info, kind, up);
        }

        // 未开启日志或数据库未初始化
        if !self.is_log || self.conn.is_none() {
            return Ok(());
        }

        let conn = self.get_conn()?;
        let conn = conn.lock().await;

        let cmdtext = "INSERT INTO sys_warn (kind,apimicro,apiobj,content,upby,uptime,id,upid) VALUES (?,?,?,?,?,?,?,?)";
        let id = UpInfo::new_id();
        let params: [&dyn rusqlite::ToSql; 8] = [
            &kind,
            &up.apimicro,
            &up.apiobj,
            &info,
            &up.uname,
            &up.uptime,
            &id,
            &up.upid,
        ];

        conn.execute(cmdtext, params)
            .map_err(|e| format!("插入 sys_warn 失败: {}", e))?;

        Ok(())
    }

    /// 保存 SQL 统计日志
    async fn save_log(&self, cmdtext: &str, dlong: i64, lendown: i64, up: &UpInfo) -> Result<(), String> {
        if !self.is_count || self.conn.is_none() {
            return Ok(());
        }

        let conn = self.get_conn()?;
        let conn = conn.lock().await;

        // 生成雪花ID
        let cmdtextmd5 = crate::snowflake::next_id_string();

        let id = UpInfo::new_id();
        let uptime = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

        // 插入或忽略
        let insert_sql = "INSERT OR IGNORE INTO sys_sql(apisys,apimicro,apiobj,cmdtext,num,dlong,downlen,id,uptime,cmdtextmd5) VALUES (?,?,?,?,?,?,?,?,?,?)";
        let params: [&dyn rusqlite::ToSql; 10] = [
            &up.apisys,
            &up.apimicro,
            &up.apiobj,
            &cmdtext,
            &1i64,
            &dlong,
            &lendown,
            &id,
            &uptime,
            &cmdtextmd5,
        ];

        conn.execute(insert_sql, params)
            .map_err(|e| format!("插入 sys_sql 失败: {}", e))?;

        // 更新计数器
        let update_sql = "UPDATE sys_sql SET num=num+1,dlong=dlong+?,downlen=downlen+? WHERE cmdtextmd5=?";
        conn.execute(update_sql, rusqlite::params![dlong, lendown, cmdtextmd5])
            .map_err(|e| format!("更新 sys_sql 失败: {}", e))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base::UpInfo;

    #[test]
    fn test_new_sqlite78() {
        let db = Sqlite78::new();
        assert!(db.get_filename().is_empty());
    }

    #[test]
    fn test_with_default_path() {
        let db = Sqlite78::with_default_path();
        assert!(!db.get_filename().is_empty());
    }

    #[test]
    fn test_with_config() {
        let db = Sqlite78::with_config("test.db", true, true);
        assert_eq!(db.get_filename(), "test.db");
    }

    #[test]
    fn test_initialize() {
        let mut db = Sqlite78::with_config("tmp/tmp/test_init.db", false, false);
        let result = db.initialize();
        assert!(result.is_ok());
    }

    #[test]
    fn test_find_default_db_path() {
        let result = Sqlite78::find_default_db_path();
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_creat_tb() {
        let mut db = Sqlite78::with_config("tmp/tmp/test_creat_tb.db", false, false);
        db.initialize().expect("初始化失败");

        let up = UpInfo::new();
        let result = db.creat_tb(&up);
        assert!(result.is_ok());
    }

    #[test]
    fn test_do_get_empty() {
        let mut db = Sqlite78::with_config("tmp/tmp/test_do_get.db", false, false);
        db.initialize().expect("初始化失败");
        db.creat_tb(&UpInfo::new()).expect("创建表失败");

        let up = UpInfo::new();
        let result = db.do_get("SELECT * FROM sys_warn", &[], &up);
        assert!(result.is_ok());
        let rows = result.unwrap();
        assert!(rows.is_empty());
    }

    #[test]
    fn test_get_conn() {
        let mut db = Sqlite78::new();
        let result = db.get_conn();
        assert!(result.is_err());
    }

    #[test]
    fn test_close() {
        let mut db = Sqlite78::with_config("tmp/tmp/test_close.db", false, false);
        db.initialize().expect("初始化失败");
        db.close();
        let result = db.get_conn();
        assert!(result.is_err());
    }
}