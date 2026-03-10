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

    pub fn getone(
        &self,
        id: &str,
        caller: &str,
        summary: &str,
    ) -> Result<Option<TestTbRecord>, String> {
        let id_owned = id.to_string();
        self.audit.do_action_with_count(
            &self.db,
            "getone",
            caller,
            &format!("{} | id={}", summary, id),
            || {
                let sql = "SELECT * FROM testtb WHERE id = ?";
                match self.db.query(sql, &[&id_owned as &dyn rusqlite::ToSql]) {
                    Ok(rows) if !rows.is_empty() => {
                        let row = &rows[0];
                        Ok(Some(TestTbRecord {
                            idpk: row.get("idpk").and_then(|v| v.as_i64()).unwrap_or(0),
                            id: row
                                .get("id")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                            cid: row
                                .get("cid")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                            kind: row
                                .get("kind")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                            item: row
                                .get("item")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                            data: row
                                .get("data")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                            upby: row
                                .get("upby")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                            uptime: row
                                .get("uptime")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                        }))
                    }
                    _ => Ok(None),
                }
            },
        )
    }

    pub fn mlist(
        &self,
        caller: &str,
        limit: i32,
        summary: &str,
    ) -> Result<Vec<TestTbRecord>, String> {
        self.audit
            .do_action_with_count(&self.db, "mlist", caller, summary, || {
                let sql = format!("SELECT * FROM testtb ORDER BY idpk DESC LIMIT {}", limit);
                match self.db.query(&sql, &[]) {
                    Ok(rows) => {
                        let result: Vec<TestTbRecord> = rows
                            .iter()
                            .map(|row| TestTbRecord {
                                idpk: row.get("idpk").and_then(|v| v.as_i64()).unwrap_or(0),
                                id: row
                                    .get("id")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                cid: row
                                    .get("cid")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                kind: row
                                    .get("kind")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                item: row
                                    .get("item")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                data: row
                                    .get("data")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                upby: row
                                    .get("upby")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                uptime: row
                                    .get("uptime")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                            })
                            .collect();
                        Ok(result)
                    }
                    _ => Ok(Vec::new()),
                }
            })
    }

    pub fn msave(
        &self,
        record: &TestTbRecord,
        caller: &str,
        summary: &str,
    ) -> Result<String, String> {
        let id = Uuid::new_v4().to_string();
        let id_clone = id.clone();
        let record_cid = record.cid.clone();
        let record_kind = record.kind.clone();
        let record_item = record.item.clone();
        let record_data = record.data.clone();
        let caller_owned = caller.to_string();

        self.audit.do_action_with_count(
            &self.db,
            "msave",
            caller,
            &format!("{} | id={}, kind={}", summary, id, record.kind),
            || {
                let sql = "INSERT INTO testtb (id, cid, kind, item, data, upby, uptime) VALUES (?, ?, ?, ?, ?, ?, datetime('now'))";
                self.db.execute_with_params(
                    sql,
                    &[&id_clone, &record_cid, &record_kind, &record_item, &record_data, &caller_owned],
                )?;
                Ok(id)
            },
        )
    }

    pub fn mdelete(&self, id: &str, caller: &str, summary: &str) -> Result<bool, String> {
        let id_owned = id.to_string();
        self.audit.do_action_with_count(
            &self.db,
            "mdelete",
            caller,
            &format!("{} | id={}", summary, id),
            || {
                let sql = "DELETE FROM testtb WHERE id = ?";
                self.db.execute_with_params(sql, &[&id_owned])?;
                Ok(true)
            },
        )
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
    use crate::register_ability_simple;

    fn setup_permissions(testtb: &TestTb) {
        let _ = register_ability_simple(&testtb.db, "testtb", "*", "testtb", "同表调用全部权限");
        let _ =
            register_ability_simple(&testtb.db, "testtb", "getone", "inventory", "库存服务查询");
        let _ = register_ability_simple(&testtb.db, "testtb", "mlist", "inventory", "库存服务列表");
        let _ = register_ability_simple(&testtb.db, "testtb", "msave", "inventory", "库存服务保存");
        let _ = register_ability_simple(&testtb.db, "testtb", "getone", "trade", "交易服务查询");
        let _ = register_ability_simple(&testtb.db, "testtb", "mlist", "trade", "交易服务列表");
    }

    #[test]
    fn test_audit_permission() {
        println!("\n=== 权限测试（使用 audit 组件）===\n");

        let testtb = TestTb::new();
        setup_permissions(&testtb);

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
        let id = testtb
            .msave(&record, "testtb", "内部保存测试数据")
            .expect("testtb 应该能保存");
        println!("  - msave 成功: {}", id);

        let _list = testtb
            .mlist("testtb", 10, "内部列表查询")
            .expect("testtb 应该能列表");
        println!("  - mlist 成功: {} 条", _list.len());

        let _found = testtb
            .getone(&id, "testtb", "内部查询单条")
            .expect("testtb 应该能查询");
        println!("  - getone 成功: {:?}", _found.is_some());

        testtb
            .mdelete(&id, "testtb", "内部删除测试数据")
            .expect("testtb 应该能删除");
        println!("  - mdelete 成功");

        println!("\n【2】inventory 调用（允许 msave/mlist/getone）");
        let id2 = testtb
            .msave(&record, "inventory", "库存服务保存测试数据")
            .expect("inventory 应该能保存");
        println!("  - msave 成功: {}", id2);

        let _list = testtb
            .mlist("inventory", 10, "库存服务查询")
            .expect("inventory 应该能列表");
        println!("  - mlist 成功");

        let _found = testtb
            .getone(&id2, "inventory", "库存服务查询单条")
            .expect("inventory 应该能查询");
        println!("  - getone 成功");

        let result = testtb.mdelete(&id2, "inventory", "尝试删除");
        assert!(result.is_err(), "inventory 不应该能删除");
        println!("  - mdelete 拒绝: {}", result.unwrap_err());

        println!("\n【3】trade 调用（只允许 mlist/getone）");
        let _list = testtb
            .mlist("trade", 10, "交易服务查询")
            .expect("trade 应该能列表");
        println!("  - mlist 成功");

        let _found = testtb
            .getone(&id2, "trade", "交易服务查询单条")
            .expect("trade 应该能查询");
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

        let result = testtb.getone(&id2, "unknown", "未知调用");
        assert!(result.is_err());
        println!("  - getone 拒绝: {}", result.unwrap_err());

        println!("\n=== 测试完成 ===");
    }

    #[test]
    fn test_summary_format() {
        println!("\n=== 摘要格式示例 ===\n");

        let testtb = TestTb::new();
        setup_permissions(&testtb);

        let record = TestTbRecord {
            idpk: 0,
            id: String::new(),
            cid: "c1".into(),
            kind: "order".into(),
            item: "item123".into(),
            data: "d1".into(),
            upby: String::new(),
            uptime: String::new(),
        };

        println!("【真实场景摘要示例】");

        let id1 = testtb
            .msave(
                &record,
                "inventory",
                "库存服务修改订单号: 订单123状态改为已发货",
            )
            .unwrap();
        println!("  inventory msave: 库存服务修改订单号: 订单123状态改为已发货");

        let _list = testtb
            .mlist("trade", 10, "交易服务获取订单: 用于创建交易记录")
            .unwrap();
        println!("  trade mlist: 交易服务获取订单: 用于创建交易记录");

        testtb
            .mdelete(&id1, "testtb", "内部清理: 删除测试数据")
            .unwrap();
        println!("  testtb mdelete: 内部清理: 删除测试数据");

        println!("\n=== 摘要格式示例完成 ===");
    }
}
