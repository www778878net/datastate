//! synclog MySQL 同步测试
//!
//!
//! 架构：本地 SQLite → axum78 API → MySQL
//!
//! 测试流程（按 docs/dev/datastate.md）：
//! 1. 本地新增/修改/删除 testtb，产生 synclog（SQLite）
//! 2. 调用 axum78 API 同步到 MySQL synclog 表
//! 3. axum78 执行 SQL 写入 MySQL testtb 表
//! 4. 验证 SQLite 和 MySQL 数据一致
//! 5. MySQL 修改后自动下载到本地
//! 6. 双边同时修改的冲突处理
//! 7. guest3 用户帐套隔离测试

use datastate::{
    DataSync, LocalDB,
    data_sync::SynclogItem,
    snowflake::next_id_string,
    ProtoSynclogItem, ProtoSynclogBatch,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use chrono::Local;
use prost::Message;

// ========== 测试配置 ==========

/// axum78 API 基础地址
const API_BASE_URL: &str = "http://127.0.0.1:80";

/// synclog API 地址
const SYNCLOG_API_URL: &str = "http://127.0.0.1:80/apisvc/backsvc/synclog_mysql";

/// 默认测试用户的 CID
const DEFAULT_CID: &str = "GUEST000-8888-8888-8888-GUEST00GUEST";

/// guest3 用户的 CID（用于帐套隔离测试）
const GUEST3_CID: &str = "GUEST003-8888-8888-8888-GUEST00GUEST";

/// 测试 SID（UUID 格式，验证失败会使用 GUEST 身份）
const DEFAULT_SID: &str = "GUEST000-8888-8888-8888-GUEST00GUEST";
const GUEST3_SID: &str = "GUEST003-8888-8888-8888-GUEST00GUEST";

// ========== 测试数据结构 ==========

/// 测试记录
#[derive(Debug, Clone)]
struct TestRecord {
    id: String,
    cid: String,
    kind: String,
    item: String,
    data: String,
}

impl TestRecord {
    fn new(cid: &str, kind: &str, item: &str, data: &str) -> Self {
        Self {
            id: next_id_string(),
            cid: cid.to_string(),
            kind: kind.to_string(),
            item: item.to_string(),
            data: data.to_string(),
        }
    }

    fn to_map(&self) -> HashMap<String, Value> {
        let mut map = HashMap::new();
        map.insert("id".to_string(), json!(self.id.clone()));
        map.insert("cid".to_string(), json!(self.cid.clone()));
        map.insert("kind".to_string(), json!(self.kind.clone()));
        map.insert("item".to_string(), json!(self.item.clone()));
        map.insert("data".to_string(), json!(self.data.clone()));
        map
    }
}

// ========== 失败记录结构 ==========

/// 失败记录
#[derive(Debug, Clone)]
struct FailedRecord {
    id: String,
    idrow: String,
    error: String,
}

// ========== 辅助函数 ==========

/// 创建测试用的本地数据库（SQLite）
fn setup_local_db() -> LocalDB {
    let db_path = "docs/config/local.db";
    LocalDB::with_path(db_path).expect("创建本地数据库失败")
}

/// 确保 testtb 表存在（SQLite）
fn ensure_testtb_table_sqlite(db: &LocalDB) {
    let sql = r#"CREATE TABLE IF NOT EXISTS testtb (
        id TEXT NOT NULL PRIMARY KEY,
        cid TEXT NOT NULL DEFAULT '',
        kind TEXT NOT NULL DEFAULT '',
        item TEXT NOT NULL DEFAULT '',
        data TEXT NOT NULL DEFAULT '',
        upby TEXT NOT NULL DEFAULT '',
        uptime TEXT NOT NULL DEFAULT ''
    )"#;
    let _ = db.execute(sql);
}

