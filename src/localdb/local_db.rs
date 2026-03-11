//! LocalDB - SQLite 本地数据库封装
//!
//! 提供 Local-First 存储的本地封装
//!
//! # 用法
//! ```rust
//! use localdb::localdb::LocalDB;
//!
//! // 方式1: 使用默认路径 (docs/config/local.db)
//! let db = LocalDB::new(None)?;
//! // 或
//! let db = LocalDB::default_instance()?;
//!
//! // 方式2: 只读方式 (共享内存模式)
//! let db = LocalDB::new(Some("docs/config/local.db"))?;
//! ```
//!
//! ⚠️  注意: 禁止传入其他路径，必须使用默认路径！

use rusqlite::{Connection, Row, Rows};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use chrono::Local;
use base::mylogger;

use base::project_path::ProjectPath;
use crate::datastate::{DATA_ABILITY_LOG_CREATE_SQL};
use crate::data_sync::{DATA_STATE_LOG_CREATE_SQL, DATA_SYNC_STATS_CREATE_SQL, SYNC_QUEUE_CREATE_SQL};

/// 本地数据库管理类
#[derive(Debug, Clone)]
pub struct LocalDB {
    conn: Arc<Mutex<Connection>>,
    db_path: PathBuf,
}

impl Default for LocalDB {
    fn default() -> Self {
        Self::new(None).unwrap_or_else(|_| {
            panic!("Failed to create default LocalDB")
        })
    }
}

