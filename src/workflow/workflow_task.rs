//! WorkflowTask - 任务实例表管理（分表）
//!
//! ## 表关系说明
//! - workflow_capability: 能力【定义表】- 存储能力模板、配置、价格等静态定义
//! - workflow_task: 任务【实例表】- 每次能力执行产生的任务实例记录
//!
//! 对应 workflow_task 表
//! 按天分表，保留7天数据

use super::{ShardType, ShardingConfig, ShardingManager};
use crate::{Sqlite78, UpInfo};
use serde_json::Value;
use std::collections::HashMap;

/// workflow_task 表建表 SQL
/// 参考 logsvc/doc/workflow/workflow_task.sql
/// 按天分表，保留7天数据
pub const SQL_CREATE_WORKFLOW_TASK: &str = r#"
CREATE TABLE IF NOT EXISTS {TABLE_NAME} (
    -- 主键
    idpk INTEGER PRIMARY KEY AUTOINCREMENT,
    id TEXT NOT NULL UNIQUE,
    cid TEXT NOT NULL DEFAULT '',

    -- 任务名称
    myname TEXT NOT NULL DEFAULT '',
    idcapability TEXT NOT NULL DEFAULT '',

    -- API 分类
    apisys TEXT NOT NULL DEFAULT 'apiwf',
    apimicro TEXT NOT NULL DEFAULT 'basic',
    apiobj TEXT NOT NULL DEFAULT '',

    -- 配置
    configjson TEXT NOT NULL DEFAULT '{}',
    description TEXT NOT NULL DEFAULT '{}',
    priority INTEGER NOT NULL DEFAULT 5,

    -- 时间
    starttime TEXT NOT NULL DEFAULT '',

    -- 执行控制
    maxcopy INTEGER NOT NULL DEFAULT 1,
    currentcopy INTEGER NOT NULL DEFAULT 0,
    timeout INTEGER NOT NULL DEFAULT 600,
    retrylimit INTEGER NOT NULL DEFAULT 3,
    retryinterval INTEGER NOT NULL DEFAULT 60,
    progress INTEGER NOT NULL DEFAULT 0,
    retrytimes INTEGER NOT NULL DEFAULT 0,
    dependencies TEXT NOT NULL DEFAULT '[]',

    -- 成本
    costdescription TEXT NOT NULL DEFAULT '{}',
    pricebase REAL NOT NULL DEFAULT 1.0,
    price REAL NOT NULL DEFAULT 1.0,
    costunit REAL NOT NULL DEFAULT 0.0,
    profittarget REAL NOT NULL DEFAULT 0.2,
    profittotal REAL NOT NULL DEFAULT 0.0,
    costtotal REAL NOT NULL DEFAULT 0.0,
    revenuetotal REAL NOT NULL DEFAULT 0.0,
    roi REAL NOT NULL DEFAULT 0.0,
    pricedescription TEXT NOT NULL DEFAULT '{}',

    -- 执行统计
    successcount INTEGER NOT NULL DEFAULT 0,
    runcount INTEGER NOT NULL DEFAULT 0,
    successrate REAL NOT NULL DEFAULT 0.0,
    errorcount INTEGER NOT NULL DEFAULT 0,
    executiontime REAL NOT NULL DEFAULT 0.0,

    -- 状态（INTEGER：0=待领取, 1=执行中, 2=已完成, 3=失败, 6=警告(完成但有警告)）
    state INTEGER NOT NULL DEFAULT 0,

    -- 时间信息
    lastruntime TEXT NOT NULL DEFAULT '',
    lasterrortime TEXT NOT NULL DEFAULT '',
    lastoktime TEXT NOT NULL DEFAULT '',
    lasterrinfo TEXT NOT NULL DEFAULT '{}',
    lastokinfo TEXT NOT NULL DEFAULT '{}',
    uptime TEXT NOT NULL DEFAULT '',
    endtime TEXT NOT NULL DEFAULT '',

    -- 关联
    idagent TEXT NOT NULL DEFAULT '',
    idworkflowinstance TEXT NOT NULL DEFAULT '',
    idparenttask TEXT NOT NULL DEFAULT '',

    -- 输入输出
    inputjson TEXT NOT NULL DEFAULT '{}',
    outputjson TEXT NOT NULL DEFAULT '{}',
    resourcereq TEXT NOT NULL DEFAULT '{}',

    -- 系统字段
    upby TEXT NOT NULL DEFAULT '',
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
pub const SQL_CREATE_WORKFLOW_TASK_INDEX: &str = r#"
CREATE INDEX IF NOT EXISTS idx_{TABLE_NAME}_instance_state ON {TABLE_NAME} (idworkflowinstance, state)
"#;

/// WorkflowTask - 工作流任务管理
pub struct WorkflowTask {
    db: Sqlite78,
    sharding_manager: Option<ShardingManager>,
}

impl WorkflowTask {
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

        let config = ShardingConfig::new(ShardType::Daily, "workflow_task")
            .with_table_sql(SQL_CREATE_WORKFLOW_TASK)
            .with_retention(7); // 保留7天

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
            let config = ShardingConfig::new(ShardType::Daily, "workflow_task");
            config.get_current_table_name()
        } else {
            "workflow_task".to_string()
        }
    }

    /// 获取当前表名（静态方法）
    pub fn get_current_table_name_static() -> String {
        let config = ShardingConfig::new(ShardType::Daily, "workflow_task");
        config.get_current_table_name()
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
                let index_sql = SQL_CREATE_WORKFLOW_TASK_INDEX.replace("{TABLE_NAME}", &table_name);
                conn.execute(&index_sql, [])
                    .map_err(|e| format!("创建索引失败: {}", e))?;
            }
        }
        Ok(())
    }

    /// 插入任务记录
    pub fn insert(&self, data: &HashMap<String, Value>, up: &UpInfo) -> Result<String, String> {
        let table_name = self.get_table_name();
        self.create_today_table()?;

        let id = data
            .get("id")
            .and_then(|v: &serde_json::Value| v.as_str())
            .unwrap_or(&UpInfo::new_id())
            .to_string();
        let myname = data.get("myname").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("");
        let idcapability = data
            .get("idcapability")
            .and_then(|v: &serde_json::Value| v.as_str())
            .unwrap_or("");
        let apisys = data
            .get("apisys")
            .and_then(|v: &serde_json::Value| v.as_str())
            .unwrap_or("apiwf");
        let apimicro = data
            .get("apimicro")
            .and_then(|v: &serde_json::Value| v.as_str())
            .unwrap_or("basic");
        let apiobj = data.get("apiobj").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("");
        let priority = data.get("priority").and_then(|v: &serde_json::Value| v.as_i64()).unwrap_or(5) as i32;
        let state = data.get("state").and_then(|v: &serde_json::Value| v.as_i64()).unwrap_or(0) as i32;
        let starttime = data.get("starttime").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("");
        let endtime = data.get("endtime").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("");
        let inputjson = data
            .get("inputjson")
            .and_then(|v: &serde_json::Value| v.as_str())
            .unwrap_or("{}");
        let outputjson = data
            .get("outputjson")
            .and_then(|v: &serde_json::Value| v.as_str())
            .unwrap_or("{}");
        let configjson = data
            .get("configjson")
            .and_then(|v: &serde_json::Value| v.as_str())
            .unwrap_or("{}");
        let description = data
            .get("description")
            .and_then(|v: &serde_json::Value| v.as_str())
            .unwrap_or("{}");
        let cid = data.get("cid").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("");
        let idagent = data.get("idagent").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("");
        let idworkflowinstance = data
            .get("idworkflowinstance")
            .and_then(|v: &serde_json::Value| v.as_str())
            .unwrap_or("");
        let runcount = data.get("runcount").and_then(|v: &serde_json::Value| v.as_i64()).unwrap_or(0) as i32;
        let successcount = data
            .get("successcount")
            .and_then(|v: &serde_json::Value| v.as_i64())
            .unwrap_or(0) as i32;
        let errorcount = data.get("errorcount").and_then(|v: &serde_json::Value| v.as_i64()).unwrap_or(0) as i32;
        let successrate = data
            .get("successrate")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let executiontime = data
            .get("executiontime")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
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
        let lastokinfo = data
            .get("lastokinfo")
            .and_then(|v: &serde_json::Value| v.as_str())
            .unwrap_or("{}");
        let lasterrinfo = data
            .get("lasterrinfo")
            .and_then(|v: &serde_json::Value| v.as_str())
            .unwrap_or("{}");
        let price = data.get("price").and_then(|v| v.as_f64()).unwrap_or(1.0);
        let costunit = data.get("costunit").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let costtotal = data
            .get("costtotal")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let revenuetotal = data
            .get("revenuetotal")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let sql = format!(
            "REPLACE INTO {} (\
                id, myname, idcapability, apisys, apimicro,\
                apiobj, priority, state, starttime, endtime,\
                inputjson, outputjson, configjson, description, cid,\
                idagent, idworkflowinstance, runcount, successcount, errorcount,\
                successrate, executiontime, lastruntime, lastoktime, lasterrortime,\
                lastokinfo, lasterrinfo, price, costunit, costtotal,\
                revenuetotal\
            ) VALUES (\
                ?, ?, ?, ?, ?,\
                ?, ?, ?, ?, ?,\
                ?, ?, ?, ?, ?,\
                ?, ?, ?, ?, ?,\
                ?, ?, ?, ?, ?,\
                ?, ?, ?, ?, ?,\
                ?\
            )",
            table_name
        );

        self.db.do_m_add(
            &sql,
            &[
                &id as &dyn rusqlite::ToSql,
                &myname as &dyn rusqlite::ToSql,
                &idcapability as &dyn rusqlite::ToSql,
                &apisys as &dyn rusqlite::ToSql,
                &apimicro as &dyn rusqlite::ToSql,

                &apiobj as &dyn rusqlite::ToSql,
                &priority as &dyn rusqlite::ToSql,
                &state as &dyn rusqlite::ToSql,
                &starttime as &dyn rusqlite::ToSql,
                &endtime as &dyn rusqlite::ToSql,

                &inputjson as &dyn rusqlite::ToSql,
                &outputjson as &dyn rusqlite::ToSql,
                &configjson as &dyn rusqlite::ToSql,
                &description as &dyn rusqlite::ToSql,
                &cid as &dyn rusqlite::ToSql,

                &idagent as &dyn rusqlite::ToSql,
                &idworkflowinstance as &dyn rusqlite::ToSql,
                &runcount as &dyn rusqlite::ToSql,
                &successcount as &dyn rusqlite::ToSql,
                &errorcount as &dyn rusqlite::ToSql,

                &successrate as &dyn rusqlite::ToSql,
                &executiontime as &dyn rusqlite::ToSql,
                &lastruntime as &dyn rusqlite::ToSql,
                &lastoktime as &dyn rusqlite::ToSql,
                &lasterrortime as &dyn rusqlite::ToSql,

                &lastokinfo as &dyn rusqlite::ToSql,
                &lasterrinfo as &dyn rusqlite::ToSql,
                &price as &dyn rusqlite::ToSql,
                &costunit as &dyn rusqlite::ToSql,
                &costtotal as &dyn rusqlite::ToSql,

                &revenuetotal as &dyn rusqlite::ToSql,
            ],
            up,
        )?;

        Ok(id)
    }

    /// 根据 ID 查询任务
    pub fn get(&self, id: &str, up: &UpInfo) -> Result<Option<HashMap<String, Value>>, String> {
        self.create_today_table()?;
        let table_name = self.get_table_name();
        let sql = format!("SELECT * FROM {} WHERE id = ?", table_name);
        let rows = self.db.do_get(&sql, &[&id as &dyn rusqlite::ToSql], up)?;
        Ok(rows.into_iter().next())
    }

    /// 更新任务状态
    pub fn update_state(&self, id: &str, state: i32, up: &UpInfo) -> Result<(), String> {
        self.create_today_table()?;
        let table_name = self.get_table_name();
        let sql = format!("UPDATE {} SET state = ? WHERE id = ?", table_name);
        self.db.do_m(
            &sql,
            &[&state as &dyn rusqlite::ToSql, &id as &dyn rusqlite::ToSql],
            up,
        )?;
        Ok(())
    }

    /// 标记任务完成（state=2，记录成功信息）
    pub fn mark_completed(&self, id: &str, info: &str, up: &UpInfo) -> Result<(), String> {
        self.create_today_table()?;
        let table_name = self.get_table_name();
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let sql = format!(
            "UPDATE {} SET state = 2, lastoktime = ?, lastokinfo = ? WHERE id = ?",
            table_name
        );
        self.db.do_m(
            &sql,
            &[
                &now as &dyn rusqlite::ToSql,
                &info as &dyn rusqlite::ToSql,
                &id as &dyn rusqlite::ToSql,
            ],
            up,
        )?;
        Ok(())
    }

    /// 标记任务失败（state=3，记录错误信息）
    pub fn mark_failed(&self, id: &str, errinfo: &str, up: &UpInfo) -> Result<(), String> {
        self.create_today_table()?;
        let table_name = self.get_table_name();
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let sql = format!(
            "UPDATE {} SET state = 3, lasterrortime = ?, lasterrinfo = ? WHERE id = ?",
            table_name
        );
        self.db.do_m(
            &sql,
            &[
                &now as &dyn rusqlite::ToSql,
                &errinfo as &dyn rusqlite::ToSql,
                &id as &dyn rusqlite::ToSql,
            ],
            up,
        )?;
        Ok(())
    }

    /// 查询工作流实例的所有任务
    pub fn get_by_instance(
        &self,
        idworkflowinstance: &str,
        up: &UpInfo,
    ) -> Result<Vec<HashMap<String, Value>>, String> {
        self.create_today_table()?;
        let table_name = self.get_table_name();
        let sql = format!(
            "SELECT * FROM {} WHERE idworkflowinstance = ? ORDER BY starttime DESC",
            table_name
        );
        self.db
            .do_get(&sql, &[&idworkflowinstance as &dyn rusqlite::ToSql], up)
    }

    /// 查询指定状态的任务
    pub fn get_by_state(
        &self,
        state: i32,
        up: &UpInfo,
    ) -> Result<Vec<HashMap<String, Value>>, String> {
        self.create_today_table()?;
        let table_name = self.get_table_name();
        let sql = format!(
            "SELECT * FROM {} WHERE state = ? ORDER BY priority DESC, starttime ASC",
            table_name
        );
        self.db.do_get(&sql, &[&state as &dyn rusqlite::ToSql], up)
    }

    /// 获取底层数据库引用
    pub fn get_db(&self) -> &Sqlite78 {
        &self.db
    }

    /// 根据 ID 查询记录（包装 get 方法）
    pub fn get_by_id(&self, id: &str) -> Result<Option<HashMap<String, Value>>, String> {
        let up = UpInfo::new();
        self.get(id, &up)
    }

    /// 更新记录（完整更新）
    pub fn update(&self, data: &HashMap<String, Value>) -> Result<(), String> {
        let id = data.get("id")
            .and_then(|v: &serde_json::Value| v.as_str())
            .ok_or_else(|| "缺少 id 字段".to_string())?;

        let up = UpInfo::new();
        let table_name = self.get_table_name();

        // 构建 SET 子句（排除 id）
        let mut set_clauses = Vec::new();
        let mut params: Vec<&dyn rusqlite::ToSql> = Vec::new();

        // 所有字段列表（与 INSERT 对应）
        let fields = [
            "myname", "idcapability", "apisys", "apimicro", "apiobj",
            "priority", "state", "starttime", "endtime",
            "inputjson", "outputjson", "configjson", "description", "cid",
            "idagent", "idworkflowinstance", "runcount", "successcount", "errorcount",
            "successrate", "executiontime", "lastruntime", "lastoktime", "lasterrortime",
            "lastokinfo", "lasterrinfo", "price", "costunit", "costtotal", "revenuetotal"
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
            return Ok(()); // 没有要更新的字段
        }

        let sql = format!("UPDATE {} SET {} WHERE id = ?", table_name, set_clauses.join(", "));
        params.push(&id as &dyn rusqlite::ToSql);

        self.db.do_m(&sql, &params, &up)?;
        Ok(())
    }

    /// 查询记录列表（支持条件）
    pub fn query_list(&self, condition: &str, params: &[&dyn rusqlite::ToSql]) -> Result<Vec<HashMap<String, Value>>, String> {
        let up = UpInfo::new();
        let table_name = self.get_table_name();
        let sql = if condition.is_empty() {
            format!("SELECT * FROM {}", table_name)
        } else {
            format!("SELECT * FROM {} WHERE {}", table_name, condition)
        };
        self.db.do_get(&sql, params, &up)
    }

    /// 删除记录
    pub fn delete(&self, id: &str) -> Result<(), String> {
        let up = UpInfo::new();
        let table_name = self.get_table_name();
        let sql = format!("DELETE FROM {} WHERE id = ?", table_name);
        self.db.do_m(&sql, &[&id as &dyn rusqlite::ToSql], &up)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_task_basic() {

        let task = WorkflowTask::with_default_path()
            .expect("创建失败");

        let table_name = task.get_table_name();
        assert!(table_name.starts_with("workflow_task_"), "表名应该是分表格式");

        let up = UpInfo::new();
        let mut data = HashMap::new();
        data.insert("id".to_string(), Value::String("test-task-001".to_string()));
        data.insert("myname".to_string(), Value::String("测试任务".to_string()));
        data.insert("apisys".to_string(), Value::String("test".to_string()));
        data.insert("apimicro".to_string(), Value::String("workflow".to_string()));
        data.insert("apiobj".to_string(), Value::String("task".to_string()));
        data.insert("state".to_string(), Value::Number(1.into())); // 1=待执行
        data.insert("priority".to_string(), Value::Number(5.into()));

        let unique_id = "test-task-001".to_string();
        let result = task.insert(&data, &up);
        assert!(result.is_ok(), "插入应该成功: {:?}", result);

        // 查询插入的记录
        let found = task.get_by_id(&unique_id);
        assert!(found.is_ok(), "查询应该成功: {:?}", found);
        let record = found.unwrap();
        assert!(record.is_some(), "记录应该存在");
    }

    #[test]
    fn test_workflow_task_delete() {

        let task = WorkflowTask::with_default_path()
            .expect("创建失败");

        // 插入一条测试数据
        let up = UpInfo::new();
        let mut data = HashMap::new();
        data.insert("id".to_string(), Value::String("test-task-delete".to_string()));
        data.insert("myname".to_string(), Value::String("删除测试任务".to_string()));
        data.insert("apisys".to_string(), Value::String("test".to_string()));
        data.insert("apimicro".to_string(), Value::String("workflow".to_string()));
        data.insert("apiobj".to_string(), Value::String("delete".to_string()));
        data.insert("state".to_string(), Value::Number(1.into()));

        let result = task.insert(&data, &up);
        assert!(result.is_ok(), "插入应该成功");

        // 删除记录
        let delete_result = task.delete("test-task-delete");
        assert!(delete_result.is_ok(), "删除应该成功");

        // 验证删除成功
        let found = task.get_by_id("test-task-delete");
        assert!(found.is_ok(), "查询应该成功");
        assert!(found.unwrap().is_none(), "记录应该已被删除");
    }
}
