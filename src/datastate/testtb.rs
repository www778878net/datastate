//! TestTb - 测试表 DataState
//!
//! 用于演示权限控制的使用方式
//!
//! 调用方及权限：
//! - testtb: 同表调用，全部权限
//! - inventory: 库存服务，可读可写
//! - trade: 交易服务，只读查询

use crate::dataaudit::DataAudit;
use crate::datamanage::DataManage;
use crate::datastate::DataState;
use crate::data_sync::DataSync;
use crate::localdb::LocalDB;
use crate::sync_config::{get_system_columns, TableConfig};
use serde::{Deserialize, Serialize};

/// testtb 表建表 SQL
pub const TESTTB_CREATE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS testtb (
    id TEXT NOT NULL PRIMARY KEY,
    idpk INTEGER,
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
        
        // 检查id是否是主键，如果不是则替换
        if let Ok(false) = db.is_id_primary_key("testtb") {
            let _ = db.ensure_id_is_primary_key("testtb");
        }

        // 直接创建 DataState 实例，使用默认数据库
        let state = DataState::with_db("testtb", db.clone());

        Self { db, audit, state }
    }

    pub fn with_db_path(db_path: &str) -> Self {
        // 设置环境变量来指定数据库路径
        std::env::set_var("SQLITE_PATH", db_path);
        
        let db = LocalDB::new(None).expect("创建数据库失败");
        let audit = DataAudit::new("testtb");

        db.execute(TESTTB_CREATE_SQL).expect("建表失败");
        
        // 检查id是否是主键，如果不是则替换
        if let Ok(false) = db.is_id_primary_key("testtb") {
            let _ = db.ensure_id_is_primary_key("testtb");
        }

        // 直接创建 DataState 实例，使用指定的数据库路径
        let state = DataState::with_db("testtb", db.clone());

        Self { db, audit, state }
    }

    fn check_caller(&self, ability: &str, caller: &str) -> Result<(), String> {
        match caller {
            "testtb" => Ok(()),
            "inventory" if matches!(ability, "getone" | "mlist" | "m_save") => Ok(()),
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

    pub fn m_add(&self, record: &std::collections::HashMap<String, serde_json::Value>, caller: &str, summary: &str) -> Result<String, String> {
        self.check_caller("m_add", caller)?;
        self.state.m_add(record, caller, summary)
    }

    pub fn m_save(&self, record: &std::collections::HashMap<String, serde_json::Value>, caller: &str, summary: &str) -> Result<String, String> {
        self.check_caller("m_save", caller)?;
        self.state.m_save(record, caller, summary)
    }

    pub fn m_update(&self, id: &str, record: &std::collections::HashMap<String, serde_json::Value>, caller: &str, summary: &str) -> Result<bool, String> {
        self.check_caller("m_update", caller)?;
        self.state.m_update(id, record, caller, summary)
    }

    pub fn m_del(&self, id: &str, caller: &str, summary: &str) -> Result<bool, String> {
        self.check_caller("m_del", caller)?;
        self.state.m_del(id, caller, summary)
    }

    /// 同步保存记录（不自动填充字段，不写 sync_queue）
    /// 用于从服务器同步数据到本地
    pub fn m_sync_save(&self, record: &std::collections::HashMap<String, serde_json::Value>) -> Result<String, String> {
        self.state.m_sync_save(record)
    }

    /// 同步更新记录（不自动填充字段，不写 sync_queue）
    pub fn m_sync_update(&self, id: &str, record: &std::collections::HashMap<String, serde_json::Value>) -> Result<bool, String> {
        self.state.m_sync_update(id, record)
    }

    /// 同步删除记录（不写 sync_queue）
    pub fn m_sync_del(&self, id: &str) -> Result<bool, String> {
        self.state.m_sync_del(id)
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

        let mut record = std::collections::HashMap::new();
        record.insert("cid".to_string(), serde_json::json!("c1"));
        record.insert("kind".to_string(), serde_json::json!("k1"));
        record.insert("item".to_string(), serde_json::json!("i1"));
        record.insert("data".to_string(), serde_json::json!("d1"));

        println!("【1】testtb 调用（全部允许）");
        let id = testtb.m_save(&record, "testtb", "内部保存").expect("testtb 应该能保存");
        println!("  - m_save 成功: {}", id);

        let _list = testtb.mlist("testtb", 10, "内部列表").expect("testtb 应该能列表");
        println!("  - mlist 成功: {} 条", _list.len());

        let _found = testtb.getone(&id, "testtb", "内部查询").expect("testtb 应该能查询");
        println!("  - getone 成功: {:?}", _found.is_some());

        testtb.m_del(&id, "testtb", "内部删除").expect("testtb 应该能删除");
        println!("  - m_del 成功");

        println!("\n【2】inventory 调用（允许 m_save/mlist/getone）");
        let id2 = testtb.m_save(&record, "inventory", "库存保存").expect("inventory 应该能保存");
        println!("  - m_save 成功: {}", id2);

        let _list = testtb.mlist("inventory", 10, "库存列表").expect("inventory 应该能列表");
        println!("  - mlist 成功");

        let _found = testtb.getone(&id2, "inventory", "库存查询").expect("inventory 应该能查询");
        println!("  - getone 成功");

        let result = testtb.m_del(&id2, "inventory", "尝试删除");
        assert!(result.is_err(), "inventory 不应该能删除");
        println!("  - m_del 拒绝: {}", result.unwrap_err());

        println!("\n【3】trade 调用（只允许 mlist/getone）");
        let _list = testtb.mlist("trade", 10, "交易列表").expect("trade 应该能列表");
        println!("  - mlist 成功");

        let _found = testtb.getone(&id2, "trade", "交易查询").expect("trade 应该能查询");
        println!("  - getone 成功");

        let result = testtb.m_save(&record, "trade", "尝试保存");
        assert!(result.is_err(), "trade 不应该能保存");
        println!("  - m_save 拒绝: {}", result.unwrap_err());

        let result = testtb.m_del(&id2, "trade", "尝试删除");
        assert!(result.is_err(), "trade 不应该能删除");
        println!("  - m_del 拒绝: {}", result.unwrap_err());

        println!("\n【4】unknown 调用（全部拒绝）");
        let result = testtb.mlist("unknown", 10, "未知调用");
        assert!(result.is_err());
        println!("  - mlist 拒绝: {}", result.unwrap_err());

        println!("\n=== 测试完成 ===");
    }
}
