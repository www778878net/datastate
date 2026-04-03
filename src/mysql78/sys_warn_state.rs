//! SysWarnMysqlState - MySQL 版警告日志状态管理
//!
//! 管理 sys_warn 表的 MySQL 操作

use crate::mysql78::{Mysql78, MysqlUpInfo};
use crate::shared::SysWarnData;

/// sys_warn 表名
pub const TABLE_NAME: &str = "sys_warn";

/// MySQL 建表 SQL
pub const CREATE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS `sys_warn` (
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
) ENGINE=InnoDB AUTO_INCREMENT=0 DEFAULT CHARSET=utf8
"#;

/// SysWarnMysqlState - MySQL 版本
pub struct SysWarnMysqlState {
    /// 数据库连接
    db: Mysql78,
}

impl SysWarnMysqlState {
    /// 创建新实例
    pub fn new(db: Mysql78) -> Self {
        Self { db }
    }

    /// 创建表
    pub fn create_table(&self, up: &MysqlUpInfo) -> Result<String, String> {
        self.db.do_get(CREATE_SQL, vec![], up)?;
        Ok("ok".to_string())
    }

    /// 插入警告记录
    pub fn insert(&self, data: &SysWarnData, up: &MysqlUpInfo) -> Result<i64, String> {
        let sql = "INSERT INTO sys_warn (id,uid,kind,apimicro,apiobj,content,upid,upby,uptime,remark,remark2,remark3,remark4,remark5,remark6) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)";

        let params = vec![
            serde_json::json!(data.id),
            serde_json::json!(data.uid),
            serde_json::json!(data.kind),
            serde_json::json!(data.apimicro),
            serde_json::json!(data.apiobj),
            serde_json::json!(data.content),
            serde_json::json!(data.upid),
            serde_json::json!(data.upby),
            serde_json::json!(data.uptime),
            serde_json::json!(data.remark),
            serde_json::json!(data.remark2),
            serde_json::json!(data.remark3),
            serde_json::json!(data.remark4),
            serde_json::json!(data.remark5),
            serde_json::json!(data.remark6),
        ];

        let result = self.db.do_m_add(sql, params, up)?;
        if let Some(err) = result.error {
            return Err(err);
        }
        Ok(result.insert_id)
    }

    /// 查询指定类型的警告
    pub fn get_by_kind(&self, kind: &str, up: &MysqlUpInfo) -> Result<Vec<SysWarnData>, String> {
        let sql = "SELECT id,uid,kind,apimicro,apiobj,content,upid,upby,uptime,remark,remark2,remark3,remark4,remark5,remark6 FROM sys_warn WHERE kind = ?";

        let rows = self.db.do_get(sql, vec![serde_json::json!(kind)], up)?;

        Ok(rows.iter().map(|row| SysWarnData {
            id: row.get("id").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("").to_string(),
            uid: row.get("uid").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("").to_string(),
            kind: row.get("kind").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("").to_string(),
            apimicro: row.get("apimicro").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("").to_string(),
            apiobj: row.get("apiobj").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("").to_string(),
            content: row.get("content").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("").to_string(),
            upid: row.get("upid").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("").to_string(),
            upby: row.get("upby").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("").to_string(),
            uptime: row.get("uptime").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("").to_string(),
            remark: row.get("remark").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("").to_string(),
            remark2: row.get("remark2").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("").to_string(),
            remark3: row.get("remark3").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("").to_string(),
            remark4: row.get("remark4").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("").to_string(),
            remark5: row.get("remark5").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("").to_string(),
            remark6: row.get("remark6").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("").to_string(),
        }).collect())
    }

    /// 删除旧记录（保留最近N条）
    pub fn clean_old(&self, keep_count: i32, up: &MysqlUpInfo) -> Result<i64, String> {
        let sql = "DELETE FROM sys_warn WHERE idpk NOT IN (SELECT idpk FROM (SELECT idpk FROM sys_warn ORDER BY idpk DESC LIMIT ?) AS tmp)";

        let result = self.db.do_m(sql, vec![serde_json::json!(keep_count)], up)?;
        Ok(result.affected_rows)
    }
}

impl Default for SysWarnMysqlState {
    fn default() -> Self {
        Self::new(Mysql78::new(Default::default()))
    }
}