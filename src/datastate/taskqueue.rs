//! 任务队列表
//!
//! 用于存储待执行的任务
//! state: 0=待执行, 1=执行中, 2=已完成, 3=失败, 4=已取消

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_json::json;

use crate::dataaudit::DataAudit;
use crate::datastate::DataState;
use crate::localdb::LocalDB;
use crate::snowflake;

/// taskqueue 表建表 SQL (SQLite版本)
pub const TASKQUEUE_CREATE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS taskqueue (
    id TEXT NOT NULL PRIMARY KEY,
    cid TEXT NOT NULL DEFAULT '',
    apisys TEXT NOT NULL DEFAULT '',
    apimicro TEXT NOT NULL DEFAULT '',
    apiobj TEXT NOT NULL DEFAULT '',
    inputjson TEXT NOT NULL DEFAULT '{}',
    outputjson TEXT NOT NULL DEFAULT '{}',
    worker TEXT NOT NULL DEFAULT '',
    price REAL NOT NULL DEFAULT 1.0,
    state INTEGER NOT NULL DEFAULT 0,
    retrylimit INTEGER NOT NULL DEFAULT 3,
    retryinterval INTEGER NOT NULL DEFAULT 60,
    retrytimes INTEGER NOT NULL DEFAULT 0,
    starttime TEXT NOT NULL DEFAULT '',
    lastruntime TEXT NOT NULL DEFAULT '',
    lasterrortime TEXT NOT NULL DEFAULT '',
    lastoktime TEXT NOT NULL DEFAULT '',
    lasterrinfo TEXT NOT NULL DEFAULT '{}',
    lastokinfo TEXT NOT NULL DEFAULT '{}',
    endtime TEXT NOT NULL DEFAULT '',
    uptime TEXT NOT NULL DEFAULT '',
    upby TEXT NOT NULL DEFAULT ''
)
"#;

/// TaskQueue 记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskQueueRecord {
    pub id: String,
    pub cid: String,
    pub apisys: String,
    pub apimicro: String,
    pub apiobj: String,
    pub inputjson: String,
    pub outputjson: String,
    pub worker: String,
    pub price: f64,
    pub state: i32,
    pub retrylimit: i32,
    pub retryinterval: i32,
    pub retrytimes: i32,
    pub starttime: String,
    pub lastruntime: String,
    pub lasterrortime: String,
    pub lastoktime: String,
    pub lasterrinfo: String,
    pub lastokinfo: String,
    pub endtime: String,
    pub uptime: String,
    pub upby: String,
}

/// TaskQueue - 任务队列表 DataState
pub struct TaskQueue {
    pub db: LocalDB,
    pub audit: DataAudit,
    pub state: DataState,
}

impl TaskQueue {
    pub fn new() -> Self {
        let db = LocalDB::new(None).expect("创建数据库失败");
        let audit = DataAudit::new("taskqueue");
        db.execute(TASKQUEUE_CREATE_SQL).expect("建表失败");
        if let Ok(false) = db.is_id_primary_key("taskqueue") {
            let _ = db.ensure_id_is_primary_key("taskqueue");
        }
        let state = DataState::with_db("taskqueue", db.clone());
        Self { db, audit, state }
    }
    
    pub fn with_db_path(db_path: &str) -> Self {
        let db = LocalDB::with_path(db_path).expect("创建数据库失败");
        let audit = DataAudit::new("taskqueue");
        db.execute(TASKQUEUE_CREATE_SQL).expect("建表失败");
        if let Ok(false) = db.is_id_primary_key("taskqueue") {
            let _ = db.ensure_id_is_primary_key("taskqueue");
        }
        let state = DataState::with_db("taskqueue", db.clone());
        Self { db, audit, state }
    }
    
    fn check_caller(&self, operation: &str, caller: &str) -> Result<(), String> {
        let allowed = match caller {
            "taskqueue" => true,
            "marketing" => true,
            "zhihu" => true,
            "xiaohongshu" => true,
            _ => false,
        };
        if !allowed {
            return Err(format!("{} 无权调用 {}", caller, operation));
        }
        Ok(())
    }
    
    pub fn m_save(&self, record: &mut TaskQueueRecord, caller: &str, summary: &str) -> Result<String, String> {
        self.check_caller("m_save", caller)?;
        let now = chrono::Utc::now().to_rfc3339();
        let id = snowflake::next_id_string();
        record.id = id.clone();
        record.uptime = now;
        let record_map = self.record_to_map(record);
        self.state.m_save(&record_map, caller, summary)?;
        Ok(id)
    }
    
    pub fn m_update(&self, id: &str, record: &mut TaskQueueRecord, caller: &str, summary: &str) -> Result<bool, String> {
        self.check_caller("m_update", caller)?;
        let now = chrono::Utc::now().to_rfc3339();
        record.uptime = now;
        let record_map = self.record_to_map(record);
        self.state.m_update(id, &record_map, caller, summary)
    }
    
