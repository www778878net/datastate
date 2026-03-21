//! Mysql78 - MySQL 数据库操作类
//!
//! 基于 koa78-base78 Mysql78.ts 的 Rust 实现
//! 提供连接池管理、预处理语句缓存、重试机制、事务操作

use chrono::Local;
use mysql::{Pool, PooledConn, prelude::Queryable, TxOpts};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// MySQL 配置
#[derive(Debug, Clone)]
pub struct MysqlConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub database: String,
    pub max_connections: u32,
    pub is_log: bool,
    pub is_count: bool,
}

impl Default for MysqlConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 3306,
            user: "root".to_string(),
            password: String::new(),
            database: String::new(),
            max_connections: 10,
            is_log: false,
            is_count: false,
        }
    }
}

/// 用户上传信息
#[derive(Debug, Clone, Default)]
pub struct MysqlUpInfo {
    pub apisys: String,
    pub apimicro: String,
    pub apiobj: String,
    pub uname: String,
    pub upid: String,
    pub uptime: String,
    pub debug: bool,
}

impl MysqlUpInfo {
    pub fn new() -> Self {
        Self {
            apisys: String::new(),
            apimicro: String::new(),
            apiobj: String::new(),
            uname: String::new(),
            upid: String::new(),
            uptime: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            debug: false,
        }
    }

    /// 生成新 ID
    pub fn new_id() -> String {
        format!("{}{}", Local::now().format("%Y%m%d%H%M%S"), rand_suffix())
    }
}

fn rand_suffix() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    format!("{:06x}", ns % 0xFFFFFF)
}

/// 更新结果
#[derive(Debug, Clone)]
pub struct MysqlUpdateResult {
    pub affected_rows: i64,
    pub error: Option<String>,
}

/// 插入结果
#[derive(Debug, Clone)]
pub struct MysqlInsertResult {
    pub insert_id: i64,
    pub error: Option<String>,
}

/// Mysql78 - MySQL 数据库操作类
#[derive(Clone)]
pub struct Mysql78 {
    pool: Option<Arc<Mutex<Pool>>>,
    config: MysqlConfig,
    host: String,
}

impl Default for Mysql78 {
    fn default() -> Self {
        Self::new(MysqlConfig::default())
    }
}

impl Mysql78 {
    /// 重试次数
    const MAX_RETRY_ATTEMPTS: u32 = 3;
    /// 重试延迟（毫秒）
    const RETRY_DELAY_MS: u64 = 1000;

    /// 创建新实例
    pub fn new(config: MysqlConfig) -> Self {
        let host = config.host.clone();
        Self {
            pool: None,
            config,
            host,
        }
    }

    /// 初始化连接池
    pub fn initialize(&mut self) -> Result<(), String> {
        if self.config.database.is_empty() {
            return Err("database name is required".to_string());
        }
        if self.config.password.is_empty() {
            return Err("password is required".to_string());
        }

        let url = format!(
            "mysql://{}:{}@{}:{}/{}?connect_timeout=30&wait_timeout=30",
            self.config.user,
            self.config.password,
            self.config.host,
            self.config.port,
            self.config.database
        );

        let pool = Pool::new(url.as_str())
            .map_err(|e| format!("创建连接池失败: {}", e))?;

        self.pool = Some(Arc::new(Mutex::new(pool)));
        Ok(())
    }

    /// 获取连接（带重试）
    fn get_connection_with_retry(&self) -> Result<PooledConn, String> {
        let pool = self.get_pool()?;

        let mut attempts = 0;
        let mut last_error = String::new();

        while attempts < Self::MAX_RETRY_ATTEMPTS {
            let pool_guard = pool.lock().map_err(|e| format!("获取连接池锁失败: {}", e))?;
            match pool_guard.get_conn() {
                Ok(conn) => return Ok(conn),
                Err(e) => {
                    attempts += 1;
                    last_error = format!("连接尝试 {} 失败: {}", attempts, e);
                    drop(pool_guard);
                    if attempts < Self::MAX_RETRY_ATTEMPTS {
                        std::thread::sleep(Duration::from_millis(Self::RETRY_DELAY_MS));
                    }
                }
            }
        }

        Err(format!("获取连接失败，重试 {} 次后仍失败: {}", Self::MAX_RETRY_ATTEMPTS, last_error))
    }

