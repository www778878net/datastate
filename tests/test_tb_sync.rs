//! testtb 完整同步测试
//!
//! 测试方案：
//! 1. 删除本地数据，添加2条，批量同步上去
//! 2. 修改之后再次删除同步，新数据是否过来

use database::{DataManage, TableConfig, get_system_columns, LocalDB};
use std::collections::HashMap;
use serde_json::Value;

fn get_test_config() -> TableConfig {
    let mut columns = get_system_columns();
    columns.insert("kind".to_string(), "TEXT NOT NULL DEFAULT ''".to_string());
    columns.insert("item".to_string(), "TEXT NOT NULL DEFAULT ''".to_string());
    columns.insert("data".to_string(), "TEXT NOT NULL DEFAULT ''".to_string());
    TableConfig {
        name: "testtb".to_string(),
        apiurl: "http://log.778878.net/apitest/testmenu/testtb".to_string(),
        columns,
        upload_cols: Some(vec!["kind".to_string(), "item".to_string(), "data".to_string()]),
        ..Default::default()
    }
}

fn clear_local_data() {
    let db = LocalDB::new(None).expect("数据库连接失败");
    let _ = db.execute("DELETE FROM testtb");
    let _ = db.execute("DELETE FROM synclog WHERE tbname = 'testtb'");
}

/// 完整同步测试（串行执行所有步骤）
#[test]
fn test_full_sync_workflow() {
    println!("\n========================================");
    println!("=== 完整同步测试（串行执行） ===");
    println!("========================================");

    // ===== 步骤1：删除后添加2条，批量同步上去 =====
    println!("\n【步骤1】删除后添加2条，批量同步上去");
    
    clear_local_data();
    
    let dm = DataManage::default();
    let state = dm.register(get_test_config()).expect("注册失败");

    // 清空 register 时可能产生的 synclog
    let db = LocalDB::new(None).expect("数据库连接失败");
    let _ = db.execute("DELETE FROM synclog WHERE tbname = 'testtb'");

    // 添加2条数据
    let timestamp = chrono::Local::now().format("%Y%m%d%H%M%S").to_string();
    
    let mut data1 = HashMap::new();
    data1.insert("kind".to_string(), Value::String(format!("test_{}", timestamp)));
    data1.insert("item".to_string(), Value::String("item_1".to_string()));
    data1.insert("data".to_string(), Value::String("测试数据1".to_string()));

    let mut data2 = HashMap::new();
    data2.insert("kind".to_string(), Value::String(format!("test_{}", timestamp)));
    data2.insert("item".to_string(), Value::String("item_2".to_string()));
    data2.insert("data".to_string(), Value::String("测试数据2".to_string()));

    let id1 = state.m_add(&data1, "testtb", "添加第1条").expect("添加失败");
    let id2 = state.m_add(&data2, "testtb", "添加第2条").expect("添加失败");
    println!("  添加2条数据成功: id1={}, id2={}", id1, id2);

    // 检查 synclog
    let pending = state.datasync.get_pending_count();
    println!("  synclog 待同步数量: {}", pending);
    assert_eq!(pending, 2, "应该有2条待同步数据");

    // 批量同步上去
    let result = state.datasync.upload_once();
    println!("  同步结果: res={}, inserted={}", result.res, result.datawf.inserted);

    assert_eq!(result.res, 0, "同步应该成功");

    // 验证 synclog 已清空
    let pending_after = state.datasync.get_pending_count();
    println!("  同步后 synclog 待同步数量: {}", pending_after);
    assert_eq!(pending_after, 0, "同步后 synclog 应该为空");

    println!("  ✅ 步骤1通过：添加2条并同步成功");

    // ===== 步骤2：删除同步后，新数据是否过来 =====
    println!("\n【步骤2】删除同步后，新数据是否过来");

    // 删除本地数据
    clear_local_data();

    // 再次下载
    let result = state.datasync.download_once();
    println!("  再次下载结果: res={}, inserted={}", result.res, result.datawf.inserted);

    // 验证新数据已过来
    let count_after = state.count("testtb", "查询本地记录数").unwrap_or(0);
    println!("  再次下载后本地记录数: {}", count_after);

    assert!(count_after > 0, "再次下载后应该有数据");

    println!("  ✅ 步骤2通过：删除后重新下载成功");

    println!("\n========================================");
    println!("=== 全部测试通过 ===");
    println!("========================================");
}