    pub fn m_del(&self, id: &str, caller: &str, summary: &str) -> Result<bool, String> {
        self.check_caller("m_del", caller)?;
        self.state.m_del(id, caller, summary)
    }
    
    pub fn getone(&self, id: &str, caller: &str) -> Result<Option<TaskQueueRecord>, String> {
        self.check_caller("getone", caller)?;
        let sql = format!("SELECT * FROM taskqueue WHERE id = '{}}'", id);
        let rows = self.db.query(&sql)?;
        Ok(rows.into_iter().next().and_then(|r| self.map_to_record(r)))
    }
    
    pub fn mlist(&self, caller: &str, limit: i32) -> Result<Vec<TaskQueueRecord>, String> {
        self.check_caller("mlist", caller)?;
        let sql = format!("SELECT * FROM taskqueue LIMIT {}", limit);
        let rows = self.db.query(&sql)?;
        Ok(rows.into_iter().filter_map(|r| self.map_to_record(r)).collect())
    }
    
    pub fn get_pending_tasks(&self, limit: i32) -> Result<Vec<TaskQueueRecord>, String> {
        let sql = format!(
            "SELECT * FROM taskqueue WHERE state = 0 ORDER BY price DESC LIMIT {}",
            limit
        );
        let rows = self.db.query(&sql)?;
        Ok(rows.into_iter().filter_map(|r| self.map_to_record(r)).collect())
    }
    
    pub fn update_state(&self, id: &str, new_state: i32, caller: &str) -> Result<bool, String> {
        self.check_caller("update_state", caller)?;
        let now = chrono::Utc::now().to_rfc3339();
        let sql = format!(
            "UPDATE taskqueue SET state = {}, uptime = '{}' WHERE id = '{}'",
            new_state, now, id
        );
        self.db.execute(&sql)
    }
    
    fn record_to_map(&self, record: &TaskQueueRecord) -> HashMap<String, Value> {
        let mut map = HashMap::new();
        map.insert("id".to_string(), json!(record.id));
        map.insert("cid".to_string(), json!(record.cid));
        map.insert("apisys".to_string(), json!(record.apisys));
        map.insert("apimicro".to_string(), json!(record.apimicro));
        map.insert("apiobj".to_string(), json!(record.apiobj));
        map.insert("inputjson".to_string(), json!(record.inputjson));
        map.insert("outputjson".to_string(), json!(record.outputjson));
        map.insert("worker".to_string(), json!(record.worker));
        map.insert("price".to_string(), json!(record.price));
        map.insert("state".to_string(), json!(record.state));
        map.insert("retrylimit".to_string(), json!(record.retrylimit));
        map.insert("retryinterval".to_string(), json!(record.retryinterval));
        map.insert("retrytimes".to_string(), json!(record.retrytimes));
        map.insert("starttime".to_string(), json!(record.starttime));
        map.insert("lastruntime".to_string(), json!(record.lastruntime));
        map.insert("lasterrortime".to_string(), json!(record.lasterrortime));
        map.insert("lastoktime".to_string(), json!(record.lastoktime));
        map.insert("lasterrinfo".to_string(), json!(record.lasterrinfo));
        map.insert("lastokinfo".to_string(), json!(record.lastokinfo));
        map.insert("endtime".to_string(), json!(record.endtime));
        map.insert("uptime".to_string(), json!(record.uptime));
        map.insert("upby".to_string(), json!(record.upby));
        map
    }
    
    fn map_to_record(&self, map: HashMap<String, Value>) -> Option<TaskQueueRecord> {
        Some(TaskQueueRecord {
            id: map.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            cid: map.get("cid").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            apisys: map.get("apisys").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            apimicro: map.get("apimicro").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            apiobj: map.get("apiobj").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            inputjson: map.get("inputjson").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            outputjson: map.get("outputjson").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            worker: map.get("worker").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            price: map.get("price").and_then(|v| v.as_f64()).unwrap_or_default(),
            state: map.get("state").and_then(|v| v.as_i64()).unwrap_or_default() as i32,
            retrylimit: map.get("retrylimit").and_then(|v| v.as_i64()).unwrap_or_default() as i32,
            retryinterval: map.get("retryinterval").and_then(|v| v.as_i64()).unwrap_or_default() as i32,
            retrytimes: map.get("retrytimes").and_then(|v| v.as_i64()).unwrap_or_default() as i32,
            starttime: map.get("starttime").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            lastruntime: map.get("lastruntime").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            lasterrortime: map.get("lasterrortime").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            lastoktime: map.get("lastoktime").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            lasterrinfo: map.get("lasterrinfo").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            lastokinfo: map.get("lastokinfo").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            endtime: map.get("endtime").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            uptime: map.get("uptime").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            upby: map.get("upby").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
        })
    }
}

impl Default for TaskQueue {
    fn default() -> Self {
        Self::new()
    }
}
