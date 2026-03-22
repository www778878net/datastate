//! 工作流实例基类 - 所有工作流实例的基类
//!
//! 特点:
//! - 轻量级，无 LangGraph 依赖
//! - 接收 context78: dict，返回 context78: dict
//! - 保留完整的生命周期管理、经济统计、日志记录
//! - 支持DAG有向无环图编排（条件节点）
//! - 执行完自动保存到 workflow_instance 表（按天分表）

use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Instant;

use crate::components::{BaseEntity, EconomicManager, LifecycleManager};
use crate::workflow::WorkflowInstance;
use crate::UpInfo;

/// 工作流实例执行结果类
#[derive(Debug, Clone)]
pub struct InstanceResult {
    /// 执行结果: 0=成功, 其他=失败
    pub res: i32,
    /// 错误信息
    pub errmsg: String,
    /// 业务结果
    pub result: Option<Value>,
    /// 操作描述
    pub operation: Option<String>,
}

impl InstanceResult {
    /// 创建新的结果
    pub fn new(res: i32, errmsg: String, result: Option<Value>, operation: Option<String>) -> Self {
        Self {
            res,
            errmsg,
            result,
            operation,
        }
    }

    /// 转换为字典
    pub fn to_dict(&self) -> Value {
        serde_json::json!({
            "res": self.res,
            "errmsg": self.errmsg,
            "result": self.result,
            "operation": self.operation
        })
    }

    /// 判断是否成功
    pub fn is_success(&self) -> bool {
        self.res == 0
    }

    /// 创建成功结果
    pub fn success(result: Option<Value>, operation: Option<String>) -> Self {
        Self {
            res: 0,
            errmsg: String::new(),
            result,
            operation,
        }
    }

    /// 创建失败结果
    pub fn failure(errmsg: &str, res: i32, operation: Option<String>) -> Self {
        Self {
            res,
            errmsg: errmsg.to_string(),
            result: None,
            operation,
        }
    }
}

impl Default for InstanceResult {
    fn default() -> Self {
        Self {
            res: 0,
            errmsg: String::new(),
            result: None,
            operation: None,
        }
    }
}

/// 工作流实例基类 Trait
///
/// 设计原则:
///     KISS - 简单直接
///     开放封闭 - 对扩展开放，对修改封闭
#[async_trait]
pub trait BaseInstance: Send + Sync {
    /// 获取基础实体组件
    fn base(&self) -> &BaseEntity;

    /// 获取可变基础实体组件
    fn base_mut(&mut self) -> &mut BaseEntity;

    /// 获取生命周期管理组件
    fn lifecycle(&self) -> &LifecycleManager;

    /// 获取可变生命周期管理组件
    fn lifecycle_mut(&mut self) -> &mut LifecycleManager;

    /// 获取经济管理组件
    fn economic(&self) -> &EconomicManager;

    /// 获取可变经济管理组件
    fn economic_mut(&mut self) -> &mut EconomicManager;

    /// 获取实例名称
    fn instance_name(&self) -> &str;

    /// 获取 DAG 图定义
    fn graph(&self) -> Option<&HashMap<String, Vec<String>>> {
        None
    }

    /// 获取分支节点列表
    fn branch_nodes(&self) -> Option<&Vec<String>> {
        None
    }

    /// 业务逻辑执行 - 子类实现
    async fn run(&mut self, context78: HashMap<String, Value>) -> HashMap<String, Value>;

