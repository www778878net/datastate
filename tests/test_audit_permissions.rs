//! 测试 testtb 权限控制
//!
//! 直接基于 testtb 表结构，用真实数据测试权限控制

use database::{
    LocalDB,
    dataaudit::DataAudit,
    audit,
    DATASTATE_AUDIT_CREATE_SQL, DATA_ABILITY_LOG_CREATE_SQL, DATA_ABILITY_DAILY_CREATE_SQL,
    SYNC_QUEUE_CREATE_SQL, DATA_STATE_LOG_CREATE_SQL, DATA_SYNC_STATS_CREATE_SQL,
    register_ability_simple, get_ability_perm,
};
use uuid::Uuid;

/// testtb 表建表 SQL
const TESTTB_DROP_SQL: &str = "DROP TABLE IF EXISTS testtb";
const TESTTB_CREATE_SQL: &str = r#"
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

/// TestTb - testtb 表
struct TestTb {
    db: LocalDB,
    audit: DataAudit,
}

impl TestTb {
    fn new() -> Self {
        Self {
            db: LocalDB::default_instance().expect("获取数据库失败"),
            audit: DataAudit::new("testtb"),
        }
    }

    fn init_tables(&self) -> Result<(), String> {
        let _ = self.db.execute(TESTTB_DROP_SQL);
        self.db.execute(TESTTB_CREATE_SQL)?;
        self.db.execute(SYNC_QUEUE_CREATE_SQL)?;
        self.db.execute(DATA_STATE_LOG_CREATE_SQL)?;
        self.db.execute(DATA_SYNC_STATS_CREATE_SQL)?;
        self.db.execute(DATASTATE_AUDIT_CREATE_SQL)?;
        self.db.execute(DATA_ABILITY_LOG_CREATE_SQL)?;
        self.db.execute(DATA_ABILITY_DAILY_CREATE_SQL)?;
        Ok(())
    }

    /// getone - 查询单条（支持跨服务调用）
    fn getone(&self, id: &str, caller: &str) -> Result<Option<serde_json::Value>, String> {
        audit!(self, "getone", caller, {
            let sql = "SELECT * FROM testtb WHERE id = ?";
            match self.db.query(sql, &[&id as &dyn rusqlite::ToSql]) {
                Ok(rows) if !rows.is_empty() => {
                    let mut result = serde_json::Map::new();
                    for (key, value) in rows[0].iter() {
                        result.insert(key.clone(), value.clone());
                    }
                    Ok(Some(serde_json::Value::Object(result)))
                }
                _ => Ok(None),
            }
        })
    }

    /// msave - 保存数据（支持跨服务调用）
    fn msave(&self, data: &serde_json::Value, caller: &str) -> Result<String, String> {
        let data_clone = data.clone();
        let caller_owned = caller.to_string();
        audit!(self, "msave", caller, {
            let id = Uuid::new_v4().to_string();

            let cid = data_clone.get("cid").and_then(|v| v.as_str()).unwrap_or("");
            let kind = data_clone.get("kind").and_then(|v| v.as_str()).unwrap_or("");
            let item = data_clone.get("item").and_then(|v| v.as_str()).unwrap_or("");
            let data_val = data_clone.get("data").and_then(|v| v.as_str()).unwrap_or("");

            let sql = "INSERT INTO testtb (id, cid, kind, item, data, upby, uptime) VALUES (?, ?, ?, ?, ?, ?, datetime('now'))";
            self.db.execute_with_params(sql, &[&id, &cid, &kind, &item, &data_val, &caller_owned])?;

            Ok(id)
        })
    }

    /// mdelete - 删除数据（支持跨服务调用）
    fn mdelete(&self, id: &str, caller: &str) -> Result<bool, String> {
        let id_owned = id.to_string();
        audit!(self, "mdelete", caller, {
            let sql = "DELETE FROM testtb WHERE id = ?";
            self.db.execute_with_params(sql, &[&id_owned])?;
            Ok(true)
        })
    }
}

#[test]
fn test_permission_with_real_data() {
    println!("=== testtb 权限测试（真实数据）===\n");

    let testtb = TestTb::new(); // 审计已在构造函数中开启
    testtb.init_tables().expect("初始化表失败");

    // 注册权限
    println!("【1】注册权限");
    
    // testtb 自己可以访问所有方法（同表调用）
    register_ability_simple(&testtb.db, "testtb", "*", "testtb", "同表调用").expect("注册失败");
    
    // readonly 只能 getone
    register_ability_simple(&testtb.db, "testtb", "getone", "readonly", "只读查询").expect("注册失败");
    
    println!("  - testtb -> testtb/* (同表调用)");
    println!("  - readonly -> testtb/getone (只读)\n");

    // 同表调用测试
    println!("【2】同表调用测试（testtb 调用自己的方法）");
    
    let data1 = serde_json::json!({"cid": "c1", "kind": "k1", "item": "i1", "data": "d1"});
    let id1 = testtb.msave(&data1, "testtb").expect("testtb 应该能保存");
    println!("  - testtb.msave 成功: id={}", id1);
    
    let row1 = testtb.getone(&id1, "testtb").expect("testtb 应该能查询");
    println!("  - testtb.getone 成功: {:?}", row1.is_some());
    
    let del1 = testtb.mdelete(&id1, "testtb").expect("testtb 应该能删除");
    println!("  - testtb.mdelete 成功: {}", del1);

    // 只读调用方测试
    println!("\n【3】只读调用方测试（readonly）");
    
    // 先插入一条数据
    let data2 = serde_json::json!({"cid": "c2", "kind": "k2", "item": "i2", "data": "d2"});
    let id2 = testtb.msave(&data2, "testtb").expect("testtb 保存");
    
    // readonly 可以查询
    let row2 = testtb.getone(&id2, "readonly").expect("readonly 应该能查询");
    println!("  - readonly.getone 成功: {:?}", row2.is_some());
    
    // readonly 不能保存
    let data3 = serde_json::json!({"cid": "c3", "kind": "k3", "item": "i3", "data": "d3"});
    let result = testtb.msave(&data3, "readonly");
    assert!(result.is_err(), "readonly 不应该能保存");
    println!("  - readonly.msave 拒绝: {}", result.unwrap_err());
    
    // readonly 不能删除
    let result = testtb.mdelete(&id2, "readonly");
    assert!(result.is_err(), "readonly 不应该能删除");
    println!("  - readonly.mdelete 拒绝: {}", result.unwrap_err());

    // 未注册调用方
    println!("\n【4】未注册调用方测试");
    let result = testtb.getone(&id2, "unknown");
    assert!(result.is_err(), "未注册调用方应该被拒绝");
    println!("  - unknown.getone 拒绝: {}", result.unwrap_err());

    // 查看权限记录
    println!("\n【5】权限记录");
    if let Some(p) = get_ability_perm(&testtb.db, "testtb", "*") {
        println!("  - testtb/*: caller={}", p.caller);
    }
    if let Some(p) = get_ability_perm(&testtb.db, "testtb", "getone") {
        println!("  - testtb/getone: caller={}", p.caller);
    }

    println!("\n=== 测试完成 ===");
}
