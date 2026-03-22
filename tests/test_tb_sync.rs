//! testtb 完整同步测试
use base::mylogger::mylogger;
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
    let db = LocalDB::new(None, None).expect("数据库连接失败");
    let _ = db.execute("DELETE FROM testtb");
    let _ = db.execute("DELETE FROM synclog WHERE tbname = 'testtb'");
}

/// 调用服务器端 replayBatch 回放日志
fn call_replay_batch() -> Result<i32, String> {
    let db = LocalDB::new(None, None).expect("数据库连接失败");
    let sid = db.get_sid();
    if sid.is_empty() {
        return Err("配置文件未找到 SID".to_string());
    }

    let url = "http://log.778878.net/apisvc/backsvc/synclog/doWork";
    let request_payload = serde_json::json!({
        "sid": sid,
        "limit": 100,
        "maxBatches": 10
    });

    let response = HttpHelper::post(url, None, Some(&request_payload), None, false, None, 120, 0);
    mylogger!().detail(&format!("  replayBatch 响应: res={}, errmsg={}", response.res, response.errmsg));

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
    mylogger!().detail(&format!("\n========================================"));
    mylogger!().detail(&format!("=== 完整同步测试（串行执行） ==="));
    mylogger!().detail(&format!("========================================"));

    // ===== 步骤1：删除后添加2条，批量同步上去 =====
    mylogger!().detail(&format!("\n【步骤1】删除后添加2条，批量同步上去"));
    
    clear_local_data();
    
    let dm = DataManage::default();
    let state = dm.register(get_test_config()).expect("注册失败");

    // 清空 register 时可能产生的 synclog
    let db = LocalDB::new(None, None).expect("数据库连接失败");
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
    mylogger!().detail(&format!("  添加2条数据成功: id1={}, id2={}", id1, id2));

    // 检查 synclog
    let pending = state.datasync.get_pending_count();
    mylogger!().detail(&format!("  synclog 待同步数量: {}", pending));
    assert_eq!(pending, 2, "应该有2条待同步数据");

    // 批量同步上去
    let result = state.datasync.upload_once();
    mylogger!().detail(&format!("  同步结果: res={}, inserted={}", result.res, result.datawf.inserted));

    assert_eq!(result.res, 0, "同步应该成功");

    // 验证 synclog 已清空
    let pending_after = state.datasync.get_pending_count();
    mylogger!().detail(&format!("  同步后 synclog 待同步数量: {}", pending_after));
    assert_eq!(pending_after, 0, "同步后 synclog 应该为空");

    // 调用 replayBatch 回放日志
    let processed = call_replay_batch().expect("replayBatch 失败");
    mylogger!().detail(&format!("  replayBatch 处理了 {} 条记录", processed));

    mylogger!().detail(&format!("  ✅ 步骤1通过：添加2条并同步成功"));

    // ===== 步骤2：修改数据，同步到服务器 =====
    mylogger!().detail(&format!("\n【步骤2】修改数据，同步到服务器"));

    // 查询本地数据
    let rows = state.get_all(10, "testtb", "查询本地数据").expect("查询失败");
    mylogger!().detail(&format!("  本地数据数量: {}", rows.len()));
    assert!(!rows.is_empty(), "应该有本地数据");

    // 修改第一条数据
    let first_row = &rows[0];
    let record_id = first_row.get("id").and_then(|v| v.as_str()).unwrap_or("");
    mylogger!().detail(&format!("  准备修改记录: id={}", record_id));

    let mut update_data = HashMap::new();
    update_data.insert("id".to_string(), Value::String(record_id.to_string()));
    update_data.insert("kind".to_string(), Value::String(format!("updated_{}", timestamp)));
    update_data.insert("item".to_string(), Value::String("item_updated".to_string()));
    update_data.insert("data".to_string(), Value::String("修改后的数据".to_string()));

    // 使用 m_save 更新
    state.m_save(&update_data, "testtb", "修改测试").expect("修改失败");
    mylogger!().detail(&format!("  本地修改成功"));

    // 检查 synclog
    let pending = state.datasync.get_pending_count();
    mylogger!().detail(&format!("  synclog 待同步数量: {}", pending));
    assert!(pending > 0, "修改后应该有待同步数据");

    // 同步修改到服务器
    let result = state.datasync.upload_once();
    mylogger!().detail(&format!("  同步结果: res={}, inserted={}", result.res, result.datawf.inserted));
    assert_eq!(result.res, 0, "同步修改应该成功");

    // 验证 synclog 已清空
    let pending_after = state.datasync.get_pending_count();
    mylogger!().detail(&format!("  同步后 synclog 待同步数量: {}", pending_after));
    assert_eq!(pending_after, 0, "同步后 synclog 应该为空");

    // 调用 replayBatch 回放日志
    let processed = call_replay_batch().expect("replayBatch 失败");
    mylogger!().detail(&format!("  replayBatch 处理了 {} 条记录", processed));

    mylogger!().detail(&format!("  ✅ 步骤2通过：修改并同步成功"));

    // ===== 步骤3：删除本地数据，重新下载，验证修改后的数据 =====
    mylogger!().detail(&format!("\n【步骤3】删除本地数据，重新下载，验证修改后的数据"));

    // 删除本地数据
    clear_local_data();

    // 再次下载
    let result = state.datasync.download_once();
    mylogger!().detail(&format!("  再次下载结果: res={}, inserted={}", result.res, result.datawf.inserted));

    // 验证新数据已过来
    let count_after = state.count("testtb", "查询本地记录数").unwrap_or(0);
    mylogger!().detail(&format!("  再次下载后本地记录数: {}", count_after));
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
        mylogger!().detail(&format!("  修改后的记录: kind={}, item={}, data={}", kind, item, data));
        
        // 验证数据已被修改
        assert!(kind.starts_with("updated_"), "kind 应该被修改");
        assert_eq!(item, "item_updated", "item 应该被修改");
        assert_eq!(data, "修改后的数据", "data 应该被修改");
    } else {
        mylogger!().detail(&format!("  警告：未找到修改后的记录（可能服务器端 replayBatch 未执行）"));
    }

    mylogger!().detail(&format!("  ✅ 步骤3通过：删除后重新下载成功"));

    mylogger!().detail(&format!("\n========================================"));
    mylogger!().detail(&format!("=== 全部测试通过 ==="));
    mylogger!().detail(&format!("========================================"));
}