    /// 执行方法，包含完整的生命周期、经济统计和日志记录
    async fn execute(&mut self, context78: HashMap<String, Value>) -> HashMap<String, Value> {
        let start_time = Instant::now();

        // 标记开始执行
        self.lifecycle_mut().mark_started();
        self.base_mut().state = 1; // running

        // 更新工作流ID
        let workflow_id = context78
            .get("idworkflowinstance")
            .and_then(|v| v.as_str())
            .unwrap_or(&self.base().idworkflowinstance);
        if !workflow_id.is_empty() {
            self.base_mut().idworkflowinstance = workflow_id.to_string();
        }

        // 记录输入
        self.base_mut().inputjson =
            serde_json::to_value(&context78).unwrap_or(Value::Object(serde_json::Map::new()));

        // 执行业务逻辑
        let result = self.run(context78.clone()).await;

        // 计算执行时间
        let execution_time = start_time.elapsed().as_secs_f64();

        // 判断执行结果
        let res = result.get("res").and_then(|v| v.as_i64()).unwrap_or(0);

        // 序列化结果用于存储
        let result_json = serde_json::to_string(&result).unwrap_or_else(|_| "{}".to_string());
        let result_value =
            serde_json::from_str(&result_json).unwrap_or(Value::Object(serde_json::Map::new()));

        // 检查是否有警告信息
        let has_warnings = result
            .get("warnings")
            .and_then(|v| v.as_array())
            .map(|arr| !arr.is_empty())
            .unwrap_or(false);

        if res == 0 {
            self.lifecycle_mut().record_success(execution_time);
            self.lifecycle_mut().lastokinfo = result_value.clone();
            // 如果有警告信息，状态为 6 (警告)，否则为 2 (完成)
            self.base_mut().state = if has_warnings { 6 } else { 2 };
        } else {
            self.lifecycle_mut().record_error();
            self.lifecycle_mut().lasterrinfo = result_value.clone();
            self.base_mut().state = 3; // failed
        }

        // 记录输出
        self.base_mut().outputjson = result_value;

        // 更新经济统计
        let cost = execution_time * self.economic().costunit;
        let price = self.economic().price;
        self.economic_mut().add_cost(cost);
        if res == 0 {
            self.economic_mut().add_revenue(price);
        }

        // 保存执行记录到数据库
        if let Err(e) = self.save_execution() {
            eprintln!("保存工作流实例记录失败: {}", e);
        }

        result
    }

    /// 保存执行记录到数据库 - 保存到 workflow_instance 表（按天分表）
    fn save_execution(&mut self) -> Result<(), String> {
        let workflow_instance = WorkflowInstance::with_default_path()?;

        // 创建今天的分表
        workflow_instance.create_today_table()?;

        let up = UpInfo::new();
        let json = self.to_instance_json();

        let _id = workflow_instance.insert(&json, &up)?;

        Ok(())
    }

