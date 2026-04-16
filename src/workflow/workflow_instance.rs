//! WorkflowInstance - 工作流实例表（分表）
//!
//! 参考 koa78-base78 workflow_instance.ts
//! 按天分表，保留7天数据

use crate::{Sqlite78, UpInfo};
use super::{ShardingConfig, ShardType, ShardingManager};
use serde_json;
use serde_json::Value;
use std::collections::HashMap;

/// workflow_instance 表建表 SQL（SQLite 版本）
/// 参考 logsvc/doc/workflow/workflow_instance.sql
/// 按天分表，保留7天数据
pub const SQL_CREATE_WORKFLOW_INSTANCE: &str = r#"
CREATE TABLE IF NOT EXISTS {TABLE_NAME} (
    -- 主键
    idpk INTEGER PRIMARY KEY AUTOINCREMENT,
    id TEXT NOT NULL UNIQUE,
    cid TEXT NOT NULL DEFAULT '',

    -- API 分类
    apisys TEXT NOT NULL DEFAULT 'apiwf',
    apimicro TEXT NOT NULL DEFAULT 'basic',
    apiobj TEXT NOT NULL DEFAULT '',
    myname TEXT NOT NULL DEFAULT '',

    -- 工作流定义
    idworkflowdefinition TEXT NOT NULL DEFAULT '',

    -- 状态（INTEGER：0=待领取, 1=执行中, 2=已完成, 3=失败, 6=警告(完成但有警告)）
    state INTEGER NOT NULL DEFAULT 0,
    priority INTEGER NOT NULL DEFAULT 5,
    agentkind TEXT NOT NULL DEFAULT '',

    -- 执行配置
    flowschema TEXT NOT NULL DEFAULT '{}',
    inputjson TEXT NOT NULL DEFAULT '{}',
    outputjson TEXT NOT NULL DEFAULT '{}',
    maxcopy INTEGER NOT NULL DEFAULT 1,
    currentcopy INTEGER NOT NULL DEFAULT 0,
    timeout INTEGER NOT NULL DEFAULT 3600,
    retrylimit INTEGER NOT NULL DEFAULT 3,
    retryinterval INTEGER NOT NULL DEFAULT 60,

    -- 资源和配置
    resourcereq TEXT NOT NULL DEFAULT '{}',
    description TEXT NOT NULL DEFAULT '{}',
    configjson TEXT NOT NULL DEFAULT '{}',

    -- 财务统计
    costtotal REAL NOT NULL DEFAULT 0.0,
    revenuetotal REAL NOT NULL DEFAULT 0.0,
    profittotal REAL NOT NULL DEFAULT 0.0,
    roi REAL NOT NULL DEFAULT 0.0,

    -- 执行统计
    runcount INTEGER NOT NULL DEFAULT 0,
    successcount INTEGER NOT NULL DEFAULT 0,
    errorcount INTEGER NOT NULL DEFAULT 0,
    successrate REAL NOT NULL DEFAULT 0.0,
    executiontime REAL NOT NULL DEFAULT 0.0,

    -- 时间信息
    lastruntime TEXT NOT NULL DEFAULT '',
    lastoktime TEXT NOT NULL DEFAULT '',
    lasterrortime TEXT NOT NULL DEFAULT '',
    lasterrinfo TEXT NOT NULL DEFAULT '{}',
    lastokinfo TEXT NOT NULL DEFAULT '{}',
    starttime TEXT NOT NULL DEFAULT '',
    endtime TEXT NOT NULL DEFAULT '',

    -- 关联信息
    idagent TEXT NOT NULL DEFAULT '',
    idparentinstance TEXT NOT NULL DEFAULT '',

    -- 系统字段
    upby TEXT NOT NULL DEFAULT '',
    uptime TEXT NOT NULL DEFAULT '',
    remark TEXT NOT NULL DEFAULT '',
    remark2 TEXT NOT NULL DEFAULT '',
    remark3 TEXT NOT NULL DEFAULT '',
    remark4 TEXT NOT NULL DEFAULT '',
    remark5 TEXT NOT NULL DEFAULT '',
    remark6 TEXT NOT NULL DEFAULT '',

    -- 同步字段
    created_at REAL NOT NULL DEFAULT 0,
    updated_at REAL NOT NULL DEFAULT 0,
    deleted INTEGER NOT NULL DEFAULT 0
)
"#;