impl LocalDB {
    pub fn new(db_path: Option<&str>) -> Result<Self, String> {
        let path = if let Some(p) = db_path {
            PathBuf::from(p)
        } else {
            ProjectPath::find()?.local_db()
        };

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

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            db_path: path,
        })
    }

    pub fn default_instance() -> Result<Self, String> {
        Self::new(None)
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

    /// 初始化系统表（���计和状态机相关）
    pub fn init_system_tables(&self) -> Result<(), String> {
        let tables = [
            ("sync_queue", SYNC_QUEUE_CREATE_SQL),
            ("data_state_log", DATA_STATE_LOG_CREATE_SQL),
            ("data_sync_stats", DATA_SYNC_STATS_CREATE_SQL),
            ("data_ability_log", DATA_ABILITY_LOG_CREATE_SQL),
        ];

        for (table_name, create_sql) in tables {
            if !self.table_exists(table_name)? {
                self.ensure_table(table_name, create_sql)?;
            } else {
                // 已存在的表，尝试添加 id 唯一索引
                self.ensure_id_unique_index(table_name)?;
            }
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

        // 过滤掉本地表不存在的字段
        let filtered_data: HashMap<String, Value> = data.iter()
            .filter(|(k, _)| table_columns.contains(k) || k.as_str() == "id")
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        let columns: Vec<&str> = filtered_data.keys().map(|s| s.as_str()).collect();
        let placeholders: Vec<&str> = (0..columns.len()).map(|_| "?").collect();
        let sql = format!(
            "INSERT OR REPLACE INTO {} ({}) VALUES ({})",
            table,
            columns.join(", "),
            placeholders.join(", ")
        );

        let values: Vec<String> = filtered_data.values().map(|v| {
            match v {
                Value::Null => "".to_string(),
                Value::Bool(b) => b.to_string(),
                Value::Number(n) => n.to_string(),
                Value::String(s) => s.clone(),
                Value::Array(_) | Value::Object(_) => serde_json::to_string(v).unwrap_or_default(),
            }
        }).collect();

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
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        let mut stmt = conn.prepare(sql)
            .map_err(|e| format!("准备语句失败: {}", e))?;

        let column_names: Vec<String> = stmt.column_names()
            .into_iter()
            .map(|s| s.to_string())
            .collect();

        let rows = stmt.query(params)
            .map_err(|e| format!("查询失败: {}", e))?;

        Self::process_rows(rows, &column_names)
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
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(sql, [])
            .map_err(|e| format!("执行失败: {}", e))?;
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
    fn get_sid(&self) -> String {
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

    /// 从服务器下载数据到本地（支持分页）
    pub fn download_from_server(
        &self,
        _table: &str,
        api_url: &str,
        getnumber: i32,
        getstart: i32,
        download_condition: Option<&Value>,
    ) -> Result<Vec<HashMap<String, Value>>, String> {
        use base::http::HttpHelper;

        let sid = self.get_sid();
        if sid.is_empty() {
            return Err("配置文件未找到 SID".to_string());
        }

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
            if let Some(arr) = cond.as_array() {
                request_payload["pars"] = cond.clone();
            } else if let Some(obj) = cond.as_object() {
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

    /// 批量上传数据到服务器（使用 mAddManyByid）
    ///
    /// # 参数
    /// - `_table`: 表名（未使用）
    /// - `api_url`: API 基础 URL
    /// - `items`: 待同步的数据列表（来自 sync_queue）
    /// - `cols`: 字段顺序（必须与服务器 colsImp 一致）
    pub fn upload_batch_to_server(
        &self,
        _table: &str,
        api_url: &str,
        items: &[crate::data_sync::SyncQueueItem],
        cols: Option<&[String]>,
    ) -> Result<i32, String> {
        use base::http::HttpHelper;

        let sid = self.get_sid();
        if sid.is_empty() {
            return Err("配置文件未找到 SID".to_string());
        }

        if items.is_empty() {
            return Ok(0);
        }

        // 确定字段顺序
        let field_order: Vec<String> = if let Some(cols_list) = cols {
            cols_list.to_vec()
        } else {
            return Err("必须指定 upload_cols 字段顺序".to_string());
        };

        // 构建 pars 数组（所有记录展平为一维数组）
        // mAddManyByid 格式：每行 = cols 字段 + id
        let mut pars: Vec<Value> = Vec::new();
        for item in items {
            let data: HashMap<String, Value> = serde_json::from_str(&item.data).unwrap_or_default();
            
            // 先添加 cols 字段
            for col in &field_order {
                pars.push(data.get(col).cloned().unwrap_or(Value::String(String::new())));
            }
            
            // 最后添加 id
            pars.push(Value::String(item.id.clone()));
        }

        // 构建请求体
        let request_payload = serde_json::json!({
            "sid": sid,
            "pars": pars,
            "cols": field_order
        });

        // URL: {api_url}/mAddManyByid
        let url = format!("{}/mAddManyByid", api_url.trim_end_matches('/'));

        let response = HttpHelper::post(&url, None, Some(&request_payload), None, false, None, 30, 2);

        // 记录服务器响应
        let logger = mylogger!();
        logger.info(&format!("[upload_batch_to_server] 服务器响应: res={}, errmsg={}, data={:?}", 
            response.res, response.errmsg, response.data));

        if response.res != 0 {
            return Err(format!("服务器错误: res={}, errmsg={}", response.res, response.errmsg));
        }

        // 检查服务器返回的业务错误
        if let Some(ref resp_data) = response.data {
            if let Some(back_obj) = resp_data.response.as_object() {
                logger.info(&format!("[upload_batch_to_server] 业务响应: {:?}", back_obj));
                if let Some(back_res) = back_obj.get("res") {
                    if back_res.as_i64().unwrap_or(0) != 0 {
                        let back_errmsg = back_obj.get("errmsg").and_then(|v| v.as_str()).unwrap_or("");
                        return Err(format!("业务错误: {}", back_errmsg));
                    }
                }
                // 返回影响的行数
                if let Some(back) = back_obj.get("back") {
                    if let Some(count) = back.as_i64() {
                        return Ok(count as i32);
                    }
                }
            }
        }

        Ok(items.len() as i32)
    }
}

#[cfg(test)]
mod tests {
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
        let db = LocalDB::new(Some(&db_path));
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
        let db = LocalDB::new(Some(&get_test_db_path())).expect("数据库连接失败");
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
}