    /// 转换为实例记录 JSON（保存到 workflow_instance 表）
    fn to_instance_json(&mut self) -> HashMap<String, Value> {
        let mut result = HashMap::new();

        // 生成实例 ID
        let id = if self.base().id.is_empty() {
            format!(
                "{}_{}",
                self.instance_name(),
                chrono::Local::now().format("%Y%m%d%H%M%S")
            )
        } else {
            self.base().id.clone()
        };

        result.insert("id".to_string(), Value::String(id));
        result.insert(
            "myname".to_string(),
            Value::String(self.instance_name().to_string()),
        );

        // API 分类
        result.insert(
            "apisys".to_string(),
            Value::String(self.base().apisys.clone()),
        );
        result.insert(
            "apimicro".to_string(),
            Value::String(self.base().apimicro.clone()),
        );
        result.insert(
            "apiobj".to_string(),
            Value::String(self.base().apiobj.clone()),
        );

        // 状态
        result.insert("state".to_string(), Value::Number(self.base().state.into()));
        result.insert(
            "priority".to_string(),
            Value::Number(self.base().priority.into()),
        );

        // 时间
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        result.insert("starttime".to_string(), Value::String(now.clone()));
        if self.base().state == 2 || self.base().state == 3 || self.base().state == 6 {
            result.insert("endtime".to_string(), Value::String(now));
        }

        // 输入输出 - 需要转换为字符串
        let inputjson_str =
            serde_json::to_string(&self.base().inputjson).unwrap_or_else(|_| "{}".to_string());
        result.insert("inputjson".to_string(), Value::String(inputjson_str));
        result.insert("outputjson".to_string(), Value::String("{}".to_string()));
        let configjson_str =
            serde_json::to_string(&self.base().configjson).unwrap_or_else(|_| "{}".to_string());
        result.insert("configjson".to_string(), Value::String(configjson_str));
        let description_str =
            serde_json::to_string(&self.base().description).unwrap_or_else(|_| "{}".to_string());
        result.insert("description".to_string(), Value::String(description_str));

        // 用户信息
        result.insert("cid".to_string(), Value::String(self.base().cid.clone()));
        result.insert(
            "idagent".to_string(),
            Value::String(self.base().idagent.clone()),
        );

        // 生命周期统计
        result.insert(
            "runcount".to_string(),
            Value::Number(self.lifecycle().runcount.into()),
        );
        result.insert(
            "successcount".to_string(),
            Value::Number(self.lifecycle().successcount.into()),
        );
        result.insert(
            "errorcount".to_string(),
            Value::Number(self.lifecycle().errorcount.into()),
        );
        result.insert(
            "successrate".to_string(),
            serde_json::json!(self.lifecycle().successrate),
        );
        result.insert(
            "executiontime".to_string(),
            serde_json::json!(self.lifecycle().executiontime),
        );

        // lastokinfo 和 lasterrinfo 需要转换为字符串
        let lastokinfo_str = serde_json::to_string(&self.lifecycle().lastokinfo)
            .unwrap_or_else(|_| "{}".to_string());
        result.insert("lastokinfo".to_string(), Value::String(lastokinfo_str));
        let lasterrinfo_str = serde_json::to_string(&self.lifecycle().lasterrinfo)
            .unwrap_or_else(|_| "{}".to_string());
        result.insert("lasterrinfo".to_string(), Value::String(lasterrinfo_str));

        // 时间信息
        if let Some(t) = &self.lifecycle().lastruntime {
            result.insert(
                "lastruntime".to_string(),
                Value::String(t.format("%Y-%m-%d %H:%M:%S").to_string()),
            );
        }
        if let Some(t) = &self.lifecycle().lastoktime {
            result.insert(
                "lastoktime".to_string(),
                Value::String(t.format("%Y-%m-%d %H:%M:%S").to_string()),
            );
        }
        if let Some(t) = &self.lifecycle().lasterrortime {
            result.insert(
                "lasterrortime".to_string(),
                Value::String(t.format("%Y-%m-%d %H:%M:%S").to_string()),
            );
        }

        // 经济信息
        result.insert(
            "costtotal".to_string(),
            serde_json::json!(self.economic().costtotal),
        );
        result.insert(
            "revenuetotal".to_string(),
            serde_json::json!(self.economic().revenuetotal),
        );
        result.insert(
            "profittotal".to_string(),
            serde_json::json!(self.economic().revenuetotal - self.economic().costtotal),
        );

        let roi = if self.economic().costtotal > 0.0 {
            (self.economic().revenuetotal - self.economic().costtotal) / self.economic().costtotal
        } else {
            0.0
        };
        result.insert("roi".to_string(), serde_json::json!(roi));

        result
    }

    /// 使实例可直接调用
    async fn call(&mut self, context78: HashMap<String, Value>) -> HashMap<String, Value> {
        self.execute(context78).await
    }
}

/// 实例基类实现辅助结构
pub struct InstanceBase {
    pub base: BaseEntity,
    pub lifecycle: LifecycleManager,
    pub economic: EconomicManager,
    pub idworkflowdefinition: String,
    pub idparentinstance: String,
    pub priority: i32,
    pub maxcopy: i32,
    pub currentcopy: i32,
    pub timeout: i32,
    pub retrylimit: i32,
    pub retryinterval: i32,
    pub flowschema: Value,
    pub resourcereq: Value,
}

impl Default for InstanceBase {
    fn default() -> Self {
        Self {
            base: BaseEntity::default(),
            lifecycle: LifecycleManager::default(),
            economic: EconomicManager::default(),
            idworkflowdefinition: String::new(),
            idparentinstance: String::new(),
            priority: 0,
            maxcopy: 1,
            currentcopy: 0,
            timeout: 3600,
            retrylimit: 1,
            retryinterval: 15,
            flowschema: Value::Object(serde_json::Map::new()),
            resourcereq: Value::Object(serde_json::Map::new()),
        }
    }
}