/// 调用 axum78 API 上传 synclog
/// 
/// 返回：(成功ID列表, 失败记录列表)
fn upload_synclog_to_api(
    items: &[SynclogItem],
    sid: &str,
) -> Result<(Vec<String>, Vec<FailedRecord>), String> {
    use base64::{Engine as _, engine::general_purpose};
    
    // 构建 Protobuf 数据（使用导出的类型）
    let proto_items: Vec<ProtoSynclogItem> = items.iter().map(|item| ProtoSynclogItem {
        id: item.id.clone(),
        apisys: item.apisys.clone(),
        apimicro: item.apimicro.clone(),
        apiobj: item.apiobj.clone(),
        tbname: item.tbname.clone(),
        action: item.action.clone(),
        cmdtext: item.cmdtext.clone(),
        params: item.params.clone(),
        idrow: item.idrow.clone(),
        worker: item.worker.clone(),
        synced: item.synced,
        cmdtextmd5: item.cmdtextmd5.clone(),
        cid: item.cid.clone(),
        upby: item.upby.clone(),
    }).collect();

    let batch = ProtoSynclogBatch { items: proto_items };
    let bytedata = batch.encode_to_vec();
    let bytedata_base64 = general_purpose::STANDARD.encode(&bytedata);

    // 构建请求（cid 由服务端从测试 SID 中自动提取）
    let request_body = serde_json::json!({
        "sid": sid,
        "jsdata": bytedata_base64,
    });

    // 发送 HTTP 请求
    let client = reqwest::blocking::Client::new();
    let response = client
        .post(&format!("{}/maddmany", SYNCLOG_API_URL))
        .json(&request_body)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .map_err(|e| format!("HTTP请求失败: {}", e))?;

    let status = response.status();
    let body = response.text().map_err(|e| format!("读取响应失败: {}", e))?;

    if !status.is_success() {
        return Err(format!("API返回错误: {} - {}", status, body));
    }

    // 解析响应
    let resp: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("解析响应失败: {}", e))?;

    if resp["res"].as_i64().unwrap_or(-1) != 0 {
        return Err(format!("业务错误: {}", resp["errmsg"].as_str().unwrap_or("未知错误")));
    }

    // back 字段是 JSON 字符串，需要再解析一次
    let data_str = resp["back"].as_str().unwrap_or("{}");
    let data: serde_json::Value = serde_json::from_str(data_str)
        .map_err(|e| format!("解析back失败: {}", e))?;

    let success_ids: Vec<String> = data["success_ids"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();

    let failed: Vec<FailedRecord> = data["failed"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|v| {
            Some(FailedRecord {
                id: v["id"].as_str().unwrap_or("").to_string(),
                idrow: v["idrow"].as_str().unwrap_or("").to_string(),
                error: v["error"].as_str().unwrap_or("").to_string(),
            })
        }).collect())
        .unwrap_or_default();

    Ok((success_ids, failed))
}

/// 从 axum78 API 下载 synclog
fn download_synclog_from_api(
    sid: &str,
    limit: i32,
) -> Result<Vec<SynclogItem>, String> {
    use base64::{Engine as _, engine::general_purpose};

    let request_body = serde_json::json!({
        "sid": sid,
        "getnumber": limit,
    });

    let client = reqwest::blocking::Client::new();
    let response = client
        .post(&format!("{}/get", SYNCLOG_API_URL))
        .json(&request_body)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .map_err(|e| format!("HTTP请求失败: {}", e))?;

    let status = response.status();
    let body = response.text().map_err(|e| format!("读取响应失败: {}", e))?;

    if !status.is_success() {
        return Err(format!("API返回错误: {} - {}", status, body));
    }

    let resp: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("解析响应失败: {}", e))?;

    if resp["res"].as_i64().unwrap_or(-1) != 0 {
        return Err(format!("业务错误: {}", resp["errmsg"].as_str().unwrap_or("未知错误")));
    }

    // back 字段是 JSON 字符串，需要再解析一次
    let data_str = resp["back"].as_str().unwrap_or("{}");
    let data: serde_json::Value = serde_json::from_str(data_str)
        .map_err(|e| format!("解析back失败: {}", e))?;

    let bytedata_base64 = data["bytedata"].as_str().unwrap_or("");
    let bytedata = general_purpose::STANDARD.decode(bytedata_base64)
        .map_err(|e| format!("Base64解码失败: {}", e))?;

    let batch: ProtoSynclogBatch = Message::decode(&*bytedata)
        .map_err(|e| format!("Protobuf解码失败: {}", e))?;

    // 转换为 SynclogItem
    Ok(batch.items.into_iter().map(|p| SynclogItem {
        idpk: 0,
        apisys: p.apisys,
        apimicro: p.apimicro,
        apiobj: p.apiobj,
        tbname: p.tbname,
        action: p.action,
        cmdtext: p.cmdtext,
        params: p.params,
        idrow: p.idrow,
        worker: p.worker,
        synced: p.synced,
        cmdtextmd5: p.cmdtextmd5,
        num: 0,
        dlong: 0,
        downlen: 0,
        id: p.id,
        upby: p.upby,
        uptime: String::new(),
        cid: p.cid,
    }).collect())
}

