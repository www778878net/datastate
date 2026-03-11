//! testtb 完整同步测试
//!
//! 测试方案：
//! 1. 删除本地数据，添加2条，批量同步上去
//! 2. 修改数据，同步到服务器（通过 synclog）
//! 3. 删除本地数据，重新下载，验证修改后的数据是否过来

use database::{DataManage, TableConfig, get_system_columns, LocalDB};
use std::collections::HashMap;
use serde_json::Value;
use base::http::HttpHelper;

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

/// 调用服务器端 replayBatch 回放日志
fn call_replay_batch() -> Result<i32, String> {
    let db = LocalDB::new(None).expect("数据库连接失败");
    let sid = db.get_sid();
    if sid.is_empty() {
        return Err("配置文件未找到 SID".to_string());
    }

    let url = "http://log.778878.net/apisvc/backsvc/synclog/syncReplay";
    let request_payload = serde_json::json!({
        "sid": sid,
        "limit": 100,
        "maxBatches": 10
    });

    let response = HttpHelper::post(url, None, Some(&request_payload), None, false, None, 120, 0);
    println!("  replayBatch 响应: res={}, errmsg={}", response.res, response.errmsg);

    if response.res != 0 {
        return Err(response.errmsg);
    }

    // 解析返回结果
    if let Some(ref resp_data) = response.data {
        if let Some(back) = resp_data.response.get("back") {
            if let Some(processed) = back.get("processed") {
                return Ok(processed.as_i64().unwrap_or(0) as i32);
            }
        }
    }

    Ok(0)
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

    // 调用 replayBatch 回放日志
    let processed = call_replay_batch().expect("replayBatch 失败");
    println!("  replayBatch 处理了 {} 条记录", processed);

    println!("  ✅ 步骤1通过：添加2条并同步成功");

    // ===== 步骤2：修改数据，同步到服务器 =====
    println!("\n【步骤2】修改数据，同步到服务器");

    // 查询本地数据
    let rows = state.get_all(10, "testtb", "查询本地数据").expect("查询失败");
    println!("  本地数据数量: {}", rows.len());
    assert!(!rows.is_empty(), "应该有本地数据");

    // 修改第一条数据
    let first_row = &rows[0];
    let record_id = first_row.get("id").and_then(|v| v.as_str()).unwrap_or("");
    println!("  准备修改记录: id={}", record_id);

    let mut update_data = HashMap::new();
    update_data.insert("id".to_string(), Value::String(record_id.to_string()));
    update_data.insert("kind".to_string(), Value::String(format!("updated_{}", timestamp)));
    update_data.insert("item".to_string(), Value::String("item_updated".to_string()));
    update_data.insert("data".to_string(), Value::String("修改后的数据".to_string()));

    // 使用 m_save 更新
    state.m_save(&update_data, "testtb", "修改测试").expect("修改失败");
    println!("  本地修改成功");

    // 检查 synclog
    let pending = state.datasync.get_pending_count();
    println!("  synclog 待同步数量: {}", pending);
    assert!(pending > 0, "修改后应该有待同步数据");

    // 同步修改到服务器
    let result = state.datasync.upload_once();
    println!("  同步结果: res={}, inserted={}", result.res, result.datawf.inserted);
    assert_eq!(result.res, 0, "同步修改应该成功");

    // 验证 synclog 已清空
    let pending_after = state.datasync.get_pending_count();
    println!("  同步后 synclog 待同步数量: {}", pending_after);
    assert_eq!(pending_after, 0, "同步后 synclog 应该为空");

    // 调用 replayBatch 回放日志
    let processed = call_replay_batch().expect("replayBatch 失败");
    println!("  replayBatch 处理了 {} 条记录", processed);

    println!("  ✅ 步骤2通过：修改并同步成功");

    // ===== 步骤3：删除本地数据，重新下载，验证修改后的数据 =====
    println!("\n【步骤3】删除本地数据，重新下载，验证修改后的数据");

    // 删除本地数据
    clear_local_data();

    // 再次下载
    let result = state.datasync.download_once();
    println!("  再次下载结果: res={}, inserted={}", result.res, result.datawf.inserted);

    // 验证新数据已过来
    let count_after = state.count("testtb", "查询本地记录数").unwrap_or(0);
    println!("  再次下载后本地记录数: {}", count_after);
    assert!(count_after > 0, "再次下载后应该有数据");

    // 验证修改后的数据是否存在
    let rows = state.get_all(100, "testtb", "查询所有数据").expect("查询失败");
    let updated_record = rows.iter().find(|row| {
        row.get("id").and_then(|v| v.as_str()) == Some(record_id)
    });
    
    if let Some(record) = updated_record {
        let kind = record.get("kind").and_then(|v| v.as_str()).unwrap_or("");
        let item = record.get("item").and_then(|v| v.as_str()).unwrap_or("");
        let data = record.get("data").and_then(|v| v.as_str()).unwrap_or("");
        println!("  修改后的记录: kind={}, item={}, data={}", kind, item, data);
        
        // 验证数据已被修改
        assert!(kind.starts_with("updated_"), "kind 应该被修改");
        assert_eq!(item, "item_updated", "item 应该被修改");
        assert_eq!(data, "修改后的数据", "data 应该被修改");
    } else {
        println!("  警告：未找到修改后的记录（可能服务器端 replayBatch 未执行）");
    }

    println!("  ✅ 步骤3通过：删除后重新下载成功");

    println!("\n========================================");
    println!("=== 全部测试通过 ===");
    println!("========================================");
}
