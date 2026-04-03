//! 生命周期管理组件 (LifecycleManager)
//! 管理能力的生命周期和执行统计

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// 生命周期管理组件：管理能力执行的生命周期和统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleManager {
    // 时间管理
    /// 创建时间
    #[serde(skip_serializing_if = "Option::is_none")]
    pub createtime: Option<DateTime<Local>>,
    /// 开始时间
    #[serde(skip_serializing_if = "Option::is_none")]
    pub starttime: Option<DateTime<Local>>,
    /// 结束时间
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endtime: Option<DateTime<Local>>,

    // 执行统计
    /// 运行次数
    #[serde(default)]
    pub runcount: i32,
    /// 成功次数
    #[serde(default)]
    pub successcount: i32,
    /// 错误次数
    #[serde(default)]
    pub errorcount: i32,
    /// 成功率
    #[serde(default)]
    pub successrate: f64,
    /// 执行时间(秒)
    #[serde(default)]
    pub executiontime: f64,

    // 最后记录
    /// 最后运行时间
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lastruntime: Option<DateTime<Local>>,
    /// 最后成功时间
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lastoktime: Option<DateTime<Local>>,
    /// 最后错误时间
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lasterrortime: Option<DateTime<Local>>,
    /// 最后成功信息
    #[serde(default)]
    pub lastokinfo: Value,
    /// 最后错误信息
    #[serde(default)]
    pub lasterrinfo: Value,
}

impl Default for LifecycleManager {
    fn default() -> Self {
        Self {
            createtime: None,
            starttime: None,
            endtime: None,
            runcount: 0,
            successcount: 0,
            errorcount: 0,
            successrate: 0.0,
            executiontime: 0.0,
            lastruntime: None,
            lastoktime: None,
            lasterrortime: None,
            lastokinfo: Value::Object(serde_json::Map::new()),
            lasterrinfo: Value::Object(serde_json::Map::new()),
        }
    }
}

impl LifecycleManager {
    /// 标记开始执行
    pub fn mark_started(&mut self) {
        let now = Local::now();
        self.starttime = Some(now);
        self.runcount += 1;
        self.lastruntime = Some(now);
    }

    /// 记录执行成功
    pub fn record_success(&mut self, execution_time: f64) {
        self.successcount += 1;
        self.executiontime += execution_time;
        self.lastoktime = Some(Local::now());
        self.update_success_rate();
    }

    /// 记录执行错误
    pub fn record_error(&mut self) {
        self.errorcount += 1;
        self.lasterrortime = Some(Local::now());
        self.update_success_rate();
    }

    /// 更新成功率
    fn update_success_rate(&mut self) {
        if self.runcount > 0 {
            self.successrate = (self.successcount as f64 / self.runcount as f64) * 100.0;
        } else {
            self.successrate = 0.0;
        }
    }

    /// 从字典加载数据
    pub fn load_from_dict(&mut self, data: &HashMap<String, Value>) {
        if let Some(v) = data.get("runcount").and_then(|v: &serde_json::Value| v.as_i64()) {
            self.runcount = v as i32;
        }
        if let Some(v) = data.get("successcount").and_then(|v: &serde_json::Value| v.as_i64()) {
            self.successcount = v as i32;
        }
        if let Some(v) = data.get("errorcount").and_then(|v: &serde_json::Value| v.as_i64()) {
            self.errorcount = v as i32;
        }
        if let Some(v) = data.get("successrate").and_then(|v| v.as_f64()) {
            self.successrate = v;
        }
        if let Some(v) = data.get("executiontime").and_then(|v| v.as_f64()) {
            self.executiontime = v;
        }
        if let Some(v) = data.get("lastokinfo").cloned() {
            self.lastokinfo = v;
        }
        if let Some(v) = data.get("lasterrinfo").cloned() {
            self.lasterrinfo = v;
        }
    }