impl InstanceBase {
    /// 从 JSON 数据初始化
    pub fn from_json(json_data: &HashMap<String, Value>) -> Self {
        let mut base = Self::default();

        // 过滤掉需要丢弃的字段
        let ignored_fields = [
            "uptime", "upby", "remark", "remark2", "remark3", "remark4", "remark5", "remark6",
        ];
        let filtered_data: HashMap<String, Value> = json_data
            .iter()
            .filter(|(k, _)| !ignored_fields.contains(&k.as_str()))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        // 加载组件
        base.base.load_from_dict(&filtered_data);
        base.lifecycle.load_from_dict(&filtered_data);
        base.economic.load_from_dict(&filtered_data);

        // 确保 ID 存在
        if base.base.id.is_empty() {
            base.base.id = crate::snowflake::next_id_string();
        }

        // 设置工作流实例ID
        if base.base.idworkflowinstance.is_empty() {
            base.base.idworkflowinstance = base.base.id.clone();
        }

        // 加载实例特有字段
        if let Some(v) = filtered_data
            .get("idworkflowdefinition")
            .and_then(|v| v.as_str())
        {
            base.idworkflowdefinition = v.to_string();
        }
        if let Some(v) = filtered_data
            .get("idparentinstance")
            .and_then(|v| v.as_str())
        {
            base.idparentinstance = v.to_string();
        }
        if let Some(v) = filtered_data.get("priority").and_then(|v| v.as_i64()) {
            base.priority = v as i32;
        }
        if let Some(v) = filtered_data.get("maxcopy").and_then(|v| v.as_i64()) {
            base.maxcopy = v as i32;
        }
        if let Some(v) = filtered_data.get("currentcopy").and_then(|v| v.as_i64()) {
            base.currentcopy = v as i32;
        }
        if let Some(v) = filtered_data.get("timeout").and_then(|v| v.as_i64()) {
            base.timeout = v as i32;
        }
        if let Some(v) = filtered_data.get("retrylimit").and_then(|v| v.as_i64()) {
            base.retrylimit = v as i32;
        }
        if let Some(v) = filtered_data.get("retryinterval").and_then(|v| v.as_i64()) {
            base.retryinterval = v as i32;
        }
        if let Some(v) = filtered_data.get("flowschema").cloned() {
            base.flowschema = v;
        }
        if let Some(v) = filtered_data.get("resourcereq").cloned() {
            base.resourcereq = v;
        }

        base
    }

    /// 转换为 JSON
    pub fn to_json(&self) -> HashMap<String, Value> {
        let mut result = HashMap::new();

        result.extend(self.base.to_dict());
        result.extend(self.lifecycle.to_dict());
        result.extend(self.economic.to_dict());

        result.insert(
            "idworkflowdefinition".to_string(),
            Value::String(self.idworkflowdefinition.clone()),
        );
        result.insert(
            "idparentinstance".to_string(),
            Value::String(self.idparentinstance.clone()),
        );
        result.insert("priority".to_string(), Value::Number(self.priority.into()));
        result.insert("maxcopy".to_string(), Value::Number(self.maxcopy.into()));
        result.insert(
            "currentcopy".to_string(),
            Value::Number(self.currentcopy.into()),
        );
        result.insert("timeout".to_string(), Value::Number(self.timeout.into()));
        result.insert(
            "retrylimit".to_string(),
            Value::Number(self.retrylimit.into()),
        );
        result.insert(
            "retryinterval".to_string(),
            Value::Number(self.retryinterval.into()),
        );
        result.insert("flowschema".to_string(), self.flowschema.clone());
        result.insert("resourcereq".to_string(), self.resourcereq.clone());

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instance_base_default() {
        let base = InstanceBase::default();

        assert_eq!(base.priority, 0);
        assert_eq!(base.maxcopy, 1);
        assert_eq!(base.timeout, 3600);
    }

    #[test]
    fn test_instance_base_from_json() {
        let mut json = HashMap::new();
        json.insert("id".to_string(), Value::String("test-id".to_string()));
        json.insert("priority".to_string(), Value::Number(5.into()));
        json.insert("price".to_string(), serde_json::json!(10.0));

        let base = InstanceBase::from_json(&json);

        assert_eq!(base.base.id, "test-id");
        assert_eq!(base.priority, 5);
        assert_eq!(base.economic.price, 10.0);
    }

    #[test]
    fn test_instance_base_auto_id() {
        let base = InstanceBase::from_json(&HashMap::new());
        assert!(!base.base.id.is_empty());
        assert_eq!(base.base.id.len(), 36); // UUID
    }
}
