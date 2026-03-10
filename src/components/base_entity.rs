//! 基础实体组件 (BaseEntity)
//! 职责：负责身份标识、接口定义和基本属性
//! 第一性原理：身份是存在的根本，接口是交互的契约

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// 基础实体组件：身份标识、能力定义和搜索索引
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseEntity {
    // 身份标识
    /// 全局唯一ID
    #[serde(default)]
    pub id: String,
    /// 自增主键
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idpk: Option<i64>,
    /// 公司标识
    #[serde(default)]
    pub cid: String,
    /// 状态: 0=待执行, 1=运行中, 2=已完成, 3=失败, 4=已取消, 5=暂停, 6=警告(完成但有警告)
    #[serde(default = "default_state")]
    pub state: i32,
    /// 优先级
    #[serde(default)]
    pub priority: i32,
    /// 实体名称
    #[serde(default)]
    pub myname: String,
    /// 工作流实例ID
    #[serde(default)]
    pub idworkflowinstance: String,

    // 能力接口
    /// 输入接口定义
    #[serde(default)]
    pub inputjson: Value,
    /// 输出接口定义
    #[serde(default)]
    pub outputjson: Value,
    /// 能力描述
    #[serde(default)]
    pub description: Value,
    /// 配置信息
    #[serde(default)]
    pub configjson: Value,
    /// 资源需求
    #[serde(default)]
    pub resourcereq: Value,
    /// 透传的原始输入
    #[serde(default)]
    pub preinputjson: Value,

    // 分类索引（搜索字段）
    /// 多个微服务组成（大的系统）
    #[serde(default = "default_apisys")]
    pub apisys: String,
    /// 系统/目录（微服务）
    #[serde(default)]
    pub apimicro: String,
    /// 对象/类（能力或业务对象）
    #[serde(default)]
    pub apiobj: String,
    /// 代理标识符
    #[serde(default)]
    pub idagent: String,

    // SID相关字段
    /// 会话ID
    #[serde(default)]
    pub sid: String,
    /// 用户名
    #[serde(default)]
    pub uname: String,
    /// 用户ID
    #[serde(default)]
    pub uid: String,
    /// 余额信息
    #[serde(default)]
    pub money78: String,
    /// 消费信息
    #[serde(default)]
    pub consume: String,
    /// 公司名称
    #[serde(default)]
    pub coname: String,
}

fn default_state() -> i32 { 1 }
fn default_apisys() -> String { "apitmp".to_string() }

impl Default for BaseEntity {
    fn default() -> Self {
        Self {
            id: String::new(),
            idpk: None,
            cid: String::new(),
            state: 1,
            priority: 0,
            myname: String::new(),
            idworkflowinstance: String::new(),
            inputjson: Value::Object(serde_json::Map::new()),
            outputjson: Value::Object(serde_json::Map::new()),
            description: Value::Object(serde_json::Map::new()),
            configjson: Value::Object(serde_json::Map::new()),
            resourcereq: Value::Object(serde_json::Map::new()),
            preinputjson: Value::Object(serde_json::Map::new()),
            apisys: "apitmp".to_string(),
            apimicro: String::new(),
            apiobj: String::new(),
            idagent: String::new(),
            sid: String::new(),
            uname: String::new(),
            uid: String::new(),
            money78: String::new(),
            consume: String::new(),
            coname: String::new(),
        }
    }
}

