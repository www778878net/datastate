//! SysSqlMysqlState - MySQL 版 SQL 统计状态管理
//!
//! 管理 sys_sql 表的 MySQL 操作

use crate::mysql78::{Mysql78, MysqlUpInfo};
use crate::shared::SysSqlData;

/// sys_sql 表名
pub const TABLE_NAME: &str = "sys_sql";

/// MySQL 建表 SQL
pub const CREATE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS `sys_sql` (
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
) ENGINE=InnoDB AUTO_INCREMENT=0 DEFAULT CHARSET=utf8
"#;

/// SysSqlMysqlState - MySQL 版本
pub struct SysSqlMysqlState {
    /// 数据库连接
    db: Mysql78,
}

impl SysSqlMysqlState {
    /// 创建新实例
    pub fn new(db: Mysql78) -> Self {
        Self { db }
    }

    /// 创建表
    pub fn create_table(&self, up: &MysqlUpInfo) -> Result<String, String> {
        self.db.do_get(CREATE_SQL, vec![], up)?;
        Ok("ok".to_string())
    }

    /// 记录 SQL 执行统计
    pub fn log_sql(&self, data: &SysSqlData, up: &MysqlUpInfo) -> Result<(), String> {
        // MySQL 使用 ON DUPLICATE KEY UPDATE
        let sql = "INSERT INTO sys_sql(id,cid,apisys,apimicro,apiobj,cmdtext,uname,num,dlong,downlen,upby,cmdtextmd5,uptime) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?) ON DUPLICATE KEY UPDATE num=num+1,dlong=dlong+?,downlen=downlen+?";

        let params = vec![
            serde_json::json!(data.id),
            serde_json::json!(data.cid),
            serde_json::json!(data.apisys),
            serde_json::json!(data.apimicro),
            serde_json::json!(data.apiobj),
            serde_json::json!(data.cmdtext),
            serde_json::json!(data.uname),
            serde_json::json!(1),
            serde_json::json!(data.dlong),
            serde_json::json!(data.downlen),
            serde_json::json!(data.upby),
            serde_json::json!(data.cmdtextmd5),
            serde_json::json!(data.uptime),
            serde_json::json!(data.dlong),
            serde_json::json!(data.downlen),
        ];

        self.db.do_m(sql, params, up)?;
        Ok(())
    }

    /// 获取慢 SQL 列表
    pub fn get_slow_sql(&self, min_dlong: i64, limit: i32, up: &MysqlUpInfo) -> Result<Vec<SysSqlData>, String> {
        let sql = "SELECT id,cid,apisys,apimicro,apiobj,cmdtext,uname,num,dlong,downlen,upby,cmdtextmd5,uptime FROM sys_sql WHERE dlong > ? ORDER BY dlong DESC LIMIT ?";

        let rows = self.db.do_get(sql, vec![serde_json::json!(min_dlong), serde_json::json!(limit)], up)?;

        Ok(rows.iter().map(|row| SysSqlData {
            id: row.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            cid: row.get("cid").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            apisys: row.get("apisys").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            apimicro: row.get("apimicro").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            apiobj: row.get("apiobj").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            cmdtext: row.get("cmdtext").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            uname: row.get("uname").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            num: row.get("num").and_then(|v| v.as_i64()).unwrap_or(0),
            dlong: row.get("dlong").and_then(|v| v.as_i64()).unwrap_or(0),
            downlen: row.get("downlen").and_then(|v| v.as_i64()).unwrap_or(0),
            upby: row.get("upby").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            cmdtextmd5: row.get("cmdtextmd5").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            uptime: row.get("uptime").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            ..Default::default()
        }).collect())
    }

    /// 获取高频 SQL 列表
    pub fn get_hot_sql(&self, min_num: i64, limit: i32, up: &MysqlUpInfo) -> Result<Vec<SysSqlData>, String> {
        let sql = "SELECT id,cid,apisys,apimicro,apiobj,cmdtext,uname,num,dlong,downlen,upby,cmdtextmd5,uptime FROM sys_sql WHERE num > ? ORDER BY num DESC LIMIT ?";

        let rows = self.db.do_get(sql, vec![serde_json::json!(min_num), serde_json::json!(limit)], up)?;

        Ok(rows.iter().map(|row| SysSqlData {
            id: row.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            cid: row.get("cid").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            apisys: row.get("apisys").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            apimicro: row.get("apimicro").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            apiobj: row.get("apiobj").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            cmdtext: row.get("cmdtext").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            uname: row.get("uname").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            num: row.get("num").and_then(|v| v.as_i64()).unwrap_or(0),
            dlong: row.get("dlong").and_then(|v| v.as_i64()).unwrap_or(0),
            downlen: row.get("downlen").and_then(|v| v.as_i64()).unwrap_or(0),
            upby: row.get("upby").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            cmdtextmd5: row.get("cmdtextmd5").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            uptime: row.get("uptime").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            ..Default::default()
        }).collect())
    }
}

impl Default for SysSqlMysqlState {
    fn default() -> Self {
        Self::new(Mysql78::new(Default::default()))
    }
}