    /// 转换为字典
    pub fn to_dict(&self) -> HashMap<String, Value> {
        let mut map = HashMap::new();
        if let Some(t) = &self.createtime {
            map.insert("createtime".to_string(), Value::String(t.to_rfc3339()));
        }
        if let Some(t) = &self.starttime {
            map.insert("starttime".to_string(), Value::String(t.to_rfc3339()));
        }
        if let Some(t) = &self.endtime {
            map.insert("endtime".to_string(), Value::String(t.to_rfc3339()));
        }
        map.insert("runcount".to_string(), Value::Number(self.runcount.into()));
        map.insert("successcount".to_string(), Value::Number(self.successcount.into()));
        map.insert("errorcount".to_string(), Value::Number(self.errorcount.into()));
        map.insert("successrate".to_string(), serde_json::json!(self.successrate));
        map.insert("executiontime".to_string(), serde_json::json!(self.executiontime));
        if let Some(t) = &self.lastruntime {
            map.insert("lastruntime".to_string(), Value::String(t.to_rfc3339()));
        }
        if let Some(t) = &self.lastoktime {
            map.insert("lastoktime".to_string(), Value::String(t.to_rfc3339()));
        }
        if let Some(t) = &self.lasterrortime {
            map.insert("lasterrortime".to_string(), Value::String(t.to_rfc3339()));
        }
        map.insert("lastokinfo".to_string(), self.lastokinfo.clone());
        map.insert("lasterrinfo".to_string(), self.lasterrinfo.clone());
        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试1：默认创建
    /// 验证：runcount=0, successcount=0, errorcount=0, successrate=0.0
    #[test]
    fn test_lifecycle_manager_default() {
        let manager = LifecycleManager::default();

        assert_eq!(manager.runcount, 0);
        assert_eq!(manager.successcount, 0);
        assert_eq!(manager.errorcount, 0);
        assert_eq!(manager.successrate, 0.0);
        assert_eq!(manager.executiontime, 0.0);
        assert!(manager.createtime.is_none());
        assert!(manager.starttime.is_none());
    }

    /// 测试2：执行流程
    /// 验证：runcount=1, successcount=1, executiontime=1.5, successrate=100.0
    #[test]
    fn test_lifecycle_manager_success_flow() {
        let mut manager = LifecycleManager::default();

        manager.mark_started();
        manager.record_success(1.5);

        assert_eq!(manager.runcount, 1);
        assert_eq!(manager.successcount, 1);
        assert_eq!(manager.executiontime, 1.5);
        assert_eq!(manager.successrate, 100.0);
        assert!(manager.starttime.is_some());
        assert!(manager.lastoktime.is_some());
    }

    /// 测试3：错误记录流程
    #[test]
    fn test_lifecycle_manager_error_flow() {
        let mut manager = LifecycleManager::default();

        manager.mark_started();
        manager.record_error();

        assert_eq!(manager.runcount, 1);
        assert_eq!(manager.successcount, 0);
        assert_eq!(manager.errorcount, 1);
        assert_eq!(manager.successrate, 0.0);
        assert!(manager.lasterrortime.is_some());
    }

    /// 测试4：混合执行场景
    #[test]
    fn test_lifecycle_manager_mixed_execution() {
        let mut manager = LifecycleManager::default();

        // 第一次成功
        manager.mark_started();
        manager.record_success(1.0);

        // 第二次失败
        manager.mark_started();
        manager.record_error();

        // 第三次成功
        manager.mark_started();
        manager.record_success(2.0);

        assert_eq!(manager.runcount, 3);
        assert_eq!(manager.successcount, 2);
        assert_eq!(manager.errorcount, 1);
        assert_eq!(manager.executiontime, 3.0);
        // 成功率 = 2/3 * 100 = 66.66...
        assert!((manager.successrate - 66.66666666666666).abs() < 0.01);
    }

    /// 测试5：字典加载和转换
    #[test]
    fn test_lifecycle_manager_dict_operations() {
        let mut manager = LifecycleManager::default();
        manager.runcount = 10;
        manager.successcount = 8;
        manager.errorcount = 2;
        manager.successrate = 80.0;
        manager.executiontime = 15.5;

        let dict = manager.to_dict();

        assert_eq!(dict.get("runcount").and_then(|v: &serde_json::Value| v.as_i64()), Some(10));
        assert_eq!(dict.get("successcount").and_then(|v: &serde_json::Value| v.as_i64()), Some(8));
        assert_eq!(dict.get("errorcount").and_then(|v: &serde_json::Value| v.as_i64()), Some(2));

        let mut loaded = LifecycleManager::default();
        loaded.load_from_dict(&dict);

        assert_eq!(loaded.runcount, 10);
        assert_eq!(loaded.successcount, 8);
        assert_eq!(loaded.errorcount, 2);
    }

    /// 测试6：lastokinfo 和 lasterrinfo 处理
    #[test]
    fn test_lifecycle_manager_info_fields() {
        let mut manager = LifecycleManager::default();
        let ok_info = serde_json::json!({"result": "success", "data": "test"});
        let err_info = serde_json::json!({"error": "test error"});

        manager.lastokinfo = ok_info.clone();
        manager.lasterrinfo = err_info.clone();

        let dict = manager.to_dict();

        assert_eq!(dict.get("lastokinfo"), Some(&ok_info));
        assert_eq!(dict.get("lasterrinfo"), Some(&err_info));
    }
}