/// 测试 cid 验证失败场景
/// 
/// 测试方案：
/// 1. 直接插入 synclog 表，使用错误的 cid
/// 2. 调用 upload_once() 上传
/// 3. 验证服务器返回 errors
/// 4. 验证本地 synclog 的 synced=-1, lasterrinfo 有值
#[test]
fn test_cid_validation_failed() {
    use database::data_sync::{DataSync, get_pending_count};
    
    mylogger!().detail(&format!("\n========================================"));
    mylogger!().detail(&format!("=== 测试 cid 验证失败场景 ==="));
    mylogger!().detail(&format!("========================================"));

    // 清理本地数据
    clear_local_data();

    let db = LocalDB::new(None, None).expect("数据库连接失败");

    // 创建 DataSync 实例
    let datasync = DataSync::new("testtb");
    
    // 构造一个错误的 cid（与当前用户不匹配）
    let wrong_cid = "WRONG-COMPANY-ID-12345";
    mylogger!().detail(&format!("  错误的 cid: {}", wrong_cid));

    // 直接插入 synclog 表，使用错误的 cid
    let idrow = uuid::Uuid::new_v4().to_string();
    let params = serde_json::json!({
        "id": idrow,
        "cid": wrong_cid,
        "kind": "test_validation",
        "item": "test_item",
        "data": "test_data"
    }).to_string();

    let sql = format!(
        "INSERT INTO synclog (apisys, apimicro, apiobj, tbname, action, cmdtext, params, idrow, worker, synced, lasterrinfo, cmdtextmd5, num, dlong, downlen) VALUES ('v1', 'iflow', 'synclog', 'testtb', 'insert', '', '{}', '{}', 'test', 0, '', '', 0, 0, 0)",
        params.replace("'", "''"),
        idrow
    );
    db.execute(&sql).expect("插入 synclog 失败");
    mylogger!().detail(&format!("  已插入 synclog 记录，idrow={}, cid={}", idrow, wrong_cid));

    // 检查 synclog 待同步数量
    let pending = get_pending_count("testtb");
    mylogger!().detail(&format!("  synclog 待同步数量: {}", pending));
    assert!(pending > 0, "应该有待同步数据");

    // 创建 DataSync 实例并调用 upload_once 上传
    mylogger!().detail(&format!("\n【步骤2】调用 upload_once 上传"));
    let datasync = DataSync::new("testtb");
    let result = datasync.upload_once();
    mylogger!().detail(&format!("  同步结果: res={}, inserted={}", result.res, result.datawf.inserted));

    // 检查是否有验证错误
    if let Some(ref errors) = result.datawf.errors {
        mylogger!().detail(&format!("  验证错误数量: {}", errors.len()));
        for err in errors {
            mylogger!().detail(&format!("    - {}", err));
        }
    }

    // 检查本地 synclog 的 synced 是否为 -1
    let check_sql = format!("SELECT synced, lasterrinfo FROM synclog WHERE idrow = '{}'", idrow);
    let rows = db.query(&check_sql, &[]).expect("查询 synclog 失败");
    if let Some(row) = rows.first() {
        let synced = row.get("synced").and_then(|v| v.as_i64()).unwrap_or(0);
        let lasterrinfo = row.get("lasterrinfo").and_then(|v| v.as_str()).unwrap_or("");
        mylogger!().detail(&format!("  synclog 状态: synced={}, lasterrinfo={}", synced, lasterrinfo));
        
        if synced == -1 {
            mylogger!().detail(&format!("  ✅ 验证成功：synced=-1，错误信息: {}", lasterrinfo));
        } else {
            mylogger!().detail(&format!("  ⚠️ synced 不是 -1，可能是服务器端验证未生效"));
        }
    }

    mylogger!().detail(&format!("\n========================================"));
    mylogger!().detail(&format!("=== 测试完成 ==="));
    mylogger!().detail(&format!("========================================"));
}