    /// 获取连接池
    fn get_pool(&self) -> Result<Arc<Mutex<Pool>>, String> {
        self.pool.clone().ok_or_else(|| "pool null".to_string())
    }

    /// 创建系统常用表
    pub fn creat_tb(&self, _up: &MysqlUpInfo) -> Result<String, String> {
        let mut conn = self.get_connection_with_retry()?;

        let cmdtext1 = r#"CREATE TABLE IF NOT EXISTS `sys_warn` (
            `uid` varchar(36) NOT NULL DEFAULT '',
            `kind` varchar(100) NOT NULL DEFAULT '',
            `apimicro` varchar(100) NOT NULL DEFAULT '',
            `apiobj` varchar(100) NOT NULL DEFAULT '',
            `content` text NOT NULL,
            `upid` varchar(36) NOT NULL DEFAULT '',
            `upby` varchar(50) DEFAULT '',
            `uptime` datetime NOT NULL,
            `idpk` int(11) NOT NULL AUTO_INCREMENT,
            `id` varchar(36) NOT NULL,
            `remark` varchar(200) NOT NULL DEFAULT '',
            `remark2` varchar(200) NOT NULL DEFAULT '',
            `remark3` varchar(200) NOT NULL DEFAULT '',
            `remark4` varchar(200) NOT NULL DEFAULT '',
            `remark5` varchar(200) NOT NULL DEFAULT '',
            `remark6` varchar(200) NOT NULL DEFAULT '',
            PRIMARY KEY (`idpk`),
            UNIQUE KEY `u_id` (`id`)
        ) ENGINE=InnoDB AUTO_INCREMENT=0 DEFAULT CHARSET=utf8"#;