/// 创建索引 SQL
pub const SQL_CREATE_WORKFLOW_INSTANCE_INDEX: &str = r#"
CREATE INDEX IF NOT EXISTS idx_{TABLE_NAME}_kind_state ON {TABLE_NAME} (agentkind, state)
"#;

/// WorkflowInstance - 工作流实例管理
pub struct WorkflowInstance {
    db: Sqlite78,
    sharding_manager: Option<ShardingManager>,
}

impl WorkflowInstance {
    /// 创建新实例（不分表）
    pub fn new(db: Sqlite78) -> Self {
        Self {
            db,
            sharding_manager: None,
        }
    }

    /// 创建分表实例
    pub fn with_sharding(mut db: Sqlite78) -> Result<Self, String> {
        db.initialize()?;

        let config = ShardingConfig::new(ShardType::Daily, "workflow_instance")
            .with_table_sql(SQL_CREATE_WORKFLOW_INSTANCE)
            .with_retention(7);  // 保留7天

        let conn = db.get_conn()?;
        let sharding_manager = ShardingManager::new(conn, config);

        Ok(Self {
            db,
            sharding_manager: Some(sharding_manager),
        })
    }

    /// 使用默认数据库路径创建分表实例
    pub fn with_default_path() -> Result<Self, String> {
        let db = Sqlite78::with_default_path();
        Self::with_sharding(db)
    }

    /// 使用指定路径创建分表实例
    pub fn with_path(path: &str) -> Result<Self, String> {
        let db = Sqlite78::with_config(path, false, false);
        Self::with_sharding(db)
    }

    /// 获取当前表名
    pub fn get_table_name(&self) -> String {
        if let Some(ref _manager) = self.sharding_manager {
            let config = ShardingConfig::new(ShardType::Daily, "workflow_instance");
            config.get_current_table_name()
        } else {
            "workflow_instance".to_string()
        }
    }

    /// 执行分表维护
    pub fn perform_maintenance(&mut self) -> Result<super::MaintenanceResult, String> {
        if let Some(ref mut manager) = self.sharding_manager {
            return manager.perform_maintenance();
        }
        Ok(super::MaintenanceResult::default())
    }

    /// 创建今天的分表
    pub fn create_today_table(&self) -> Result<(), String> {
        if let Some(ref manager) = self.sharding_manager {
            let table_name = self.get_table_name();
            if !manager.table_exists(&table_name)? {
                manager.create_sharding_table(&table_name)?;

                // 创建索引
                let conn = self.db.get_conn()?;
                let conn = conn.lock().map_err(|e| e.to_string())?;
                let index_sql = SQL_CREATE_WORKFLOW_INSTANCE_INDEX.replace("{TABLE_NAME}", &table_name);
                conn.execute(&index_sql, [])
                    .map_err(|e| format!("创建索引失败: {}", e))?;
            }
        }
        Ok(())
    }

