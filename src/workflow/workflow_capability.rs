//! WorkflowCapability - 能力定义表管理
//!
//! ## 表关系说明
//! - workflow_capability: 能力【定义表】- 存储能力模板、配置、价格等静态定义
//! - workflow_task: 任务【实例表】- 每次能力执行产生的任务实例记录
//!
//! 管理 workflow_capability 表的 CRUD 操作

use crate::{Sqlite78, UpInfo};
use serde_json::Value;
use std::collections::HashMap;

/// 工作流能力定义表建表 SQL
pub const SQL_CREATE_WORKFLOW_CAPABILITY: &str = r#"
CREATE TABLE IF NOT EXISTS workflow_capability (
    -- 主键
    id TEXT NOT NULL PRIMARY KEY,
    -- 能力名称
    capability TEXT NOT NULL DEFAULT '',
    -- API 分类
    apisys TEXT NOT NULL DEFAULT 'apiwf',
    apimicro TEXT NOT NULL DEFAULT 'basic',
    apiobj TEXT NOT NULL DEFAULT '',
    -- 描述和状态
    description TEXT NOT NULL DEFAULT '{}',
    state INTEGER NOT NULL DEFAULT 1,
    -- 输入输出模式
    inputjson TEXT NOT NULL DEFAULT '{}',
    outputjson TEXT NOT NULL DEFAULT '{}',
    -- 执行配置
    priority INTEGER NOT NULL DEFAULT 0,
    maxcopy INTEGER NOT NULL DEFAULT 1,
    timeout INTEGER NOT NULL DEFAULT 600,
    retrylimit INTEGER NOT NULL DEFAULT 3,
    retryinterval INTEGER NOT NULL DEFAULT 60,
    -- 配置信息
    configjson TEXT NOT NULL DEFAULT '{}',
    resourcereq TEXT NOT NULL DEFAULT '{}',
    dependencies TEXT NOT NULL DEFAULT '[]',
    -- 价格成本配置
    pricebase REAL NOT NULL DEFAULT 1.0,
    price REAL NOT NULL DEFAULT 1.0,
    costunit REAL NOT NULL DEFAULT 0.0,
    profittarget REAL NOT NULL DEFAULT 0.2,
    -- 财务统计
    profittotal REAL NOT NULL DEFAULT 0.0,
    costtotal REAL NOT NULL DEFAULT 0.0,
    revenuetotal REAL NOT NULL DEFAULT 0.0,
    roi REAL NOT NULL DEFAULT 0.0,
    -- 描述信息
    costdescription TEXT NOT NULL DEFAULT '{}',
    pricedescription TEXT NOT NULL DEFAULT '{}',
    -- 执行统计
    successcount INTEGER NOT NULL DEFAULT 0,
    runcount INTEGER NOT NULL DEFAULT 0,
    successrate REAL NOT NULL DEFAULT 0.0,
    errorcount INTEGER NOT NULL DEFAULT 0,
    executiontime REAL NOT NULL DEFAULT 0.0,
    -- 时间信息
    lastruntime TEXT NOT NULL DEFAULT '',
    lastoktime TEXT NOT NULL DEFAULT '',
    lasterrortime TEXT NOT NULL DEFAULT '',
    lasterrinfo TEXT NOT NULL DEFAULT '{}',
    lastokinfo TEXT NOT NULL DEFAULT '{}',
    -- 用户信息
    uname TEXT NOT NULL DEFAULT '',
    uid TEXT NOT NULL DEFAULT '',
    cid TEXT NOT NULL DEFAULT '',
    -- 时间戳
    created_at TEXT NOT NULL DEFAULT '',
    updated_at TEXT NOT NULL DEFAULT ''
)
"#;

/// WorkflowCapability - 工作流能力定义管理
pub struct WorkflowCapability {
    db: Sqlite78,
}

impl WorkflowCapability {
    /// 创建新实例
    pub fn new(db: Sqlite78) -> Self {
        Self { db }
    }

    /// 使用默认数据库路径创建实例
    pub fn with_default_path() -> Result<Self, String> {
        let mut db = Sqlite78::with_default_path();
        db.initialize()?;
        Ok(Self { db })
    }