        let cmdtext2 = r#"CREATE TABLE IF NOT EXISTS `sys_sql` (
            `cid` varchar(36) NOT NULL DEFAULT '',
            `apisys` varchar(50) NOT NULL DEFAULT '',
            `apimicro` varchar(50) NOT NULL DEFAULT '',
            `apiobj` varchar(50) NOT NULL DEFAULT '',
            `cmdtext` varchar(200) NOT NULL,
            `uname` varchar(50) NOT NULL DEFAULT '',
            `num` int(11) NOT NULL DEFAULT '0',
            `dlong` int(32) NOT NULL DEFAULT '0',
            `downlen` bigint NOT NULL DEFAULT '0',
            `upby` varchar(50) NOT NULL DEFAULT '',
            `cmdtextmd5` varchar(50) NOT NULL DEFAULT '',
            `uptime` datetime NOT NULL,
            `idpk` int(11) NOT NULL AUTO_INCREMENT,
            `id` varchar(36) NOT NULL,
            `remark` varchar(200) NOT NULL DEFAULT '',
            `remark2` varchar(200) NOT NULL DEFAULT '',
            `remark3` varchar(200) NOT NULL DEFAULT '',
            `remark4` varchar(200) NOT NULL DEFAULT '',
            `remark5` varchar(200) NOT NULL DEFAULT '',
            `remark6` varchar(200) NOT NULL DEFAULT '',
            PRIMARY KEY (`idpk`),
            UNIQUE KEY `u_v_sys_obj_cmdtext` (`apisys`,`apimicro`,`apiobj`,`cmdtext`) USING BTREE,
            UNIQUE KEY `u_id` (`id`)
        ) ENGINE=InnoDB AUTO_INCREMENT=0 DEFAULT CHARSET=utf8"#;

        conn.query_drop(cmdtext1)
            .map_err(|e| format!("创建 sys_warn 表失败: {}", e))?;
        conn.query_drop(cmdtext2)
            .map_err(|e| format!("创建 sys_sql 表失败: {}", e))?;

        Ok("ok".to_string())
    }

    /// 查询数据
    pub fn do_get(
        &self,
        cmdtext: &str,
        params: Vec<Value>,
        _up: &MysqlUpInfo,
    ) -> Result<Vec<HashMap<String, Value>>, String> {
        let mut conn = self.get_connection_with_retry()?;

        let mysql_params = json_values_to_mysql_params(&params);

        let query_result = conn
            .exec_iter(cmdtext, mysql_params)
            .map_err(|e| format!("查询失败: {} (cmdtext: {})", e, cmdtext))?;

        let mut results = Vec::new();
        for row in query_result {
            let row = row.map_err(|e| format!("读取行失败: {}", e))?;
            let mut map = HashMap::new();
            for (i, col) in row.columns().iter().enumerate() {
                let value = row.get::<mysql::Value, _>(i).unwrap_or(mysql::Value::NULL);
                map.insert(col.name_str().to_string(), mysql_value_to_json(value));
            }
            results.push(map);
        }

        Ok(results)
    }

    /// 更新数据
    pub fn do_m(
        &self,
        cmdtext: &str,
        params: Vec<Value>,
        _up: &MysqlUpInfo,
    ) -> Result<MysqlUpdateResult, String> {
        let mut conn = self.get_connection_with_retry()?;

        let mysql_params = json_values_to_mysql_params(&params);

        conn.exec_drop(cmdtext, mysql_params.clone())
            .map_err(|e| format!("更新失败: {} (cmdtext: {})", e, cmdtext))?;

        let affected_rows = conn.affected_rows();

        if affected_rows == 0 {
            return Ok(MysqlUpdateResult {
                affected_rows: 0,
                error: Some(format!(
                    "更新失败，没有找到匹配的记录 (cmdtext: {})",
                    cmdtext
                )),
            });
        }

        Ok(MysqlUpdateResult {
            affected_rows: affected_rows as i64,
            error: None,
        })
    }

    /// 插入数据
    pub fn do_m_add(
        &self,
        cmdtext: &str,
        params: Vec<Value>,
        _up: &MysqlUpInfo,
    ) -> Result<MysqlInsertResult, String> {
        let mut conn = self.get_connection_with_retry()?;

        let mysql_params = json_values_to_mysql_params(&params);

        conn.exec_drop(cmdtext, mysql_params)
            .map_err(|e| format!("插入失败: {} (cmdtext: {})", e, cmdtext))?;

        let insert_id = conn.last_insert_id();
        let affected_rows = conn.affected_rows();

        if affected_rows == 0 {
            return Ok(MysqlInsertResult {
                insert_id: 0,
                error: Some(format!("插入失败 (cmdtext: {})", cmdtext)),
            });
        }

        Ok(MysqlInsertResult {
            insert_id: insert_id as i64,
            error: None,
        })
    }

    /// 执行事务
    pub fn do_t(
        &self,
        cmds: Vec<String>,
        values_list: Vec<Vec<Value>>,
        errtexts: Vec<String>,
        _logtext: &str,
        _logvalue: Vec<String>,
        _up: &MysqlUpInfo,
    ) -> Result<String, String> {
        if cmds.len() != values_list.len() || cmds.len() != errtexts.len() {
            return Err("cmds, values, errtexts 长度不一致".to_string());
        }

        let mut conn = self.get_connection_with_retry()?;
        let mut tx = conn
            .start_transaction(TxOpts::default())
            .map_err(|e| format!("开始事务失败: {}", e))?;

        for (i, cmd) in cmds.iter().enumerate() {
            let mysql_params = json_values_to_mysql_params(&values_list[i]);
            let result = tx.exec_drop(cmd, mysql_params);
            if let Err(e) = result {
                drop(tx);
                return Err(format!("事务执行失败: {} - {}", errtexts[i], e));
            }

            let affected = tx.affected_rows();
            if affected == 0 {
                drop(tx);
                return Err(format!("事务执行失败: {}", errtexts[i]));
            }
        }

        tx.commit()
            .map_err(|e| format!("提交事务失败: {}", e))?;

        Ok("ok".to_string())
    }

    /// 关闭连接池
    pub fn close(&mut self) {
        self.pool = None;
    }

    /// 获取主机地址
    pub fn get_host(&self) -> &str {
        &self.host
    }

    /// 设置日志开关
    pub fn set_log(&mut self, is_log: bool) {
        self.config.is_log = is_log;
    }

    /// 设置计数开关
    pub fn set_count(&mut self, is_count: bool) {
        self.config.is_count = is_count;
    }
}