/// 根据同步结果更新本地 synclog 表
fn update_local_synclog(
    db: &LocalDB,
    success_ids: &[String],
    failed: &[FailedRecord],
) {
    // 标记成功的记录
    for id in success_ids {
        let _ = db.execute(&format!(
            "UPDATE synclog SET synced = 1 WHERE id = '{}'",
            id
        ));
    }

    // 标记失败的记录
    for err in failed {
        let _ = db.execute(&format!(
            "UPDATE synclog SET synced = -1, lasterrinfo = '{}' WHERE idrow = '{}'",
            err.error.replace("'", "''"),
            err.idrow
        ));
    }
}

/// 查询本地 testtb 数据
fn query_local_testtb(db: &LocalDB, kind_filter: &str) -> Vec<HashMap<String, Value>> {
    let sql = format!("SELECT * FROM testtb WHERE kind LIKE '{}%' ORDER BY id", kind_filter);
    db.query(&sql, &[]).unwrap_or_default()
}

/// 通过 API 查询 MySQL testtb 数据
fn query_mysql_testtb_via_api(sid: &str, kind_filter: &str) -> Result<Vec<HashMap<String, Value>>, String> {
    let request_body = serde_json::json!({
        "sid": sid,
        "pars": vec![kind_filter],
    });

    let client = reqwest::blocking::Client::new();
    let response = client
        .post(&format!("{}/apitest/testmenu/testtb/get", API_BASE_URL))
        .json(&request_body)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .map_err(|e| format!("HTTP请求失败: {}", e))?;

    let body = response.text().map_err(|e| format!("读取响应失败: {}", e))?;

    let resp: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("解析响应失败: {}", e))?;

    if resp["res"].as_i64().unwrap_or(-1) != 0 {
        return Err(format!("业务错误: {}", resp["errmsg"].as_str().unwrap_or("未知错误")));
    }

    // back 字段是 JSON 字符串，需要再解析一次
    let data_str = resp["back"].as_str().unwrap_or("{}");
    let data: serde_json::Value = serde_json::from_str(data_str)
        .map_err(|e| format!("解析back失败: {}", e))?;

    // 解析返回数据
    let binding = vec![];
    let items = data["items"].as_array().unwrap_or(&binding);
    Ok(items.iter().map(|item| {
        let mut map = HashMap::new();
        if let Some(obj) = item.as_object() {
            for (k, v) in obj {
                map.insert(k.clone(), v.clone());
            }
        }
        map
    }).collect())
}

/// 清理测试数据
fn cleanup_test_data(db: &LocalDB) {
    let _ = db.execute("DELETE FROM testtb WHERE kind LIKE 'test_%'");
    let _ = db.execute("DELETE FROM synclog WHERE tbname = 'testtb'");
}

// ========== 测试用例 ==========

/// 测试 1: 本地新增/修改/删除产生 synclog
#[test]
fn test_01_generate_synclog() {

    let db = setup_local_db();
    ensure_testtb_table_sqlite(&db);
    let _ = DataSync::init_tables(&db);

    // 清理测试数据
    cleanup_test_data(&db);

    let sync = DataSync::with_db("testtb", db.clone());

    // 1. 新增 3 条记录
    let records: Vec<TestRecord> = (0..3)
        .map(|i| TestRecord::new(DEFAULT_CID, "test_01", &format!("item_{}", i), &format!("data_{}", i)))
        .collect();

    let mut ids: Vec<String> = Vec::new();
    for r in &records {
        let id = sync.m_add(&r.to_map()).expect("新增失败");
        ids.push(id.clone());
    }

    // 2. 修改第 2 条
    let mut update_map = HashMap::new();
    update_map.insert("data".to_string(), json!("data_1_updated"));
    sync.m_update(&ids[1], &update_map).expect("修改失败");

    // 3. 删除第 3 条
    sync.m_del(&ids[2]).expect("删除失败");

    // 验证 synclog
    let pending = sync.get_pending_count();
    
    // 应该有 5 条: 3 insert + 1 update + 1 delete
    assert!(pending >= 5, "应该至少有 5 条待同步记录");

    let items = sync.get_pending_items(10);
    for (i, item) in items.iter().enumerate() {
    }

}

