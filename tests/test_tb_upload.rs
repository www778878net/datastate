//! testtb 上传测试
//!
//! 测试 DataSync 的 CRUD 方法（m_add, m_save, m_del）

use database::{DataManage, TableConfig, get_system_columns, DataState};
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