    /// 插入工作流实例
    pub fn insert(&self, data: &HashMap<String, Value>, up: &UpInfo) -> Result<String, String> {
        let table_name = self.get_table_name();
        self.create_today_table()?;

        let id = data
            .get("id")
            .and_then(|v: &serde_json::Value| v.as_str())
            .unwrap_or(&UpInfo::new_id())
            .to_string();
        let cid = data.get("cid").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("");
        let apisys = data
            .get("apisys")
            .and_then(|v: &serde_json::Value| v.as_str())
            .unwrap_or("apiwf");
        let apimicro = data
            .get("apimicro")
            .and_then(|v: &serde_json::Value| v.as_str())
            .unwrap_or("basic");
        let apiobj = data.get("apiobj").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("");
        let myname = data.get("myname").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("");
        let idworkflowdefinition = data
            .get("idworkflowdefinition")
            .and_then(|v: &serde_json::Value| v.as_str())
            .unwrap_or("");
        let state = data.get("state").and_then(|v: &serde_json::Value| v.as_i64()).unwrap_or(0) as i32;
        let priority = data.get("priority").and_then(|v: &serde_json::Value| v.as_i64()).unwrap_or(5) as i32;
        let agentkind = data.get("agentkind").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("");
        let flowschema = data
            .get("flowschema")
            .and_then(|v: &serde_json::Value| v.as_str())
            .unwrap_or("{}");
        let inputjson = data
            .get("inputjson")
            .and_then(|v: &serde_json::Value| v.as_str())
            .unwrap_or("{}");
        let outputjson = data
            .get("outputjson")
            .and_then(|v: &serde_json::Value| v.as_str())
            .unwrap_or("{}");
        let maxcopy = data.get("maxcopy").and_then(|v: &serde_json::Value| v.as_i64()).unwrap_or(1) as i32;
        let currentcopy = data.get("currentcopy").and_then(|v: &serde_json::Value| v.as_i64()).unwrap_or(0) as i32;
        let timeout = data.get("timeout").and_then(|v: &serde_json::Value| v.as_i64()).unwrap_or(3600) as i32;
        let retrylimit = data.get("retrylimit").and_then(|v: &serde_json::Value| v.as_i64()).unwrap_or(3) as i32;
        let retryinterval = data.get("retryinterval").and_then(|v: &serde_json::Value| v.as_i64()).unwrap_or(60) as i32;
        let resourcereq = data
            .get("resourcereq")
            .and_then(|v: &serde_json::Value| v.as_str())
            .unwrap_or("{}");
        let description = data
            .get("description")
            .and_then(|v: &serde_json::Value| v.as_str())
            .unwrap_or("{}");
        let configjson = data
            .get("configjson")
            .and_then(|v: &serde_json::Value| v.as_str())
            .unwrap_or("{}");
        let costtotal = data.get("costtotal").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let revenuetotal = data.get("revenuetotal").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let profittotal = data.get("profittotal").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let roi = data.get("roi").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let runcount = data.get("runcount").and_then(|v: &serde_json::Value| v.as_i64()).unwrap_or(0) as i32;
        let successcount = data.get("successcount").and_then(|v: &serde_json::Value| v.as_i64()).unwrap_or(0) as i32;
        let errorcount = data.get("errorcount").and_then(|v: &serde_json::Value| v.as_i64()).unwrap_or(0) as i32;
        let successrate = data.get("successrate").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let executiontime = data.get("executiontime").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let lastruntime = data
            .get("lastruntime")
            .and_then(|v: &serde_json::Value| v.as_str())
            .unwrap_or("");
        let lastoktime = data
            .get("lastoktime")
            .and_then(|v: &serde_json::Value| v.as_str())
            .unwrap_or("");
        let lasterrortime = data
            .get("lasterrortime")
            .and_then(|v: &serde_json::Value| v.as_str())
            .unwrap_or("");
        let lasterrinfo = data
            .get("lasterrinfo")
            .and_then(|v: &serde_json::Value| v.as_str())
            .unwrap_or("{}");
        let lastokinfo = data
            .get("lastokinfo")
            .and_then(|v: &serde_json::Value| v.as_str())
            .unwrap_or("{}");
        let starttime = data.get("starttime").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("");
        let endtime = data.get("endtime").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("");
        let idagent = data.get("idagent").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("");
        let idparentinstance = data
            .get("idparentinstance")
            .and_then(|v: &serde_json::Value| v.as_str())
            .unwrap_or("");

        let sql = format!(
            "REPLACE INTO {} (id, cid, apisys, apimicro, apiobj, myname, idworkflowdefinition, state, priority, agentkind, flowschema, inputjson, outputjson, maxcopy, currentcopy, timeout, retrylimit, retryinterval, resourcereq, description, configjson, costtotal, revenuetotal, profittotal, roi, runcount, successcount, errorcount, successrate, executiontime, lastruntime, lastoktime, lasterrortime, lasterrinfo, lastokinfo, starttime, endtime, idagent, idparentinstance) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            table_name
        );

        self.db.do_m_add(
            &sql,
            &[
                &id as &dyn rusqlite::ToSql,
                &cid as &dyn rusqlite::ToSql,
                &apisys as &dyn rusqlite::ToSql,
                &apimicro as &dyn rusqlite::ToSql,
                &apiobj as &dyn rusqlite::ToSql,
                &myname as &dyn rusqlite::ToSql,
                &idworkflowdefinition as &dyn rusqlite::ToSql,
                &state as &dyn rusqlite::ToSql,
                &priority as &dyn rusqlite::ToSql,
                &agentkind as &dyn rusqlite::ToSql,
                &flowschema as &dyn rusqlite::ToSql,
                &inputjson as &dyn rusqlite::ToSql,
                &outputjson as &dyn rusqlite::ToSql,
                &maxcopy as &dyn rusqlite::ToSql,
                &currentcopy as &dyn rusqlite::ToSql,
                &timeout as &dyn rusqlite::ToSql,
                &retrylimit as &dyn rusqlite::ToSql,
                &retryinterval as &dyn rusqlite::ToSql,
                &resourcereq as &dyn rusqlite::ToSql,
                &description as &dyn rusqlite::ToSql,
                &configjson as &dyn rusqlite::ToSql,
                &costtotal as &dyn rusqlite::ToSql,
                &revenuetotal as &dyn rusqlite::ToSql,
                &profittotal as &dyn rusqlite::ToSql,
                &roi as &dyn rusqlite::ToSql,
                &runcount as &dyn rusqlite::ToSql,
                &successcount as &dyn rusqlite::ToSql,
                &errorcount as &dyn rusqlite::ToSql,
                &successrate as &dyn rusqlite::ToSql,
                &executiontime as &dyn rusqlite::ToSql,
                &lastruntime as &dyn rusqlite::ToSql,
                &lastoktime as &dyn rusqlite::ToSql,
                &lasterrortime as &dyn rusqlite::ToSql,
                &lasterrinfo as &dyn rusqlite::ToSql,
                &lastokinfo as &dyn rusqlite::ToSql,
                &starttime as &dyn rusqlite::ToSql,
                &endtime as &dyn rusqlite::ToSql,
                &idagent as &dyn rusqlite::ToSql,
                &idparentinstance as &dyn rusqlite::ToSql,
            ],
            up,
        )?;

        Ok(id)
    }

