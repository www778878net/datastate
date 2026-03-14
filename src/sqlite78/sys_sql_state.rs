//! SysSqlSqliteState - SQLite 版 SQL 统计状态管理
//!
//! 管理 sys_sql 表的 SQLite 操作

use crate::sqlite78::Sqlite78;
use crate::shared::SysSqlData;
use base::UpInfo;

/// sys_sql 表名
pub const TABLE_NAME: &str = "sys_sql";

/// SQLite 建表 SQL
pub const CREATE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS sys_sql (
    cid TEXT NOT NULL DEFAULT '',
    apisys TEXT NOT NULL DEFAULT '',
    apimicro TEXT NOT NULL DEFAULT '',
    apiobj TEXT NOT NULL DEFAULT '',
    cmdtext TEXT NOT NULL,
    num INTEGER NOT NULL DEFAULT 0,
    dlong INTEGER NOT NULL DEFAULT 0,
    downlen INTEGER NOT NULL DEFAULT 1,
    upby TEXT NOT NULL DEFAULT '',
    cmdtextmd5 TEXT NOT NULL DEFAULT '',
    uptime TEXT NOT NULL DEFAULT '',
    idpk INTEGER PRIMARY KEY AUTOINCREMENT,
    id TEXT NOT NULL,
    UNIQUE(cmdtextmd5),
    UNIQUE(id)
)
"#;

/// SysSqlSqliteState - SQLite 版本
pub struct SysSqlSqliteState {
    /// 数据库连接
    db: Sqlite78,
}

impl SysSqlSqliteState {
    /// 创建新实例
    pub fn new(db: Sqlite78) -> Self {
        Self { db }
    }

    /// 创建表
    pub fn create_table(&self, _up: &UpInfo) -> Result<String, String> {
        let conn = self.db.get_conn()?;
        let conn = conn.lock().map_err(|e| e.to_string())?;
        conn.execute(CREATE_SQL, [])
            .map_err(|e| format!("创建 sys_sql 表失败: {}", e))?;
        Ok("ok".to_string())
    }

    /// 记录 SQL 执行统计
    pub fn log_sql(&self, data: &SysSqlData, up: &UpInfo) -> Result<(), String> {
        // 插入或忽略
        let insert_sql = "INSERT OR IGNORE INTO sys_sql(id,cid,apisys,apimicro,apiobj,cmdtext,uname,num,dlong,downlen,upby,cmdtextmd5,uptime) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?)";

        let params: [&dyn rusqlite::ToSql; 13] = [
            &data.id,
            &data.cid,
            &data.apisys,
            &data.apimicro,
            &data.apiobj,
            &data.cmdtext,
            &data.uname,
            &1i64,
            &data.dlong,
            &data.downlen,
            &data.upby,
            &data.cmdtextmd5,
            &data.uptime,
        ];

        self.db.do_m_add(insert_sql, &params, up)?;

        // 更新计数器
        let update_sql = "UPDATE sys_sql SET num=num+1,dlong=dlong+?,downlen=downlen+? WHERE cmdtextmd5=?";
        self.db.do_m(update_sql, rusqlite::params![data.dlong, data.downlen, data.cmdtextmd5], up)?;

        Ok(())
    }

    /// 获取慢 SQL 列表
    pub fn get_slow_sql(&self, min_dlong: i64, limit: i32, up: &UpInfo) -> Result<Vec<SysSqlData>, String> {
        let sql = "SELECT id,cid,apisys,apimicro,apiobj,cmdtext,uname,num,dlong,downlen,upby,cmdtextmd5,uptime FROM sys_sql WHERE dlong > ? ORDER BY dlong DESC LIMIT ?";

        let rows = self.db.do_get(sql, &[&min_dlong as &dyn rusqlite::ToSql, &limit as &dyn rusqlite::ToSql], up)?;

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
    pub fn get_hot_sql(&self, min_num: i64, limit: i32, up: &UpInfo) -> Result<Vec<SysSqlData>, String> {
        let sql = "SELECT id,cid,apisys,apimicro,apiobj,cmdtext,uname,num,dlong,downlen,upby,cmdtextmd5,uptime FROM sys_sql WHERE num > ? ORDER BY num DESC LIMIT ?";

        let rows = self.db.do_get(sql, &[&min_num as &dyn rusqlite::ToSql, &limit as &dyn rusqlite::ToSql], up)?;

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

impl Default for SysSqlSqliteState {
    fn default() -> Self {
        Self::new(Sqlite78::new())
    }
}