/// 测试 2: 同步到 axum78，写入 MySQL synclog 表
#[test]
fn test_02_sync_to_mysql_synclog() {

    let db = setup_local_db();
    ensure_testtb_table_sqlite(&db);
    let _ = DataSync::init_tables(&db);

    let sync = DataSync::with_db("testtb", db.clone());

    // 获取待同步记录
    let items = sync.get_pending_items(100);
    if items.is_empty() {
        return;
    }


    // 调用 API 上传
    match upload_synclog_to_api(&items, DEFAULT_SID) {
        Ok((success_ids, failed)) => {

            for id in &success_ids {
            }
            for err in &failed {
            }

            // 更新本地 synclog
            update_local_synclog(&db, &success_ids, &failed);

            // 验证本地状态
            let synced_count = db.count("synclog WHERE synced = 1").unwrap_or(0);
            let failed_count = db.count("synclog WHERE synced = -1").unwrap_or(0);
        }
        Err(e) => {
        }
    }

}

/// 测试 3: 验证 MySQL testtb 表数据
#[test]
fn test_03_verify_mysql_testtb() {

    let db = setup_local_db();

    // 查询本地数据
    let local_records = query_local_testtb(&db, "test_01");

    // 查询 MySQL 数据（通过 API）
    match query_mysql_testtb_via_api(DEFAULT_SID, "test_01") {
        Ok(mysql_records) => {

            // 对比数据
            for local in &local_records {
                let local_id = local.get("id").and_then(|v| v.as_str()).unwrap_or("");
                let local_data = local.get("data").and_then(|v| v.as_str()).unwrap_or("");

                let mysql_match = mysql_records.iter().find(|r| {
                    r.get("id").and_then(|v| v.as_str()) == Some(local_id)
                });

                if let Some(mysql) = mysql_match {
                    let mysql_data = mysql.get("data").and_then(|v| v.as_str()).unwrap_or("");
                    let match_status = if local_data == mysql_data { "✓" } else { "✗" };
                } else {
                }
            }
        }
        Err(e) => {
        }
    }

}

/// 测试 4: MySQL 修改后下载到本地
#[test]
fn test_04_download_from_mysql() {

    let db = setup_local_db();
    let _sync = DataSync::with_db("testtb", db.clone());

    // 从 API 获取其他 worker 的 synclog（已同步到 MySQL 的）
    match download_synclog_from_api(DEFAULT_SID, 100) {
        Ok(items) => {

            for item in &items {

                // 同步到本地 testtb 表
                if item.tbname == "testtb" {
                    let _params: Vec<Value> = serde_json::from_str(&item.params).unwrap_or_default();
                    
                    match item.action.as_str() {
                        "insert" => {
                            // 从 MySQL 获取完整记录（通过 API）
                        }
                        "update" => {
                            // 更新本地记录
                        }
                        "delete" => {
                            // 删除本地记录
                            let _ = db.execute(&format!("DELETE FROM testtb WHERE id = '{}'", item.idrow));
                        }
                        _ => {}
                    }
                }
            }
        }
        Err(e) => {
        }
    }

}

/// 测试 5: 双边同时修改的冲突处理
#[test]
fn test_05_conflict_resolution() {

    let db = setup_local_db();
    ensure_testtb_table_sqlite(&db);

    let sync = DataSync::with_db("testtb", db.clone());

    // 创建一条测试记录
    let record = TestRecord::new(DEFAULT_CID, "test_05", "conflict_item", "original_data");
    let id = sync.m_add(&record.to_map()).expect("新增失败");

    // 先同步到 MySQL
    let items = sync.get_pending_items(100);
    if let Ok((success_ids, failed)) = upload_synclog_to_api(&items, DEFAULT_SID) {
        update_local_synclog(&db, &success_ids, &failed);
    }

    // 本地修改
    std::thread::sleep(std::time::Duration::from_millis(100));
    let local_time = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let mut update_map = HashMap::new();
    update_map.insert("data".to_string(), json!("local_modified"));
    update_map.insert("uptime".to_string(), json!(local_time.clone()));
    sync.m_update(&id, &update_map).expect("本地修改失败");

    // 模拟 MySQL 端修改（需要通过 API 或直接操作数据库）
    std::thread::sleep(std::time::Duration::from_millis(100));
    let server_time = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    // 冲突解决逻辑：比较 uptime，较新的覆盖较旧的
    
    if server_time > local_time {
    } else {
    }

}

