//! 能力基类 - 所有工作流能力的基类
//!
//! 特点:
//! - 轻量级，无 LangGraph 依赖
//! - 接收 context78: dict，返回 context78: dict
//! - 保留完整的生命周期管理、经济统计、日志记录
//! - 执行完自动保存到 workflow_task 表（任务实例表）
//!
//! 返回结果建议使用 CapabilityResult:
//! ```ignore
//! let cr = CapabilityResult::success(Some(result), Some("操作描述".to_string()));
//! context78.insert("类名".to_string(), cr.to_dict());
//! ```

use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use crate::components::{BaseEntity, EconomicManager, LifecycleManager};
use crate::workflow::{WorkflowCapability, WorkflowTask};
use crate::UpInfo;
use base::MyLogger;

/// 能力基类 Trait
///
/// 设计原则:
///     KISS - 简单直接
///     开放封闭 - 对扩展开放，对修改封闭
#[async_trait]
pub trait BaseCapability: Send + Sync {
    /// 获取基础实体组件（可变）
    fn base(&mut self) -> &mut BaseEntity;

    /// 获取生命周期管理组件（可变）
    fn lifecycle(&mut self) -> &mut LifecycleManager;

    /// 获取经济管理组件（可变）
    fn economic(&mut self) -> &mut EconomicManager;

    /// 获取能力名称
    fn capability_name(&self) -> &str;

    /// 获取日志器
    fn logger(&self) -> Option<&Arc<MyLogger>> {
        None
    }

    /// 设置日志器
    fn set_logger(&mut self, _logger: Arc<MyLogger>) {}

    /// 是否保存 task 记录到 workflow_task 表
    /// 默认 false，只有需要持久化的关键能力设为 true
    fn save_task(&self) -> bool {
        false
    }

    /// 子类必须实现的业务逻辑方法
    ///
    /// 参数:
    ///     context78: 状态机的状态集合
    ///
    /// 返回:
    ///     更新后的 context78
    async fn run(&self, context78: HashMap<String, Value>) -> HashMap<String, Value>;

    /// 执行方法，包含完整的生命周期、经济统计和日志记录
    async fn execute(&mut self, context78: HashMap<String, Value>) -> HashMap<String, Value> {
        let start_time = Instant::now();
        let cap_name = self.capability_name().to_string();

        // 标记开始执行
        self.lifecycle().mark_started();
        self.base().state = 1; // running

        // 记录输入
        self.base().inputjson =
            serde_json::to_value(&context78).unwrap_or(Value::Object(serde_json::Map::new()));

        // 记录开始日志
        if let Some(logger) = self.logger() {
            logger.debug(&format!("开始执行能力: {}", cap_name));
        }

        // 执行业务逻辑
        let result = self.run(context78.clone()).await;

        // 计算执行时间
        let execution_time = start_time.elapsed().as_secs_f64();

        // 判断执行结果 - 先查顶层，再查嵌套的能力对象
        let res = result
            .get("res")
            .and_then(|v: &serde_json::Value| v.as_i64())
            .or_else(|| {
                result
                    .get(cap_name.as_str())
                    .and_then(|v| v.get("res"))
                    .and_then(|v: &serde_json::Value| v.as_i64())
            })
            .unwrap_or(-1);

        // 序列化结果用于存储
        let result_value =
            serde_json::to_value(&result).unwrap_or(Value::Object(serde_json::Map::new()));

        // 检查是否有警告信息
        let has_warnings = result
            .get("warnings")
            .and_then(|v| v.as_array())
            .map(|arr| !arr.is_empty())
            .unwrap_or(false);

        if res == 0 {
            // 业务执行成功
            self.lifecycle().record_success(execution_time);
            // 单能力只保存自己的结果，从 context78 中提取以能力名为 key 的数据
            let cap_result = result
                .get(cap_name.as_str())
                .cloned()
                .unwrap_or_else(|| Value::Object(serde_json::Map::new()));
            self.lifecycle().lastokinfo = cap_result;
            // 如果有警告信息，状态为 6 (警告)，否则为 2 (完成)
            self.base().state = if has_warnings { 6 } else { 2 };
            self.base().outputjson = result_value;
            if let Some(logger) = self.logger() {
                if has_warnings {
                    logger.warn(&format!(
                        "能力执行成功但有警告: {}, 耗时: {:.2}秒",
                        cap_name, execution_time
                    ));
                } else {
                    logger.detail(&format!(
                        "能力执行成功: {}, 耗时: {:.2}秒",
                        cap_name, execution_time
                    ));
                }
            }
        } else {
            // 业务执行失败
            self.lifecycle().record_error();
            self.lifecycle().lasterrinfo = result_value;
            self.base().state = 3; // failed
            let errmsg = result
                .get(cap_name.as_str())
                .and_then(|v| v.get("errmsg"))
                .and_then(|v: &serde_json::Value| v.as_str())
                .unwrap_or("未知错误");
            if let Some(logger) = self.logger() {
                logger.error(&format!(
                    "能力执行失败: {}, 错误: {}, 耗时: {:.2}秒",
                    cap_name, errmsg, execution_time
                ));
            }
        }

        // 更新经济统计
        let cost = execution_time * self.economic().costunit;
        let price = self.economic().price;
        self.economic().add_cost(cost);
        if res == 0 {
            self.economic().add_revenue(price);
        }

        // 保存执行记录到数据库（仅 save_task=true 时保存）
        if self.save_task() {
            if let Some(logger) = self.logger() {
                logger.detail(&format!("准备保存能力执行记录: {}", cap_name));
            }
            if let Err(e) = self.save_execution() {
                if let Some(logger) = self.logger() {
                    logger.error(&format!("保存能力执行记录失败: {}", e));
                }
            }
        }

        result
    }

