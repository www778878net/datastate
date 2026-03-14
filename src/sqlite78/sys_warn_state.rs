//! SysWarnSqliteState - SQLite 版警告日志状态管理
//!
//! 管理 sys_warn 表的 SQLite 操作

use crate::sqlite78::Sqlite78;
use crate::shared::SysWarnData;
use base::UpInfo;

/// sys_warn 表名
pub const TABLE_NAME: &str = "sys_warn";

/// SQLite 建表 SQL
pub const CREATE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS sys_warn (
    uid TEXT NOT NULL DEFAULT '',
    kind TEXT NOT NULL DEFAULT '',
    apimicro TEXT NOT NULL DEFAULT '',
    apiobj TEXT NOT NULL DEFAULT '',
    content TEXT NOT NULL,
    upid TEXT NOT NULL DEFAULT '',
    upby TEXT DEFAULT '',
    uptime TEXT NOT NULL DEFAULT '',
    idpk INTEGER PRIMARY KEY AUTOINCREMENT,
    id TEXT NOT NULL,
    UNIQUE(id)
)
"#;

/// SysWarnSqliteState - SQLite 版本
pub struct SysWarnSqliteState {
    /// 数据库连接
    db: Sqlite78,
}

impl SysWarnSqliteState {
    /// 创建新实例
    pub fn new(db: Sqlite78) -> Self {
        Self { db }
    }

    /// 创建表
    pub fn create_table(&self, _up: &UpInfo) -> Result<String, String> {
        let conn = self.db.get_conn()?;
        let conn = conn.lock().map_err(|e| e.to_string())?;
        conn.execute(CREATE_SQL, [])
            .map_err(|e| format!("创建 sys_warn 表失败: {}", e))?;
        Ok("ok".to_string())
    }

    /// 插入警告记录
    pub fn insert(&self, data: &SysWarnData, up: &UpInfo) -> Result<i64, String> {
        let sql = "INSERT INTO sys_warn (id,uid,kind,apimicro,apiobj,content,upid,upby,uptime,remark,remark2,remark3,remark4,remark5,remark6) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)";

        let params: [&dyn rusqlite::ToSql; 15] = [
            &data.id,
            &data.uid,
            &data.kind,
            &data.apimicro,
            &data.apiobj,
            &data.content,
            &data.upid,
            &data.upby,
            &data.uptime,
            &data.remark,
            &data.remark2,
            &data.remark3,
            &data.remark4,
            &data.remark5,
            &data.remark6,
        ];

        let result = self.db.do_m_add(sql, &params, up)?;
        if let Some(err) = result.error {
            return Err(err);
        }
        Ok(result.insert_id)
    }

    /// 查询指定类型的警告
    pub fn get_by_kind(&self, kind: &str, up: &UpInfo) -> Result<Vec<SysWarnData>, String> {
        let sql = "SELECT id,uid,kind,apimicro,apiobj,content,upid,upby,uptime,remark,remark2,remark3,remark4,remark5,remark6 FROM sys_warn WHERE kind = ?";

        let rows = self.db.do_get(sql, &[&kind as &dyn rusqlite::ToSql], up)?;

        Ok(rows.iter().map(|row| SysWarnData {
            id: row.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            uid: row.get("uid").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            kind: row.get("kind").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            apimicro: row.get("apimicro").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            apiobj: row.get("apiobj").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            content: row.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            upid: row.get("upid").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            upby: row.get("upby").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            uptime: row.get("uptime").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            remark: row.get("remark").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            remark2: row.get("remark2").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            remark3: row.get("remark3").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            remark4: row.get("remark4").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            remark5: row.get("remark5").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            remark6: row.get("remark6").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        }).collect())
    }

    /// 删除旧记录（保留最近N条）
    pub fn clean_old(&self, keep_count: i32, up: &UpInfo) -> Result<i64, String> {
        let sql = "DELETE FROM sys_warn WHERE idpk NOT IN (SELECT idpk FROM sys_warn ORDER BY idpk DESC LIMIT ?)";

        let result = self.db.do_m(sql, &[&keep_count as &dyn rusqlite::ToSql], up)?;
        Ok(result.affected_rows)
    }
}

impl Default for SysWarnSqliteState {
    fn default() -> Self {
        Self::new(Sqlite78::new())
    }
}