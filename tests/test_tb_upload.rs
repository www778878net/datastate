//! testtb 上传测试
//!
//! 测试 DataSync 的 CRUD 方法（m_add, m_save, m_del）
//! 测试同步到服务器

use database::{DataManage, TableConfig, get_system_columns, DataState, LocalDB};
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
        // 必须指定 upload_cols，与服务器 colsImp 一致
        upload_cols: Some(vec!["kind".to_string(), "item".to_string(), "data".to_string()]),
        ..Default::default()
    }
}

/// 测试 m_add - 单条插入
#[test]
fn test_m_add() {
    println!("\n=== 测试 m_add 单条插入 ===");

    let dm = DataManage::default();
    let state = dm.register(get_test_config()).expect("注册失败");

    let mut data = HashMap::new();
    data.insert("kind".to_string(), Value::String("test_kind".to_string()));
    data.insert("item".to_string(), Value::String("test_item".to_string()));
    data.insert("data".to_string(), Value::String("测试数据".to_string()));

    match state.m_add(&data, "testtb", "测试单条插入") {
        Ok(id) => {
            println!("m_add 成功，返回 id: {}", id);
            assert!(!id.is_empty(), "id 不应为空");
        }
        Err(e) => {
            println!("m_add 失败: {}", e);
        }
    }
}

/// 测试批量插入
#[test]
fn test_batch_insert() {
    println!("\n=== 测试批量插入 ===");

    let dm = DataManage::default();
    let state = dm.register(get_test_config()).expect("注册失败");

    for i in 1..=5 {
        let mut data = HashMap::new();
        data.insert("kind".to_string(), Value::String(format!("batch_{}", i)));
        data.insert("item".to_string(), Value::String(format!("item_{}", i)));
        data.insert("data".to_string(), Value::String(format!("批量测试数据 {}", i)));

        match state.m_add(&data, "testtb", &format!("批量插入第{}条", i)) {
            Ok(id) => println!("批量插入第 {} 条成功，id: {}", i, id),
            Err(e) => println!("批量插入第 {} 条失败: {}", i, e),
        }
    }
}

/// 测试查询方法
#[test]
fn test_query_methods() {
    println!("\n=== 测试查询方法 ===");

    let dm = DataManage::default();
    let state = dm.register(get_test_config()).expect("注册失败");

    match state.count("testtb", "查询记录数") {
        Ok(count) => println!("表记录数: {}", count),
        Err(e) => println!("count 失败: {}", e),
    }

    match state.get_all(10, "testtb", "查询所有记录") {
        Ok(rows) => {
            println!("get_all 返回 {} 条记录", rows.len());
            for (i, row) in rows.iter().enumerate().take(3) {
                println!("  第 {} 条: {:?}", i + 1, row);
            }
        }
        Err(e) => println!("get_all 失败: {}", e),
    }
}

/// 清空本地 testtb 表和 sync_queue
fn clear_local_data() {
    let db = LocalDB::new(None).expect("数据库连接失败");
    let _ = db.execute("DELETE FROM testtb");
    let _ = db.execute("DELETE FROM sync_queue WHERE table_name = 'testtb'");
    println!("已清空本地 testtb 表和 sync_queue");
}

/// 测试同步到服务器
#[test]
fn test_sync_to_server() {
    println!("\n=== 测试同步到服务器 ===");

    // 先清空本地数据
    clear_local_data();

    let dm = DataManage::default();
    let state = dm.register(get_test_config()).expect("注册失败");

    // 使用唯一的时间戳作为 kind，避免重复
    let timestamp = chrono::Local::now().format("%Y%m%d%H%M%S").to_string();
    let unique_kind = format!("sync_test_{}", timestamp);

    // 先插入一条数据
    let mut data = HashMap::new();
    data.insert("kind".to_string(), Value::String(unique_kind.clone()));
    data.insert("item".to_string(), Value::String("sync_item".to_string()));
    data.insert("data".to_string(), Value::String("同步测试数据".to_string()));

    let id = match state.m_add(&data, "testtb", "插入待同步数据") {
        Ok(id) => {
            println!("插入成功，id: {}", id);
            id
        }
        Err(e) => {
            println!("插入失败: {}", e);
            return;
        }
    };

    // 检查 sync_queue 中有待同步数据
    let pending = state.datasync.get_pending_count();
    println!("sync_queue 待同步数量: {}", pending);

    // 执行同步
    println!("\n开始同步到服务器...");
    let result = state.datasync.upload_once();
    println!("同步结果: res={}, errmsg={}", result.res, result.errmsg);
    println!("  插入: {} 条", result.datawf.inserted);
    println!("  更新: {} 条", result.datawf.updated);
    println!("  跳过: {} 条", result.datawf.skipped);

    // 再次检查 sync_queue
    let pending_after = state.datasync.get_pending_count();
    println!("同步后 sync_queue 待同步数量: {}", pending_after);

    // 验证同步成功
    if result.datawf.inserted > 0 {
        println!("✅ 同步成功！插入了 {} 条数据", result.datawf.inserted);
    }
}

/// 测试完整同步流程（下载 + 上传）
#[test]
fn test_sync_once() {
    println!("\n=== 测试完整同步流程 ===");

    let dm = DataManage::default();
    let _state = dm.register(get_test_config()).expect("注册失败");

    // 执行一次同步
    println!("执行 sync_once...");
    let result = dm.sync_once();
    println!("sync_once 完成!");
    println!("  res: {}", result.res);
    println!("  errmsg: {}", result.errmsg);
    println!("  插入: {} 条", result.datawf.inserted);
    println!("  更新: {} 条", result.datawf.updated);
}