    /// 保存执行记录到数据库 - 保存到 workflow_task 表（任务实例表）
    fn save_execution(&mut self) -> Result<(), String> {
        let workflow_task = WorkflowTask::with_default_path()?;

        // 创建今天的分表
        workflow_task.create_today_table()?;

        let up = UpInfo::new();
        let json = self.to_task_json();

        let id = workflow_task.insert(&json, &up)?;

        if let Some(logger) = self.logger() {
            logger.detail(&format!("任务记录保存成功, id: {}", id));
        }

        Ok(())
    }

    /// 转换为任务记录 JSON（保存到 workflow_task 表）
    fn to_task_json(&mut self) -> HashMap<String, Value> {
        let mut result = HashMap::new();

        // 生成任务 ID
        let id = format!(
            "{}_{}",
            self.capability_name(),
            chrono::Local::now().format("%Y%m%d%H%M%S")
        );
        result.insert("id".to_string(), Value::String(id));
        result.insert(
            "myname".to_string(),
            Value::String(self.capability_name().to_string()),
        );
        result.insert(
            "idcapability".to_string(),
            Value::String(self.capability_name().to_string()),
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
        if self.base().state == 2 || self.base().state == 3 {
            result.insert("endtime".to_string(), Value::String(now));
        }

        // 输入输出 - 需要转换为字符串
        // 先克隆数据，避免借用问题
        let inputjson = self.base().inputjson.clone();
        let configjson = self.base().configjson.clone();
        let description = self.base().description.clone();
        let lastokinfo = self.lifecycle().lastokinfo.clone();
        let lasterrinfo = self.lifecycle().lasterrinfo.clone();

        let inputjson_str = serde_json::to_string(&inputjson).unwrap_or_else(|_| "{}".to_string());
        result.insert("inputjson".to_string(), Value::String(inputjson_str));

        // outputjson 作为模板，保持为空 {}
        result.insert("outputjson".to_string(), Value::String("{}".to_string()));

        let configjson_str =
            serde_json::to_string(&configjson).unwrap_or_else(|_| "{}".to_string());
        result.insert("configjson".to_string(), Value::String(configjson_str));

        let description_str =
            serde_json::to_string(&description).unwrap_or_else(|_| "{}".to_string());
        result.insert("description".to_string(), Value::String(description_str));

        // lastokinfo 和 lasterrinfo 需要转换为字符串
        let lastokinfo_str =
            serde_json::to_string(&lastokinfo).unwrap_or_else(|_| "{}".to_string());
        result.insert("lastokinfo".to_string(), Value::String(lastokinfo_str));

        let lasterrinfo_str =
            serde_json::to_string(&lasterrinfo).unwrap_or_else(|_| "{}".to_string());
        result.insert("lasterrinfo".to_string(), Value::String(lasterrinfo_str));

        // 用户信息
        result.insert("cid".to_string(), Value::String(self.base().cid.clone()));
        result.insert(
            "idagent".to_string(),
            Value::String(self.base().idagent.clone()),
        );
        result.insert(
            "idworkflowinstance".to_string(),
            Value::String(self.base().idworkflowinstance.clone()),
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
            "price".to_string(),
            serde_json::json!(self.economic().price),
        );
        result.insert(
            "costunit".to_string(),
            serde_json::json!(self.economic().costunit),
        );
        result.insert(
            "costtotal".to_string(),
            serde_json::json!(self.economic().costtotal),
        );
        result.insert(
            "revenuetotal".to_string(),
            serde_json::json!(self.economic().revenuetotal),
        );

        result
    }

    /// 使实例可直接调用
    async fn call(&mut self, context78: HashMap<String, Value>) -> HashMap<String, Value> {
        self.execute(context78).await
    }
}

/// 能力基类实现辅助结构
pub struct CapabilityBase {
    pub base: BaseEntity,
    pub lifecycle: LifecycleManager,
    pub economic: EconomicManager,
    pub capability: String,
    pub maxcopy: i32,
    pub timeout: i32,
    pub retrylimit: i32,
    pub retryinterval: i32,
    pub dependencies: Vec<String>,
    /// 是否保存 task 记录到 workflow_task 表，默认 false
    /// 只有关键步骤设为 true，内部辅助步骤保持 false 不写 DB
    pub save_task: bool,
    pub logger: Option<Arc<MyLogger>>,
}

impl Default for CapabilityBase {
    fn default() -> Self {
        Self {
            base: BaseEntity::default(),
            lifecycle: LifecycleManager::default(),
            economic: EconomicManager::default(),
            capability: String::new(),
            maxcopy: 1,
            timeout: 600,
            retrylimit: 3,
            retryinterval: 60,
            dependencies: Vec::new(),
            save_task: false,
            logger: None,
        }
    }
}

impl CapabilityBase {
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