    /// 根据 ID 查询工作流实例
    pub fn get(&self, id: &str, up: &UpInfo) -> Result<Option<HashMap<String, Value>>, String> {
        self.create_today_table()?;
        let table_name = self.get_table_name();
        let sql = format!("SELECT * FROM {} WHERE id = ?", table_name);
        let rows = self.db.do_get(&sql, &[&id as &dyn rusqlite::ToSql], up)?;
        Ok(rows.into_iter().next())
    }

    /// 更新工作流状态
    pub fn update_state(&self, id: &str, state: i32, up: &UpInfo) -> Result<(), String> {
        let table_name = self.get_table_name();
        let sql = format!("UPDATE {} SET state = ? WHERE id = ?", table_name);
        self.db.do_m(&sql, &[&state as &dyn rusqlite::ToSql, &id as &dyn rusqlite::ToSql], up)?;
        Ok(())
    }

    /// 标记工作流完成（state=2，记录成功信息）
    pub fn mark_completed(&self, id: &str, info: &str, up: &UpInfo) -> Result<(), String> {
        let table_name = self.get_table_name();
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let sql = format!(
            "UPDATE {} SET state = 2, lastoktime = ?, lastokinfo = ?, endtime = ? WHERE id = ?",
            table_name
        );
        self.db.do_m(
            &sql,
            &[&now as &dyn rusqlite::ToSql, &info as &dyn rusqlite::ToSql, &now as &dyn rusqlite::ToSql, &id as &dyn rusqlite::ToSql],
            up
        )?;
        Ok(())
    }