impl BaseEntity {
    /// 从字典加载数据
    pub fn load_from_dict(&mut self, data: &HashMap<String, Value>) {
        if let Some(v) = data.get("id").and_then(|v| v.as_str()) {
            self.id = v.to_string();
        }
        if let Some(v) = data.get("idpk").and_then(|v| v.as_i64()) {
            self.idpk = Some(v);
        }
        if let Some(v) = data.get("cid").and_then(|v| v.as_str()) {
            self.cid = v.to_string();
        }
        if let Some(v) = data.get("state").and_then(|v| v.as_i64()) {
            self.state = v as i32;
        }
        if let Some(v) = data.get("priority").and_then(|v| v.as_i64()) {
            self.priority = v as i32;
        }
        if let Some(v) = data.get("myname").and_then(|v| v.as_str()) {
            self.myname = v.to_string();
        }
        if let Some(v) = data.get("idworkflowinstance").and_then(|v| v.as_str()) {
            self.idworkflowinstance = v.to_string();
        }
        if let Some(v) = data.get("inputjson").cloned() {
            self.inputjson = v;
        }
        if let Some(v) = data.get("outputjson").cloned() {
            self.outputjson = v;
        }
        if let Some(v) = data.get("description").cloned() {
            self.description = v;
        }
        if let Some(v) = data.get("configjson").cloned() {
            self.configjson = v;
        }
        if let Some(v) = data.get("resourcereq").cloned() {
            self.resourcereq = v;
        }
        if let Some(v) = data.get("preinputjson").cloned() {
            self.preinputjson = v;
        }
        if let Some(v) = data.get("apisys").and_then(|v| v.as_str()) {
            self.apisys = v.to_string();
        }
        if let Some(v) = data.get("apimicro").and_then(|v| v.as_str()) {
            self.apimicro = v.to_string();
        }
        if let Some(v) = data.get("apiobj").and_then(|v| v.as_str()) {
            self.apiobj = v.to_string();
        }
        if let Some(v) = data.get("idagent").and_then(|v| v.as_str()) {
            self.idagent = v.to_string();
        }
        if let Some(v) = data.get("sid").and_then(|v| v.as_str()) {
            self.sid = v.to_string();
        }
        if let Some(v) = data.get("uname").and_then(|v| v.as_str()) {
            self.uname = v.to_string();
        }
        if let Some(v) = data.get("uid").and_then(|v| v.as_str()) {
            self.uid = v.to_string();
        }
        if let Some(v) = data.get("money78").and_then(|v| v.as_str()) {
            self.money78 = v.to_string();
        }
        if let Some(v) = data.get("consume").and_then(|v| v.as_str()) {
            self.consume = v.to_string();
        }
        if let Some(v) = data.get("coname").and_then(|v| v.as_str()) {
            self.coname = v.to_string();
        }
    }

    /// 转换为字典
    pub fn to_dict(&self) -> HashMap<String, Value> {
        let mut map = HashMap::new();
        map.insert("id".to_string(), Value::String(self.id.clone()));
        if let Some(idpk) = self.idpk {
            map.insert("idpk".to_string(), Value::Number(idpk.into()));
        }
        map.insert("cid".to_string(), Value::String(self.cid.clone()));
        map.insert("state".to_string(), Value::Number(self.state.into()));
        map.insert("priority".to_string(), Value::Number(self.priority.into()));
        map.insert("myname".to_string(), Value::String(self.myname.clone()));
        map.insert("idworkflowinstance".to_string(), Value::String(self.idworkflowinstance.clone()));
        map.insert("inputjson".to_string(), self.inputjson.clone());
        map.insert("outputjson".to_string(), self.outputjson.clone());
        map.insert("description".to_string(), self.description.clone());
        map.insert("configjson".to_string(), self.configjson.clone());
        map.insert("resourcereq".to_string(), self.resourcereq.clone());
        map.insert("preinputjson".to_string(), self.preinputjson.clone());
        map.insert("apisys".to_string(), Value::String(self.apisys.clone()));
        map.insert("apimicro".to_string(), Value::String(self.apimicro.clone()));
        map.insert("apiobj".to_string(), Value::String(self.apiobj.clone()));
        map.insert("idagent".to_string(), Value::String(self.idagent.clone()));
        map.insert("sid".to_string(), Value::String(self.sid.clone()));
        map.insert("uname".to_string(), Value::String(self.uname.clone()));
        map.insert("uid".to_string(), Value::String(self.uid.clone()));
        map.insert("money78".to_string(), Value::String(self.money78.clone()));
        map.insert("consume".to_string(), Value::String(self.consume.clone()));
        map.insert("coname".to_string(), Value::String(self.coname.clone()));
        map
    }
}