    /// 使用指定路径创建实例
    pub fn with_path(path: &str) -> Result<Self, String> {
        let mut db = Sqlite78::with_config(path, false, false);
        db.initialize()?;
        Ok(Self { db })
    }

    /// 初始化表
    pub async fn init_table(&self) -> Result<(), String> {
        let conn = self.db.get_conn()?;
        let conn = conn.lock().await;
        conn.execute(SQL_CREATE_WORKFLOW_CAPABILITY, [])
            .map_err(|e| format!("创建 workflow_capability 表失败: {}", e))?;
        Ok(())
    }

    /// 插入或更新能力定义
    pub async fn insert(&self, data: &HashMap<String, Value>, up: &UpInfo) -> Result<String, String> {
        let id = data.get("id")
            .and_then(|v: &serde_json::Value| v.as_str())
            .unwrap_or(&UpInfo::new_id())
            .to_string();

        let columns: Vec<&str> = data.keys().map(|s| s.as_str()).collect();
        let placeholders: Vec<&str> = (0..columns.len()).map(|_| "?").collect();
        let sql = format!(
            "REPLACE INTO workflow_capability ({}) VALUES ({})",
            columns.join(", "),
            placeholders.join(", ")
        );

        let values: Vec<String> = data.values().map(|v| {
            match v {
                Value::Null => "".to_string(),
                Value::Bool(b) => b.to_string(),
                Value::Number(n) => n.to_string(),
                Value::String(s) => s.clone(),
                Value::Array(_) | Value::Object(_) => serde_json::to_string(v).unwrap_or_default(),
            }
        }).collect();

        let params_vec: Vec<&dyn rusqlite::ToSql> = values.iter()
            .map(|v| v as &dyn rusqlite::ToSql)
            .collect();

        let result = self.db.do_m_add_tosql(&sql, params_vec.as_slice(), up).await?;
        if let Some(e) = result.error {
            return Err(format!("插入失败: {}", e));
        }

        Ok(id)
    }

    /// 根据 ID 查询能力定义
    pub async fn get(&self, id: &str, up: &UpInfo) -> Result<Option<HashMap<String, Value>>, String> {
        let sql = "SELECT * FROM workflow_capability WHERE id = ?";
        let rows = self.db.do_get_tosql(sql, &[&id as &dyn rusqlite::ToSql], up).await?;
        Ok(rows.into_iter().next())
    }

    /// 更新能力状态
    pub async fn update_state(&self, id: &str, state: i32, up: &UpInfo) -> Result<(), String> {
        let sql = "UPDATE workflow_capability SET state = ? WHERE id = ?";
        self.db.do_m_tosql(sql, &[&state as &dyn rusqlite::ToSql, &id as &dyn rusqlite::ToSql], up).await?;
        Ok(())
    }

    /// 查询所有启用的能力
    pub async fn get_enabled(&self, up: &UpInfo) -> Result<Vec<HashMap<String, Value>>, String> {
        let sql = "SELECT * FROM workflow_capability WHERE state = 1 ORDER BY priority DESC";
        self.db.do_get(sql, &[], up).await
    }

    /// 获取底层数据库引用
    pub fn get_db(&self) -> &Sqlite78 {
        &self.db
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_capability_init() {
        let cap = WorkflowCapability::with_path("tmp/tmp/test_workflow_cap.db").expect("创建失败");
        let result = cap.init_table();
        assert!(result.is_ok(), "初始化表应该成功: {:?}", result);
    }

    #[test]
    fn test_workflow_capability_insert_and_get() {
        let cap = WorkflowCapability::with_path("tmp/tmp/test_workflow_cap_crud.db").expect("创建失败");
        cap.init_table().expect("初始化表失败");

        let up = UpInfo::new();

        let mut data = HashMap::new();
        data.insert("id".to_string(), Value::String("cap_001".to_string()));
        data.insert("capability".to_string(), Value::String("test_capability".to_string()));
        data.insert("description".to_string(), Value::String("测试能力".to_string()));

        let result = cap.insert(&data, &up);
        assert!(result.is_ok(), "插入应该成功: {:?}", result);

        // 查询
        let found = cap.get("cap_001", &up).expect("查询失败");
        assert!(found.is_some(), "应该找到记录");
        let record = found.unwrap();
        assert_eq!(
            record.get("capability").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or(""),
            "test_capability"
        );
    }
}