    /// 标记工作流失败（state=3，记录错误信息）
    pub fn mark_failed(&self, id: &str, errinfo: &str, up: &UpInfo) -> Result<(), String> {
        let table_name = self.get_table_name();
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let sql = format!(
            "UPDATE {} SET state = 3, lasterrortime = ?, lasterrinfo = ?, endtime = ? WHERE id = ?",
            table_name
        );
        self.db.do_m(
            &sql,
            &[&now as &dyn rusqlite::ToSql, &errinfo as &dyn rusqlite::ToSql, &now as &dyn rusqlite::ToSql, &id as &dyn rusqlite::ToSql],
            up
        )?;
        Ok(())
    }

    /// 查询运行中的工作流（state = 1）
    pub fn get_running(&self, up: &UpInfo) -> Result<Vec<HashMap<String, Value>>, String> {
        let table_name = self.get_table_name();
        let sql = format!("SELECT * FROM {} WHERE state = 1 ORDER BY priority DESC", table_name);
        self.db.do_get(&sql, &[], up)
    }

    /// 查询指定工作流定义的所有实例
    pub fn get_by_workflow(&self, idworkflowdefinition: &str, up: &UpInfo) -> Result<Vec<HashMap<String, Value>>, String> {
        let table_name = self.get_table_name();
        let sql = format!(
            "SELECT * FROM {} WHERE idworkflowdefinition = ? ORDER BY starttime DESC",
            table_name
        );
        self.db.do_get(&sql, &[&idworkflowdefinition as &dyn rusqlite::ToSql], up)
    }

    /// 获取底层数据库引用
    pub fn get_db(&self) -> &Sqlite78 {
        &self.db
    }

    /// 根据 ID 查询记录
    pub fn get_by_id(&self, id: &str, up: &UpInfo) -> Result<Option<HashMap<String, Value>>, String> {
        let table_name = self.get_table_name();
        let sql = format!("SELECT * FROM {} WHERE id = ?", table_name);
        let rows = self.db.do_get(&sql, &[&id as &dyn rusqlite::ToSql], up)?;
        Ok(rows.into_iter().next())
    }

    /// 更新记录（完整更新）
    pub fn update(&self, data: &HashMap<String, Value>, up: &UpInfo) -> Result<(), String> {
        let id = data.get("id")
            .and_then(|v: &serde_json::Value| v.as_str())
            .ok_or_else(|| "缺少 id 字段".to_string())?;

        let table_name = self.get_table_name();

        // 构建 SET 子句（排除 id）
        let mut set_clauses = Vec::new();
        let mut params: Vec<&dyn rusqlite::ToSql> = Vec::new();

        // 所有字段列表（与 INSERT 对应）
        let fields = [
            "cid", "apisys", "apimicro", "apiobj", "myname",
            "idworkflowdefinition", "state", "priority", "agentkind", "flowschema",
            "inputjson", "outputjson", "maxcopy", "currentcopy", "timeout",
            "retrylimit", "retryinterval", "resourcereq", "description", "configjson",
            "costtotal", "revenuetotal", "profittotal", "roi", "runcount",
            "successcount", "errorcount", "successrate", "executiontime", "lastruntime",
            "lastoktime", "lasterrortime", "lastokinfo", "lasterrinfo", "starttime",
            "endtime", "idagent", "idparentinstance"
        ];

        // 存储字符串值以避免生命周期问题
        let mut string_storage = Vec::new();

        for field in fields.iter() {
            if let Some(value) = data.get(*field) {
                set_clauses.push(format!("{} = ?", field));
                // 将 Value 转换为 String（所有字段都是 TEXT 类型）
                let value_str = match value {
                    Value::String(s) => s.clone(),
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    Value::Array(arr) => serde_json::to_string(arr).unwrap_or_default(),
                    Value::Object(obj) => serde_json::to_string(obj).unwrap_or_default(),
                    Value::Null => "".to_string(),
                };
                string_storage.push(value_str);
            }
        }

        // 现在添加所有存储的值到 params
        for value_str in &string_storage {
            params.push(value_str as &dyn rusqlite::ToSql);
        }

        if set_clauses.is_empty() {
            return Ok(());
        }

        let sql = format!("UPDATE {} SET {} WHERE id = ?", table_name, set_clauses.join(", "));
        params.push(&id as &dyn rusqlite::ToSql);

        self.db.do_m(&sql, &params, up)?;
        Ok(())
    }

