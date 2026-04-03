//! DataSyncMysql - MySQL 版本同步组件
//!
//! 职责：同步队列管理、状态变更日志、同步统计
//! MySQL 版本，使用 Mysql78 作为数据库后端

use crate::mysql78::{Mysql78, MysqlConfig, MysqlUpInfo};
use chrono::Local;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

// ========== 建表 SQL (MySQL 语法) ==========

/// synclog 表建表 SQL - 同步日志表（MySQL 版本）
pub const SYNCLOG_CREATE_SQL_MYSQL: &str = r#"
CREATE TABLE IF NOT EXISTS `synclog` (
    `idpk` INT NOT NULL AUTO_INCREMENT,
    `apisys` VARCHAR(50) NOT NULL DEFAULT 'v1',
    `apimicro` VARCHAR(50) NOT NULL DEFAULT 'iflow',
    `apiobj` VARCHAR(50) NOT NULL DEFAULT 'synclog',
    `tbname` VARCHAR(100) NOT NULL DEFAULT '',
    `action` VARCHAR(20) NOT NULL DEFAULT '',
    `cmdtext` TEXT NOT NULL,
    `params` TEXT NOT NULL,
    `idrow` VARCHAR(100) NOT NULL DEFAULT '',
    `worker` VARCHAR(50) NOT NULL DEFAULT '',
    `synced` INT NOT NULL DEFAULT 0,
    `lasterrinfo` TEXT NOT NULL,
    `cmdtextmd5` VARCHAR(50) NOT NULL DEFAULT '',
    `num` INT NOT NULL DEFAULT 0,
    `dlong` BIGINT NOT NULL DEFAULT 0,
    `downlen` BIGINT NOT NULL DEFAULT 0,
    `id` VARCHAR(50) NOT NULL DEFAULT '',
    `upby` VARCHAR(50) NOT NULL DEFAULT '',
    `uptime` DATETIME NOT NULL,
    `cid` VARCHAR(50) NOT NULL DEFAULT '',
    PRIMARY KEY (`idpk`),
    INDEX `idx_tbname_synced` (`tbname`, `synced`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4
"#;

/// data_state_log 表 - 状态变更日志（MySQL 版本）
pub const DATA_STATE_LOG_CREATE_SQL_MYSQL: &str = r#"
CREATE TABLE IF NOT EXISTS `data_state_log` (
    `idpk` INT NOT NULL AUTO_INCREMENT,
    `id` VARCHAR(50) NOT NULL,
    `table_name` VARCHAR(100) NOT NULL,
    `old_status` VARCHAR(50) NOT NULL DEFAULT '',
    `new_status` VARCHAR(50) NOT NULL DEFAULT '',
    `reason` VARCHAR(500) NOT NULL DEFAULT '',
    `upby` VARCHAR(50) NOT NULL DEFAULT '',
    `uptime` DATETIME NOT NULL,
    PRIMARY KEY (`idpk`),
    INDEX `idx_table_name` (`table_name`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4
"#;

/// data_sync_stats 表 - 同步统计（MySQL 版本）
pub const DATA_SYNC_STATS_CREATE_SQL_MYSQL: &str = r#"
CREATE TABLE IF NOT EXISTS `data_sync_stats` (
    `idpk` INT NOT NULL AUTO_INCREMENT,
    `id` VARCHAR(50) NOT NULL,
    `table_name` VARCHAR(100) NOT NULL,
    `downloaded` INT NOT NULL DEFAULT 0,
    `updated` INT NOT NULL DEFAULT 0,
    `skipped` INT NOT NULL DEFAULT 0,
    `failed` INT NOT NULL DEFAULT 0,
    `stat_date` DATE NOT NULL,
    `upby` VARCHAR(50) NOT NULL DEFAULT '',
    `uptime` DATETIME NOT NULL,
    PRIMARY KEY (`idpk`),
    UNIQUE KEY `u_table_date` (`table_name`, `stat_date`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4
"#;

// ========== 数据结构（与 SQLite 版本相同） ==========

/// 同步日志项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynclogItemMysql {
    pub idpk: i64,
    pub apisys: String,
    pub apimicro: String,
    pub apiobj: String,
    pub tbname: String,
    pub action: String,
    pub cmdtext: String,
    pub params: String,
    pub idrow: String,
    pub worker: String,
    pub synced: i32,
    pub cmdtextmd5: String,
    pub num: i32,
    pub dlong: i64,
    pub downlen: i64,
    pub id: String,
    pub upby: String,
    pub uptime: String,
    pub cid: String,
}

/// 状态变更日志
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateLogMysql {
    pub idpk: i64,
    pub id: String,
    pub table_name: String,
    pub old_status: String,
    pub new_status: String,
    pub reason: String,
    pub upby: String,
    pub uptime: String,
}

/// 同步统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatsMysql {
    pub idpk: i64,
    pub id: String,
    pub table_name: String,
    pub downloaded: i32,
    pub updated: i32,
    pub skipped: i32,
    pub failed: i32,
    pub stat_date: String,
    pub upby: String,
    pub uptime: String,
}

/// 同步结果
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SyncResultMysql {
    pub res: i32,
    pub errmsg: String,
    pub datawf: SyncDataMysql,
}

/// 同步数据详情
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SyncDataMysql {
    pub inserted: i32,
    pub updated: i32,
    pub skipped: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<String>>,
}

// ========== DataSyncMysql 组件 ==========

/// DataSyncMysql - MySQL 版本同步队列组件
#[derive(Clone)]
pub struct DataSyncMysql {
    /// 表名
    pub table_name: String,

    /// MySQL 数据库实例
    pub db: Mysql78,

    /// API URL
    pub apiurl: String,

    /// 下载间隔(秒)
    pub download_interval: i64,
    /// 上传间隔(秒)
    pub upload_interval: i64,

    /// 下载条件
    pub download_condition: Option<Value>,
    /// 下载字段
    pub download_cols: Option<Vec<String>>,
    /// 上传字段顺序
    pub upload_cols: Option<Vec<String>>,

    /// 初始化下载数量
    pub init_getnumber: i32,
    /// 每次下载数量
    pub getnumber: i32,
    /// 最小待处理数量
    pub min_pending: i32,

    /// 隔离字段类型
    pub uidcid: String,

    /// 是否启用下载
    pub download_enabled: bool,
    /// 是否启用上传
    pub upload_enabled: bool,

    /// 上次下载时间
    pub last_download: f64,
    /// 上次上传时间
    pub last_upload: f64,

    /// 错误信息
    pub last_error: Option<String>,
}

impl std::fmt::Debug for DataSyncMysql {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DataSyncMysql")
            .field("table_name", &self.table_name)
            .field("apiurl", &self.apiurl)
            .field("download_interval", &self.download_interval)
            .field("upload_interval", &self.upload_interval)
            .field("download_enabled", &self.download_enabled)
            .field("upload_enabled", &self.upload_enabled)
            .field("last_download", &self.last_download)
            .field("last_upload", &self.last_upload)
            .field("last_error", &self.last_error)
            .finish()
    }
}

impl DataSyncMysql {
    /// 创建新实例
    pub fn new(table_name: &str, db: Mysql78) -> Self {
        Self {
            table_name: table_name.to_string(),
            db,
            apiurl: String::new(),
            download_interval: 300,
            upload_interval: 300,
            download_condition: None,
            download_cols: None,
            upload_cols: None,
            init_getnumber: 0,
            getnumber: 2000,
            min_pending: 0,
            uidcid: "cid".to_string(),
            download_enabled: true,
            upload_enabled: true,
            last_download: 0.0,
            last_upload: 0.0,
            last_error: None,
        }
    }

    /// 使用配置创建
    pub fn with_config(table_name: &str, mysql_config: MysqlConfig) -> Result<Self, String> {
        let mut db = Mysql78::new(mysql_config);
        db.initialize()?;
        Ok(Self::new(table_name, db))
    }

    /// 初始化同步队列相关表
    pub fn init_tables(&self) -> Result<(), String> {
        let up = MysqlUpInfo::new();
        self.db.do_get(SYNCLOG_CREATE_SQL_MYSQL, vec![], &up)?;
        self.db.do_get(DATA_STATE_LOG_CREATE_SQL_MYSQL, vec![], &up)?;
        self.db.do_get(DATA_SYNC_STATS_CREATE_SQL_MYSQL, vec![], &up)?;
        Ok(())
    }

    /// 当前时间戳
    pub fn current_time() -> f64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64()
    }

    // ========== CRUD 方法 ==========

    /// 插入记录
    pub fn m_add(&self, record: &HashMap<String, Value>) -> Result<String, String> {
        let id = crate::snowflake::next_id_string();
        let uptime = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

        let mut record_with_id = record.clone();
        record_with_id.insert("id".to_string(), Value::String(id.clone()));
        record_with_id.insert("uptime".to_string(), Value::String(uptime.clone()));

        let (cmdtext, params) = self.build_insert_sql(&record_with_id);
        let up = MysqlUpInfo::new();

        self.db.do_m_add(&cmdtext, params, &up)?;
        Ok(id)
    }

    /// 更新记录
    pub fn m_update(&self, id: &str, record: &HashMap<String, Value>) -> Result<bool, String> {
        let uptime = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

        let mut record_with_time = record.clone();
        record_with_time.insert("uptime".to_string(), Value::String(uptime));

        let (cmdtext, params) = self.build_update_sql(id, &record_with_time);
        let up = MysqlUpInfo::new();

        let result = self.db.do_m(&cmdtext, params, &up)?;
        Ok(result.affected_rows > 0)
    }

    /// 保存记录（存在更新，不存在插入）
    pub fn m_save(&self, record: &HashMap<String, Value>) -> Result<String, String> {
        let id = record.get("id")
            .and_then(|v: &serde_json::Value| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| crate::snowflake::next_id_string());

        let existing = self.get_one(&id)?;
        if existing.is_some() {
            self.m_update(&id, record)?;
        } else {
            self.m_add(record)?;
        }
        Ok(id)
    }

    /// 删除记录
    pub fn m_del(&self, id: &str) -> Result<bool, String> {
        let cmdtext = format!("DELETE FROM `{}` WHERE `id` = ?", self.table_name);
        let up = MysqlUpInfo::new();
        let result = self.db.do_m(&cmdtext, vec![Value::String(id.to_string())], &up)?;
        Ok(result.affected_rows > 0)
    }

    /// 同步保存记录（不写 synclog）
    pub fn m_sync_save(&self, record: &HashMap<String, Value>) -> Result<String, String> {
        self.m_save(record)
    }

    /// 同步更新记录（不写 synclog）
    pub fn m_sync_update(&self, id: &str, record: &HashMap<String, Value>) -> Result<bool, String> {
        self.m_update(id, record)
    }

    /// 同步删除记录（不写 synclog）
    pub fn m_sync_del(&self, id: &str) -> Result<bool, String> {
        self.m_del(id)
    }

    /// 查询记录
    pub fn get(&self, where_clause: &str, params: Vec<Value>) -> Result<Vec<HashMap<String, Value>>, String> {
        let sql = format!("SELECT * FROM `{}` WHERE {}", self.table_name, where_clause);
        self.do_get(&sql, params)
    }

    /// 查询单条记录
    pub fn get_one(&self, id: &str) -> Result<Option<HashMap<String, Value>>, String> {
        let sql = format!("SELECT * FROM `{}` WHERE `id` = ?", self.table_name);
        let up = MysqlUpInfo::new();
        let results = self.db.do_get(&sql, vec![Value::String(id.to_string())], &up)?;
        Ok(results.into_iter().next())
    }

    /// 统计记录数
    pub fn count(&self) -> Result<i32, String> {
        let sql = format!("SELECT COUNT(*) as cnt FROM `{}`", self.table_name);
        let up = MysqlUpInfo::new();
        let results = self.db.do_get(&sql, vec![], &up)?;
        Ok(results.first()
            .and_then(|r| r.get("cnt"))
            .and_then(|v: &serde_json::Value| v.as_i64())
            .map(|n| n as i32)
            .unwrap_or(0))
    }

    /// 执行任意 SQL 查询
    pub fn do_get(&self, sql: &str, params: Vec<Value>) -> Result<Vec<HashMap<String, Value>>, String> {
        let up = MysqlUpInfo::new();
        self.db.do_get(sql, params, &up)
    }

    /// 执行任意 SQL 更新
    pub fn do_m(&self, sql: &str, params: Vec<Value>) -> Result<usize, String> {
        let up = MysqlUpInfo::new();
        let result = self.db.do_m(sql, params, &up)?;
        Ok(result.affected_rows as usize)
    }

    // ========== SQL 构建 ==========

    fn build_insert_sql(&self, record: &HashMap<String, Value>) -> (String, Vec<Value>) {
        let mut columns: Vec<&str> = record.keys().map(|s| s.as_str()).collect();
        columns.sort();

        let col_names: Vec<String> = columns.iter().map(|c| format!("`{}`", c)).collect();
        let placeholders: Vec<&str> = columns.iter().map(|_| "?").collect();

        let cmdtext = format!(
            "INSERT INTO `{}` ({}) VALUES ({})",
            self.table_name,
            col_names.join(", "),
            placeholders.join(", ")
        );

        let params: Vec<Value> = columns.iter()
            .filter_map(|c| record.get(*c).cloned())
            .collect();

        (cmdtext, params)
    }

    fn build_update_sql(&self, id: &str, record: &HashMap<String, Value>) -> (String, Vec<Value>) {
        let mut columns: Vec<&str> = record.keys().map(|s| s.as_str()).collect();
        columns.retain(|c| *c != "id");
        columns.sort();

        let set_clause = columns.iter()
            .map(|c| format!("`{}` = ?", c))
            .collect::<Vec<_>>()
            .join(", ");

        let cmdtext = format!(
            "UPDATE `{}` SET {} WHERE `id` = ?",
            self.table_name, set_clause
        );

        let mut params: Vec<Value> = columns.iter()
            .filter_map(|c| record.get(*c).cloned())
            .collect();
        params.push(Value::String(id.to_string()));

        (cmdtext, params)
    }

    // ========== 同步队列操作 ==========

    /// 获取待同步的记录数
    pub fn get_pending_count(&self) -> i32 {
        let sql = "SELECT COUNT(*) as cnt FROM synclog WHERE tbname = ? AND synced = 0";
        let up = MysqlUpInfo::new();
        match self.db.do_get(sql, vec![Value::String(self.table_name.clone())], &up) {
            Ok(results) => results.first()
                .and_then(|r| r.get("cnt"))
                .and_then(|v: &serde_json::Value| v.as_i64())
                .map(|n| n as i32)
                .unwrap_or(0),
            _ => 0,
        }
    }

    /// 获取本地表的记录数
    pub fn get_local_count(&self) -> i32 {
        let sql = format!("SELECT COUNT(*) as cnt FROM `{}`", self.table_name);
        let up = MysqlUpInfo::new();
        match self.db.do_get(&sql, vec![], &up) {
            Ok(results) => results.first()
                .and_then(|r| r.get("cnt"))
                .and_then(|v: &serde_json::Value| v.as_i64())
                .map(|n| n as i32)
                .unwrap_or(0),
            _ => 0,
        }
    }

    /// 标记已同步
    pub fn mark_synced(&self, idpk_list: &[i64]) -> Result<(), String> {
        if idpk_list.is_empty() {
            return Ok(());
        }

        let placeholders: Vec<String> = idpk_list.iter().map(|_| "?".to_string()).collect();
        let sql = format!(
            "UPDATE synclog SET synced = 1 WHERE idpk IN ({})",
            placeholders.join(", ")
        );

        let params: Vec<Value> = idpk_list.iter()
            .map(|id| Value::Number((*id).into()))
            .collect();

        let up = MysqlUpInfo::new();
        self.db.do_m(&sql, params, &up)?;
        Ok(())
    }

    // ========== 状态变更日志 ==========

    /// 记录状态变更日志
    pub fn log_status_change(
        &self,
        old_status: &str,
        new_status: &str,
        reason: &str,
        worker: &str,
    ) -> Result<(), String> {
        let id = crate::snowflake::next_id_string();
        let uptime = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

        let sql = "INSERT INTO data_state_log (id, table_name, old_status, new_status, reason, upby, uptime) VALUES (?, ?, ?, ?, ?, ?, ?)";
        let params = vec![
            Value::String(id),
            Value::String(self.table_name.clone()),
            Value::String(old_status.to_string()),
            Value::String(new_status.to_string()),
            Value::String(reason.to_string()),
            Value::String(worker.to_string()),
            Value::String(uptime),
        ];

        let up = MysqlUpInfo::new();
        self.db.do_m(sql, params, &up)?;
        Ok(())
    }

    // ========== 同步统计 ==========

    /// 更新同步统计
    pub fn update_sync_stats(
        &self,
        downloaded: i32,
        updated: i32,
        skipped: i32,
        failed: i32,
        worker: &str,
    ) -> Result<(), String> {
        let id = crate::snowflake::next_id_string();
        let today = Local::now().format("%Y-%m-%d").to_string();
        let uptime = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

        // MySQL 使用 ON DUPLICATE KEY UPDATE
        let sql = r#"
            INSERT INTO data_sync_stats (id, table_name, downloaded, updated, skipped, failed, stat_date, upby, uptime)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON DUPLICATE KEY UPDATE
                downloaded = downloaded + VALUES(downloaded),
                updated = updated + VALUES(updated),
                skipped = skipped + VALUES(skipped),
                failed = failed + VALUES(failed),
                upby = VALUES(upby),
                uptime = VALUES(uptime)
        "#;

        let params = vec![
            Value::String(id),
            Value::String(self.table_name.clone()),
            Value::Number(downloaded.into()),
            Value::Number(updated.into()),
            Value::Number(skipped.into()),
            Value::Number(failed.into()),
            Value::String(today),
            Value::String(worker.to_string()),
            Value::String(uptime),
        ];

        let up = MysqlUpInfo::new();
        self.db.do_m(sql, params, &up)?;
        Ok(())
    }
}

impl Default for DataSyncMysql {
    fn default() -> Self {
        Self::new("", Mysql78::default())
    }
}
