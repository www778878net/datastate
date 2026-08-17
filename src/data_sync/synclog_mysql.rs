//! SynclogMysql - MySQL 版同步日志分表管理
//!
//! 与 SQLite 版 Synclog 对应的 MySQL 版本，供 axum78 datasync_mysql Controller 使用：
//! - 按天分表：synclog_YYYYMMDD
//! - 写操作：00:00 立即切换到今天的表
//! - 读操作：延迟切换策略（00:00-00:30 读昨天的表，00:30 之后读今天的表）
//!
//! 职责：
//! - ensure_tables: 确保当天 + 明天的分表存在
//! - insert: 写入当天的分表
//! - get: 查询已同步记录（synced=1）供下载
//! - get_pending: 查询待重放记录（synced=0）
//! - mark_synced_one: 更新单条同步状态
//! - get_snap_id: 获取增量同步快照点
//! - get_by_worker: 按 worker 增量拉取其他客户端的变更

use crate::mysql78::{Mysql78, MysqlUpInfo};
use chrono::{Duration, Local, Timelike};
use serde_json::Value;
use std::collections::HashMap;

/// 分表名前缀
const SYNCLOG_PREFIX: &str = "synclog_";

/// synclog 分表建表 SQL（MySQL 语法）
const SYNCLOG_SHARD_CREATE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS `{table}` (
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

/// 同步日志项（MySQL 版）
#[derive(Debug, Clone)]
pub struct SynclogMysqlItem {
    pub id: String,
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
    pub lasterrinfo: String,
    pub cmdtextmd5: String,
    pub cid: String,
    pub upby: String,
}

/// SynclogMysql - MySQL 版同步日志分表管理器
#[derive(Clone)]
pub struct SynclogMysql {
    mysql: Mysql78,
}

impl SynclogMysql {
    /// 创建实例
    pub fn new(mysql: Mysql78) -> Self {
        Self { mysql }
    }

    /// 获取写表名（始终今天的表）
    fn get_write_table_name(&self) -> String {
        format!("{}{}", SYNCLOG_PREFIX, Local::now().format("%Y%m%d"))
    }

    /// 获取读表名（延迟切换策略：00:00-00:30 读昨天，00:30 之后读今天）
    fn get_read_table_name(&self) -> String {
        let now = Local::now();
        if now.hour() == 0 && now.minute() < 30 {
            format!(
                "{}{}",
                SYNCLOG_PREFIX,
                (now - Duration::days(1)).format("%Y%m%d")
            )
        } else {
            format!("{}{}", SYNCLOG_PREFIX, now.format("%Y%m%d"))
        }
    }

    /// 确保当天 + 明天的分表都存在
    pub fn ensure_tables(&self) -> Result<(), String> {
        let up = MysqlUpInfo::new();
        let now = Local::now();
        let today = now.format("%Y%m%d").to_string();
        let tomorrow = (now + Duration::days(1)).format("%Y%m%d").to_string();

        for date in [&today, &tomorrow] {
            let table = format!("{}{}", SYNCLOG_PREFIX, date);
            let sql = SYNCLOG_SHARD_CREATE_SQL.replace("{table}", &table);
            let r = self.mysql.do_m(&sql, vec![], &up)?;
            if r.error.is_some() {
                return Err(r.error.unwrap());
            }
        }
        Ok(())
    }

    /// 插入同步记录（写入今天的表）
    pub fn insert(&self, item: &SynclogMysqlItem) -> Result<(), String> {
        let up = MysqlUpInfo::new();
        let sql = format!(
            "INSERT INTO `{}` (apisys, apimicro, apiobj, tbname, action, cmdtext, params, idrow, worker, synced, lasterrinfo, cmdtextmd5, num, dlong, downlen, id, upby, uptime, cid) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,NOW(),?)",
            self.get_write_table_name()
        );
        let params = vec![
            Value::String(item.apisys.clone()),
            Value::String(item.apimicro.clone()),
            Value::String(item.apiobj.clone()),
            Value::String(item.tbname.clone()),
            Value::String(item.action.clone()),
            Value::String(item.cmdtext.clone()),
            Value::String(item.params.clone()),
            Value::String(item.idrow.clone()),
            Value::String(item.worker.clone()),
            Value::from(item.synced),
            Value::String(item.lasterrinfo.clone()),
            Value::String(item.cmdtextmd5.clone()),
            Value::from(0),
            Value::from(0),
            Value::from(0),
            Value::String(item.id.clone()),
            Value::String(item.upby.clone()),
            Value::String(item.cid.clone()),
        ];
        let r = self.mysql.do_m(&sql, params, &up)?;
        if r.error.is_some() {
            return Err(r.error.unwrap());
        }
        Ok(())
    }

    /// 查询已同步记录（synced=1），用于下载同步
    ///
    /// cid: 帐套过滤；tbname: 表名过滤（空 = 不过滤）；order: 排序（空 = idpk DESC）；start/number: 分页
    pub fn get(
        &self,
        cid: &str,
        tbname: &str,
        order: &str,
        start: i64,
        number: i64,
    ) -> Result<Vec<HashMap<String, Value>>, String> {
        let up = MysqlUpInfo::new();
        let table = self.get_read_table_name();
        let mut sql = format!("SELECT * FROM `{}` WHERE synced = 1", table);
        let mut params: Vec<Value> = Vec::new();
        if !cid.is_empty() {
            sql.push_str(" AND cid = ?");
            params.push(Value::String(cid.to_string()));
        }
        if !tbname.is_empty() {
            sql.push_str(" AND tbname = ?");
            params.push(Value::String(tbname.to_string()));
        }
        let order_clause = if order.is_empty() { "idpk DESC" } else { order };
        sql.push_str(&format!(
            " ORDER BY {} LIMIT {}, {}",
            order_clause, start, number
        ));
        self.mysql.do_get(&sql, params, &up)
    }

    /// 查询待重放记录（synced=0 且 worker != 本地）
    pub fn get_pending(&self, worker: &str, limit: i32) -> Result<Vec<HashMap<String, Value>>, String> {
        let up = MysqlUpInfo::new();
        let sql = format!(
            "SELECT * FROM `{}` WHERE synced = 0 AND worker != ? ORDER BY idpk ASC LIMIT {}",
            self.get_read_table_name(),
            limit.max(0)
        );
        self.mysql.do_get(&sql, vec![Value::String(worker.to_string())], &up)
    }

    /// 标记单条记录同步状态
    pub fn mark_synced_one(&self, id: &str, synced: i32, lasterrinfo: &str) -> Result<(), String> {
        let up = MysqlUpInfo::new();
        let sql = format!(
            "UPDATE `{}` SET synced = ?, lasterrinfo = ? WHERE id = ?",
            self.get_read_table_name()
        );
        let params = vec![
            Value::from(synced),
            Value::String(lasterrinfo.to_string()),
            Value::String(id.to_string()),
        ];
        let r = self.mysql.do_m(&sql, params, &up)?;
        if r.error.is_some() {
            return Err(r.error.unwrap());
        }
        Ok(())
    }

    /// 获取快照点：当前水位线前的最大雪花 ID（增量同步起点）
    pub fn get_snap_id(&self, tbname: &str) -> Result<String, String> {
        let up = MysqlUpInfo::new();
        let sql = format!(
            "SELECT MAX(id) AS maxid FROM `{}` WHERE synced = 1 AND tbname = ?",
            self.get_read_table_name()
        );
        let rows = self.mysql.do_get(&sql, vec![Value::String(tbname.to_string())], &up)?;
        match rows.first().and_then(|r| r.get("maxid")) {
            Some(Value::String(s)) if !s.is_empty() => Ok(s.clone()),
            Some(v) => Ok(v.to_string()),
            None => Ok("0".to_string()),
        }
    }

    /// 增量同步：synced=1 且 worker != 本地 且 id > last_server_id（按 tbname 过滤）
    pub fn get_by_worker(
        &self,
        worker: &str,
        last_server_id: &str,
        tbname: &str,
        limit: i32,
        only_synced: bool,
    ) -> Result<Vec<HashMap<String, Value>>, String> {
        let up = MysqlUpInfo::new();
        let table = self.get_read_table_name();
        let mut sql = format!("SELECT * FROM `{}` WHERE worker != ?", table);
        let mut params: Vec<Value> = vec![Value::String(worker.to_string())];

        if only_synced {
            sql.push_str(" AND synced = 1");
        }
        // last_server_id 是雪花 ID（数字字符串），转数值比较
        sql.push_str(" AND CAST(id AS UNSIGNED) > ?");
        params.push(Value::String(last_server_id.to_string()));

        if !tbname.is_empty() {
            sql.push_str(" AND tbname = ?");
            params.push(Value::String(tbname.to_string()));
        }
        sql.push_str(&format!(" ORDER BY id ASC LIMIT {}", limit.max(0)));
        self.mysql.do_get(&sql, params, &up)
    }
}
