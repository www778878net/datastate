//! TestTb - 测试表 DataState
//!
//! 用于演示权限控制的使用方式
//!
//! 调用方及权限：
//! - testtb: 同表调用，全部权限
//! - inventory: 库存服务，可读可写
//! - trade: 交易服务，只读查询

use crate::dataaudit::DataAudit;
use crate::{get_system_columns, DataManage, DataState, LocalDB, TableConfig};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// testtb 表建表 SQL
pub const TESTTB_CREATE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS testtb (
    idpk INTEGER PRIMARY KEY AUTOINCREMENT,
    id TEXT NOT NULL,
    cid TEXT NOT NULL DEFAULT '',
    kind TEXT NOT NULL DEFAULT '',
    item TEXT NOT NULL DEFAULT '',
    data TEXT NOT NULL DEFAULT '',
    upby TEXT NOT NULL DEFAULT '',
    uptime TEXT NOT NULL DEFAULT '',
    remark TEXT NOT NULL DEFAULT '',
    remark2 TEXT NOT NULL DEFAULT '',
    remark3 TEXT NOT NULL DEFAULT '',
    remark4 TEXT NOT NULL DEFAULT '',
    remark5 TEXT NOT NULL DEFAULT '',
    remark6 TEXT NOT NULL DEFAULT '',
    UNIQUE(id)
)
"#;

/// TestTb 记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestTbRecord {
    pub idpk: i64,
    pub id: String,
    pub cid: String,
    pub kind: String,
    pub item: String,
    pub data: String,
    pub upby: String,
    pub uptime: String,
}

/// TestTb - 测试表 DataState
pub struct TestTb {
    pub db: LocalDB,
    pub audit: DataAudit,
    pub state: DataState,
}

impl TestTb {
    pub fn get_config() -> TableConfig {
        let mut columns = get_system_columns();
        columns.insert("cid".to_string(), "TEXT NOT NULL DEFAULT ''".to_string());
        columns.insert("kind".to_string(), "TEXT NOT NULL DEFAULT ''".to_string());
        columns.insert("item".to_string(), "TEXT NOT NULL DEFAULT ''".to_string());
        columns.insert("data".to_string(), "TEXT NOT NULL DEFAULT ''".to_string());

        TableConfig {
            name: "testtb".to_string(),
            apiurl: "http://api.example.com/testtb".to_string(),
            download_interval: 300,
            upload_interval: 300,
            init_getnumber: 0,
            getnumber: 2000,
            columns,
            ..Default::default()
        }
    }

    pub fn new() -> Self {
        let db = LocalDB::default_instance().expect("获取数据库失败");
        let audit = DataAudit::new("testtb");

        db.execute(TESTTB_CREATE_SQL).expect("建表失败");

        let config = Self::get_config();
        let dm = DataManage::get_singleton();
        let state = dm.register(config).expect("注册到 DataManage 失败");

        Self { db, audit, state }
    }

    fn check_caller(&self, ability: &str, caller: &str) -> Result<(), String> {
        match caller {
            "testtb" => Ok(()),
            "inventory" if matches!(ability, "getone" | "mlist" | "msave") => Ok(()),
            "trade" if matches!(ability, "getone" | "mlist") => Ok(()),
            _ => Err(format!("[{}] 无权调用 testtb/{}", caller, ability)),
        }
    }

    pub fn getone(
        &self,
        id: &str,
        caller: &str,
        summary: &str,
    ) -> Result<Option<TestTbRecord>, String> {
        self.check_caller("getone", caller)?;
        self.audit.check_permission("getone", caller, summary)?;
        
        let sql = "SELECT * FROM testtb WHERE id = ?";
        match self.db.query(sql, &[&id as &dyn rusqlite::ToSql]) {
            Ok(rows) if !rows.is_empty() => {
                let row = &rows[0];
                Ok(Some(TestTbRecord {
                    idpk: row.get("idpk").and_then(|v| v.as_i64()).unwrap_or(0),
                    id: row.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    cid: row.get("cid").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    kind: row.get("kind").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    item: row.get("item").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    data: row.get("data").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    upby: row.get("upby").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    uptime: row.get("uptime").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                }))
            }
            _ => Ok(None),
        }
    }