    /// 查询记录列表
    pub fn query_list(&self, condition: &str, params: &[&dyn rusqlite::ToSql], up: &UpInfo) -> Result<Vec<HashMap<String, Value>>, String> {
        let table_name = self.get_table_name();
        let sql = if condition.is_empty() {
            format!("SELECT * FROM {}", table_name)
        } else {
            format!("SELECT * FROM {} WHERE {}", table_name, condition)
        };
        self.db.do_get(&sql, params, up)
    }

    /// 分页查询
    pub fn query_page(&self, condition: &str, params: &[&dyn rusqlite::ToSql], offset: i32, limit: i32, up: &UpInfo) -> Result<(Vec<HashMap<String, Value>>, i32), String> {
        let table_name = self.get_table_name();
        let sql = if condition.is_empty() {
            format!("SELECT * FROM {} LIMIT ? OFFSET ?", table_name)
        } else {
            format!("SELECT * FROM {} WHERE {} LIMIT ? OFFSET ?", table_name, condition)
        };

        // 准备参数：先传查询参数，再传 limit 和 offset
        let mut all_params: Vec<&dyn rusqlite::ToSql> = params.to_vec();
        all_params.push(&limit as &dyn rusqlite::ToSql);
        all_params.push(&offset as &dyn rusqlite::ToSql);

        let records = self.db.do_get(&sql, &all_params, up)?;

        // 查询总数
        let count_sql = if condition.is_empty() {
            format!("SELECT COUNT(*) as cnt FROM {}", table_name)
        } else {
            format!("SELECT COUNT(*) as cnt FROM {} WHERE {}", table_name, condition)
        };
        let count_rows = self.db.do_get(&count_sql, params, up)?;
        let total = count_rows.first()
            .and_then(|row| row.get("cnt").and_then(|v: &serde_json::Value| v.as_i64()))
            .unwrap_or(0) as i32;

        Ok((records, total))
    }