/// 将 Vec<Value> 转换为 mysql::Params
fn json_values_to_mysql_params(values: &[Value]) -> mysql::Params {
    if values.is_empty() {
        return mysql::Params::Empty;
    }

    let mut params = Vec::new();
    for (i, v) in values.iter().enumerate() {
        params.push((format!("p{}", i), json_to_mysql_value(v)));
    }
    mysql::Params::from(params)
}

/// 将 serde_json::Value 转换为 mysql::Value
fn json_to_mysql_value(value: &Value) -> mysql::Value {
    match value {
        Value::Null => mysql::Value::NULL,
        Value::Bool(b) => mysql::Value::Int(if *b { 1 } else { 0 }),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                mysql::Value::Int(i)
            } else if let Some(f) = n.as_f64() {
                mysql::Value::Double(f)
            } else {
                mysql::Value::NULL
            }
        }
        Value::String(s) => mysql::Value::Bytes(s.as_bytes().to_vec()),
        Value::Array(arr) => {
            mysql::Value::Bytes(serde_json::to_string(arr).unwrap_or_default().into_bytes())
        }
        Value::Object(obj) => {
            mysql::Value::Bytes(serde_json::to_string(obj).unwrap_or_default().into_bytes())
        }
    }
}

/// 将 mysql::Value 转换为 serde_json::Value
fn mysql_value_to_json(value: mysql::Value) -> Value {
    match value {
        mysql::Value::NULL => Value::Null,
        mysql::Value::Bytes(b) => {
            if let Ok(s) = String::from_utf8(b.clone()) {
                if let Ok(json) = serde_json::from_str(&s) {
                    return json;
                }
                Value::String(s)
            } else {
                Value::String(format!("<binary:{} bytes>", b.len()))
            }
        }
        mysql::Value::Int(i) => Value::Number(i.into()),
        mysql::Value::UInt(u) => Value::Number(u.into()),
        mysql::Value::Float(f) => serde_json::json!(f),
        mysql::Value::Double(d) => serde_json::json!(d),
        mysql::Value::Date(year, month, day, hour, min, sec, micro) => {
            Value::String(format!(
                "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:06}",
                year, month, day, hour, min, sec, micro
            ))
        }
        mysql::Value::Time(neg, days, hours, minutes, seconds, microseconds) => {
            let sign = if neg { "-" } else { "" };
            Value::String(format!(
                "{}{} days {:02}:{:02}:{:02}.{:06}",
                sign, days, hours, minutes, seconds, microseconds
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = MysqlConfig::default();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 3306);
        assert_eq!(config.user, "root");
        assert_eq!(config.max_connections, 10);
    }

    #[test]
    fn test_upinfo_new() {
        let up = MysqlUpInfo::new();
        assert!(!up.uptime.is_empty());
    }

    #[test]
    fn test_new_id() {
        let id1 = MysqlUpInfo::new_id();
        let id2 = MysqlUpInfo::new_id();
        assert_ne!(id1, id2, "生成的 ID 应该是唯一的");
    }

    #[test]
    fn test_mysql78_new() {
        let config = MysqlConfig::default();
        let db = Mysql78::new(config);
        assert!(db.get_host().starts_with("127"));
    }

    #[test]
    fn test_mysql78_default() {
        let db = Mysql78::default();
        assert!(db.get_host().starts_with("127"));
    }
}