    pub fn mlist(
        &self,
        caller: &str,
        limit: i32,
        summary: &str,
    ) -> Result<Vec<TestTbRecord>, String> {
        self.check_caller("mlist", caller)?;
        self.audit.check_permission("mlist", caller, summary)?;
        
        let sql = format!("SELECT * FROM testtb ORDER BY idpk DESC LIMIT {}", limit);
        match self.db.query(&sql, &[]) {
            Ok(rows) => {
                let result: Vec<TestTbRecord> = rows
                    .iter()
                    .map(|row| TestTbRecord {
                        idpk: row.get("idpk").and_then(|v| v.as_i64()).unwrap_or(0),
                        id: row.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        cid: row.get("cid").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        kind: row.get("kind").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        item: row.get("item").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        data: row.get("data").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        upby: row.get("upby").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        uptime: row.get("uptime").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    })
                    .collect();
                Ok(result)
            }
            _ => Ok(Vec::new()),
        }
    }

    pub fn msave(
        &self,
        record: &TestTbRecord,
        caller: &str,
        summary: &str,
    ) -> Result<String, String> {
        self.check_caller("msave", caller)?;
        self.audit.check_permission("msave", caller, summary)?;
        
        let id = Uuid::new_v4().to_string();
        let sql = "INSERT INTO testtb (id, cid, kind, item, data, upby, uptime) VALUES (?, ?, ?, ?, ?, ?, datetime('now'))";
        self.db.execute_with_params(
            sql,
            &[&id, &record.cid, &record.kind, &record.item, &record.data, &caller],
        )?;
        Ok(id)
    }

    pub fn mdelete(&self, id: &str, caller: &str, summary: &str) -> Result<bool, String> {
        self.check_caller("mdelete", caller)?;
        self.audit.check_permission("mdelete", caller, summary)?;
        
        let sql = "DELETE FROM testtb WHERE id = ?";
        self.db.execute_with_params(sql, &[&id])?;
        Ok(true)
    }
}

impl Default for TestTb {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_permission() {
        println!("\n=== 权限测试 ===\n");

        let testtb = TestTb::new();

        let record = TestTbRecord {
            idpk: 0,
            id: String::new(),
            cid: "c1".into(),
            kind: "k1".into(),
            item: "i1".into(),
            data: "d1".into(),
            upby: String::new(),
            uptime: String::new(),
        };

        println!("【1】testtb 调用（全部允许）");
        let id = testtb.msave(&record, "testtb", "内部保存").expect("testtb 应该能保存");
        println!("  - msave 成功: {}", id);

        let _list = testtb.mlist("testtb", 10, "内部列表").expect("testtb 应该能列表");
        println!("  - mlist 成功: {} 条", _list.len());

        let _found = testtb.getone(&id, "testtb", "内部查询").expect("testtb 应该能查询");
        println!("  - getone 成功: {:?}", _found.is_some());

        testtb.mdelete(&id, "testtb", "内部删除").expect("testtb 应该能删除");
        println!("  - mdelete 成功");

        println!("\n【2】inventory 调用（允许 msave/mlist/getone）");
        let id2 = testtb.msave(&record, "inventory", "库存保存").expect("inventory 应该能保存");
        println!("  - msave 成功: {}", id2);

        let _list = testtb.mlist("inventory", 10, "库存列表").expect("inventory 应该能列表");
        println!("  - mlist 成功");

        let _found = testtb.getone(&id2, "inventory", "库存查询").expect("inventory 应该能查询");
        println!("  - getone 成功");

        let result = testtb.mdelete(&id2, "inventory", "尝试删除");
        assert!(result.is_err(), "inventory 不应该能删除");
        println!("  - mdelete 拒绝: {}", result.unwrap_err());

        println!("\n【3】trade 调用（只允许 mlist/getone）");
        let _list = testtb.mlist("trade", 10, "交易列表").expect("trade 应该能列表");
        println!("  - mlist 成功");

        let _found = testtb.getone(&id2, "trade", "交易查询").expect("trade 应该能查询");
        println!("  - getone 成功");

        let result = testtb.msave(&record, "trade", "尝试保存");
        assert!(result.is_err(), "trade 不应该能保存");
        println!("  - msave 拒绝: {}", result.unwrap_err());

        let result = testtb.mdelete(&id2, "trade", "尝试删除");
        assert!(result.is_err(), "trade 不应该能删除");
        println!("  - mdelete 拒绝: {}", result.unwrap_err());

        println!("\n【4】unknown 调用（全部拒绝）");
        let result = testtb.mlist("unknown", 10, "未知调用");
        assert!(result.is_err());
        println!("  - mlist 拒绝: {}", result.unwrap_err());

        println!("\n=== 测试完成 ===");
    }
}