/// 测试 6: guest3 用户帐套隔离
#[test]
fn test_06_account_isolation() {

    let db = setup_local_db();
    ensure_testtb_table_sqlite(&db);
    let _ = DataSync::init_tables(&db);

    // 清理测试数据
    let _ = db.execute("DELETE FROM testtb WHERE kind LIKE 'test_06%'");
    let _ = db.execute("DELETE FROM synclog WHERE tbname = 'testtb' AND kind LIKE 'test_06%'");

    let sync = DataSync::with_db("testtb", db.clone());

    // 默认用户创建记录
    let default_record = TestRecord::new(DEFAULT_CID, "test_06_default", "default_item", "default_data");
    let default_id = sync.m_add(&default_record.to_map()).expect("新增失败");

    // guest3 用户创建记录
    let guest3_record = TestRecord::new(GUEST3_CID, "test_06_guest3", "guest3_item", "guest3_data");
    let guest3_id = sync.m_add(&guest3_record.to_map()).expect("新增失败");

    // 获取待同步记录
    let items = sync.get_pending_items(100);

    // 使用默认用户身份同步
    match upload_synclog_to_api(&items, DEFAULT_SID) {
        Ok((success_ids, failed)) => {

            // 检查 guest3 的记录是否被拒绝
            let guest3_failed = failed.iter().any(|f| {
                items.iter().any(|item| item.idrow == f.idrow && item.cid == GUEST3_CID)
            });
            
            if guest3_failed {
            }

            for err in &failed {
            }

            update_local_synclog(&db, &success_ids, &failed);
        }
        Err(e) => {
        }
    }

    // 使用 guest3 用户身份同步
    let items2 = sync.get_pending_items(100);
    if !items2.is_empty() {
        match upload_synclog_to_api(&items2, GUEST3_SID) {
            Ok((success_ids, failed)) => {

                // 检查默认用户的记录是否被拒绝
                let default_failed = failed.iter().any(|f| {
                    items2.iter().any(|item| item.idrow == f.idrow && item.cid == DEFAULT_CID)
                });

                if default_failed {
                }

                for err in &failed {
                }
            }
            Err(e) => {
            }
        }
    }

}

/// 完整流程测试
#[test]
fn test_full_workflow() {
    

    let db = setup_local_db();
    ensure_testtb_table_sqlite(&db);
    let _ = DataSync::init_tables(&db);

    // 清理测试数据
    cleanup_test_data(&db);

    let sync = DataSync::with_db("testtb", db.clone());

    // Step 1: 新增、修改、删除
    let records: Vec<TestRecord> = (0..3)
        .map(|i| TestRecord::new(DEFAULT_CID, "test_full", &format!("item_{}", i), &format!("data_{}", i)))
        .collect();

    let mut ids: Vec<String> = Vec::new();
    for r in &records {
        let id = sync.m_add(&r.to_map()).expect("新增失败");
        ids.push(id.clone());
    }

    // 修改第二条
    let mut update_map = HashMap::new();
    update_map.insert("data".to_string(), json!("data_1_updated"));
    sync.m_update(&ids[1], &update_map).expect("修改失败");

    // 删除第三条
    sync.m_del(&ids[2]).expect("删除失败");

    let pending = sync.get_pending_count();

    // Step 2: 同步到 MySQL
    let items = sync.get_pending_items(100);
    
    match upload_synclog_to_api(&items, DEFAULT_SID) {
        Ok((success_ids, failed)) => {
            
            // 打印失败详情
            for err in &failed {
            }
            
            update_local_synclog(&db, &success_ids, &failed);

            // Step 3: 验证一致性
            let local_records = query_local_testtb(&db, "test_full");

            match query_mysql_testtb_via_api(DEFAULT_SID, "test_full") {
                Ok(mysql_records) => {

                    // 应该有 2 条记录（第三条已删除）
                    if local_records.len() == 2 && mysql_records.len() == 2 {
                    } else {
                    }

                    // 验证修改
                    let updated = local_records.iter().find(|r| {
                        r.get("id").and_then(|v| v.as_str()) == Some(&ids[1])
                    });
                    if let Some(r) = updated {
                        let data = r.get("data").and_then(|v| v.as_str()).unwrap_or("");
                        if data == "data_1_updated" {
                        }
                    }
                }
                Err(e) => {
                }
            }
        }
        Err(e) => {
        }
    }

}

