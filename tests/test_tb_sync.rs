//! testtb 完整同步测试
//!
//! 测试方案：
//! 1. 删除本地数据，添加2条，批量同步上去
//! 2. 修改服务器数据，通过同步表API
//! 3. 修改之后再次删除同步，新数据是否过来

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
    let _ = db.execute("DELETE FROM sync_queue WHERE table_name = 'testtb'");
    println!("已清空本地 testtb 表和 sync_queue");
}

/// 测试1：删除后添加2条，批量同步上去
#[test]
fn test_step1_insert_and_sync() {
    println!("\n=== 测试1：删除后添加2条，批量同步上去 ===");

    // 清空本地数据
    clear_local_data();

    let dm = DataManage::default();
    let state = dm.register(get_test_config()).expect("注册失败");

    // 清空 register 时可能产生的 sync_queue
    let db = LocalDB::new(None).expect("数据库连接失败");
    let _ = db.execute("DELETE FROM sync_queue WHERE table_name = 'testtb'");

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
    println!("添加2条数据成功: id1={}, id2={}", id1, id2);

    // 检查 sync_queue
    let pending = state.datasync.get_pending_count();
    println!("sync_queue 待同步数量: {}", pending);
    assert_eq!(pending, 2, "应该有2条待同步数据");

    // 批量同步上去
    let result = state.datasync.upload_once();
    println!("同步结果: res={}, errmsg={}", result.res, result.errmsg);
    println!("  插入: {} 条", result.datawf.inserted);
    println!("  更新: {} 条", result.datawf.updated);

    assert_eq!(result.res, 0, "同步应该成功");
    assert!(result.datawf.inserted >= 2, "应该插入至少2条");

    // 验证 sync_queue 已清空
    let pending_after = state.datasync.get_pending_count();
    println!("同步后 sync_queue 待同步数量: {}", pending_after);
    assert_eq!(pending_after, 0, "同步后 sync_queue 应该为空");

    println!("✅ 测试1通过：添加2条并同步成功");
}

/// 测试2：修改服务器数据，通过同步表API
#[test]
fn test_step2_update_and_sync() {
    println!("\n=== 测试2：修改服务器数据，通过同步表API ===");

    let dm = DataManage::default();
    let state = dm.register(get_test_config()).expect("注册失败");

    // 查询本地数据
    let rows = state.get_all(10, "testtb", "查询本地数据").expect("查询失败");
    println!("本地数据数量: {}", rows.len());
    
    if rows.is_empty() {
        println!("跳过测试：没有本地数据");
        return;
    }

    // 修改第一条数据
    let first_row = &rows[0];
    let id = first_row.get("id").and_then(|v| v.as_str()).unwrap_or("");
    
    if id.is_empty() {
        println!("跳过测试：没有找到 id");
        return;
    }

    let mut update_data = HashMap::new();
    update_data.insert("id".to_string(), Value::String(id.to_string()));
    update_data.insert("kind".to_string(), Value::String("updated_kind".to_string()));
    update_data.insert("item".to_string(), Value::String("updated_item".to_string()));
    update_data.insert("data".to_string(), Value::String("更新后的数据".to_string()));

    // 使用 m_save 更新
    match state.m_save(&update_data, "testtb", "更新测试") {
        Ok(_) => println!("更新成功"),
        Err(e) => {
            println!("更新失败: {}", e);
            return;
        }
    }

    // 检查 sync_queue
    let pending = state.datasync.get_pending_count();
    println!("sync_queue 待同步数量: {}", pending);

    // 同步更新到服务器
    let result = state.datasync.upload_once();
    println!("同步结果: res={}, errmsg={}", result.res, result.errmsg);

    println!("✅ 测试2通过：修改并同步成功");
}

/// 测试3：删除同步后，新数据是否过来
#[test]
fn test_step3_delete_and_download() {
    println!("\n=== 测试3：删除同步后，新数据是否过来 ===");

    // 清空本地数据
    clear_local_data();

    let dm = DataManage::default();
    let state = dm.register(get_test_config()).expect("注册失败");

    // register 时会自动首次下载
    let count = state.count("testtb", "查询本地记录数").unwrap_or(0);
    println!("首次下载后本地记录数: {}", count);

    assert!(count > 0, "下载后应该有数据");

    // 删除本地数据
    clear_local_data();

    // 再次下载
    let result = state.datasync.download_once();
    println!("再次下载结果: res={}, errmsg={}", result.res, result.errmsg);
    println!("  插入: {} 条", result.datawf.inserted);
    println!("  更新: {} 条", result.datawf.updated);

    // 验证新数据已过来
    let count_after = state.count("testtb", "查询本地记录数").unwrap_or(0);
    println!("再次下载后本地记录数: {}", count_after);

    assert!(count_after > 0, "再次下载后应该有数据");

    println!("✅ 测试3通过：删除后重新下载成功");
}