    /// 删除记录
    pub fn delete(&self, id: &str, up: &UpInfo) -> Result<(), String> {
        let table_name = self.get_table_name();
        let sql = format!("DELETE FROM {} WHERE id = ?", table_name);
        self.db.do_m(&sql, &[&id as &dyn rusqlite::ToSql], up)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_instance_basic() {

        let instance = WorkflowInstance::with_default_path()
            .expect("创建失败");

        let table_name = instance.get_table_name();
        assert!(table_name.starts_with("workflow_instance_"), "表名应该是分表格式");

        let up = UpInfo::new();
        let mut data = HashMap::new();
        data.insert("id".to_string(), Value::String("test-instance-001".to_string()));
        data.insert("myname".to_string(), Value::String("测试实例".to_string()));
        data.insert("apisys".to_string(), Value::String("test".to_string()));
        data.insert("apimicro".to_string(), Value::String("workflow".to_string()));
        data.insert("apiobj".to_string(), Value::String("instance".to_string()));
        data.insert("state".to_string(), Value::Number(1.into())); // 1=执行中
        data.insert("priority".to_string(), Value::Number(5.into()));

        let unique_id = "test-instance-001".to_string();
        let result = instance.insert(&data, &up);
        assert!(result.is_ok(), "插入应该成功: {:?}", result);

        // 查询插入的记录
        let found = instance.get_by_id(&unique_id, &up);
        assert!(found.is_ok(), "查询应该成功: {:?}", found);
        let record = found.unwrap();
        assert!(record.is_some(), "记录应该存在");
    }

    #[test]
    fn test_workflow_instance_query() {

        let instance = WorkflowInstance::with_default_path()
            .expect("创建失败");

        // 插入多条测试数据
        let up = UpInfo::new();
        for i in 1..=3 {
            let mut data = HashMap::new();
            data.insert("id".to_string(), Value::String(format!("test-query-{}", i)));
            data.insert("myname".to_string(), Value::String(format!("查询测试{}", i)));
            data.insert("apisys".to_string(), Value::String("test".to_string()));
            data.insert("apimicro".to_string(), Value::String("workflow".to_string()));
            data.insert("apiobj".to_string(), Value::String("query".to_string()));
            data.insert("state".to_string(), Value::Number(2.into())); // 已完成
            data.insert("priority".to_string(), Value::Number(i.into()));

            let result = instance.insert(&data, &up);
            assert!(result.is_ok(), "插入{}应该成功", i);
        }

        // 查询所有state=2的记录
        let query_result = instance.query_list("state = ?", &[&2], &up);
        assert!(query_result.is_ok(), "查询应该成功");
        let records = query_result.unwrap();
        assert!(records.len() >= 3, "应该至少查询到3条记录");
    }

    #[test]
    fn test_workflow_instance_pagination() {

        let instance = WorkflowInstance::with_default_path()
            .expect("创建失败");

        // 插入测试数据
        let up = UpInfo::new();
        for i in 1..=10 {
            let mut data = HashMap::new();
            data.insert("id".to_string(), Value::String(format!("test-page-{}", i)));
            data.insert("myname".to_string(), Value::String(format!("分页测试{}", i)));
            data.insert("apisys".to_string(), Value::String("test".to_string()));
            data.insert("apimicro".to_string(), Value::String("workflow".to_string()));
            data.insert("apiobj".to_string(), Value::String("page".to_string()));
            data.insert("state".to_string(), Value::Number(2.into()));
            data.insert("priority".to_string(), Value::Number(i.into()));

            let result = instance.insert(&data, &up);
            assert!(result.is_ok(), "插入{}应该成功", i);
        }

        // 测试分页查询
        let page_result = instance.query_page("state = ?", &[&2], 0, 5, &up);
        assert!(page_result.is_ok(), "分页查询应该成功");
        let (records, total) = page_result.unwrap();
        assert_eq!(records.len(), 5, "第一页应该返回5条记录");
        assert!(total >= 10, "总数应该大于等于10");
    }

    #[test]
    fn test_workflow_instance_delete() {

        let instance = WorkflowInstance::with_default_path()
            .expect("创建失败");

        // 插入一条测试数据
        let up = UpInfo::new();
        let mut data = HashMap::new();
        data.insert("id".to_string(), Value::String("test-delete".to_string()));
        data.insert("myname".to_string(), Value::String("删除测试".to_string()));
        data.insert("apisys".to_string(), Value::String("test".to_string()));
        data.insert("apimicro".to_string(), Value::String("workflow".to_string()));
        data.insert("apiobj".to_string(), Value::String("delete".to_string()));
        data.insert("state".to_string(), Value::Number(1.into()));

        let result = instance.insert(&data, &up);
        assert!(result.is_ok(), "插入应该成功");

        // 删除记录
        let delete_result = instance.delete(&"test-delete".to_string(), &up);
        assert!(delete_result.is_ok(), "删除应该成功");

        // 验证删除成功
        let found = instance.get_by_id("test-delete", &up);
        assert!(found.is_ok(), "查询应该成功");
        assert!(found.unwrap().is_none(), "记录应该已被删除");
    }
}