        // 加载控制字段
        if let Some(v) = filtered_data.get("capability").and_then(|v: &serde_json::Value| v.as_str()) {
            base.capability = v.to_string();
        }
        if let Some(v) = filtered_data.get("maxcopy").and_then(|v: &serde_json::Value| v.as_i64()) {
            base.maxcopy = v as i32;
        }
        if let Some(v) = filtered_data.get("timeout").and_then(|v: &serde_json::Value| v.as_i64()) {
            base.timeout = v as i32;
        }
        if let Some(v) = filtered_data.get("retrylimit").and_then(|v: &serde_json::Value| v.as_i64()) {
            base.retrylimit = v as i32;
        }
        if let Some(v) = filtered_data.get("retryinterval").and_then(|v: &serde_json::Value| v.as_i64()) {
            base.retryinterval = v as i32;
        }
        if let Some(v) = filtered_data.get("dependencies").and_then(|v| v.as_array()) {
            base.dependencies = v
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
        }
        if let Some(v) = filtered_data.get("save_task").and_then(|v| v.as_bool()) {
            base.save_task = v;
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
            "capability".to_string(),
            Value::String(self.capability.clone()),
        );
        result.insert("maxcopy".to_string(), Value::Number(self.maxcopy.into()));
        result.insert("timeout".to_string(), Value::Number(self.timeout.into()));
        result.insert(
            "retrylimit".to_string(),
            Value::Number(self.retrylimit.into()),
        );
        result.insert(
            "retryinterval".to_string(),
            Value::Number(self.retryinterval.into()),
        );
        result.insert(
            "dependencies".to_string(),
            serde_json::json!(self.dependencies),
        );
        result.insert(
            "save_task".to_string(),
            Value::Bool(self.save_task),
        );

        result
    }

    /// 保存能力定义到数据库
    ///
    /// 保存到 workflow_capability 表
    pub fn save(&self, workflow_cap: &WorkflowCapability, up: &UpInfo) -> Result<String, String> {
        let json = self.to_json();
        workflow_cap.insert(&json, up)
    }

    /// 从数据库加载能力定义
    ///
    /// 从 workflow_capability 表加载
    pub fn load(workflow_cap: &WorkflowCapability, id: &str, up: &UpInfo) -> Result<Self, String> {
        let data = workflow_cap
            .get(id, up)?
            .ok_or_else(|| format!("能力定义不存在: {}", id))?;
        Ok(Self::from_json(&data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_base_default() {
        let base = CapabilityBase::default();

        assert_eq!(base.maxcopy, 1);
        assert_eq!(base.timeout, 600);
        assert_eq!(base.retrylimit, 3);
        assert_eq!(base.retryinterval, 60);
        assert!(base.dependencies.is_empty());
    }

    #[test]
    fn test_capability_base_from_json() {
        let mut json = HashMap::new();
        json.insert("id".to_string(), Value::String("test-id".to_string()));
        json.insert(
            "capability".to_string(),
            Value::String("test_cap".to_string()),
        );
        json.insert("timeout".to_string(), serde_json::json!(300));
        json.insert("price".to_string(), serde_json::json!(10.0));

        let base = CapabilityBase::from_json(&json);

        assert_eq!(base.base.id, "test-id");
        assert_eq!(base.capability, "test_cap");
        assert_eq!(base.timeout, 300);
        assert_eq!(base.economic.price, 10.0);
    }

    #[test]
    fn test_capability_base_to_json() {
        let mut base = CapabilityBase::default();
        base.base.id = "test-id".to_string();
        base.capability = "test_cap".to_string();
        base.timeout = 300;

        let json = base.to_json();

        assert_eq!(json.get("id").and_then(|v: &serde_json::Value| v.as_str()), Some("test-id"));
        assert_eq!(
            json.get("capability").and_then(|v: &serde_json::Value| v.as_str()),
            Some("test_cap")
        );
        assert_eq!(json.get("timeout").and_then(|v: &serde_json::Value| v.as_i64()), Some(300));
    }
}
