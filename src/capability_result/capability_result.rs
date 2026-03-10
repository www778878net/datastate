//! CapabilityResult - 能力执行结果类
//!
//! 使用结构体定义标准返回结构，提供类型安全

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 能力执行结果类
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityResult {
    /// 执行结果: 0=成功, 其他=失败
    pub res: i32,
    /// 错误信息
    #[serde(default)]
    pub errmsg: String,
    /// 业务结果
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    /// 操作描述
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation: Option<String>,
}

impl CapabilityResult {
    /// 创建新的结果
    pub fn new(res: i32, errmsg: String, result: Option<Value>, operation: Option<String>) -> Self {
        Self { res, errmsg, result, operation }
    }

    /// 转换为字典 (JSON Value)
    pub fn to_dict(&self) -> Value {
        serde_json::to_value(self).unwrap_or(Value::Null)
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

impl Default for CapabilityResult {
    fn default() -> Self {
        Self {
            res: 0,
            errmsg: String::new(),
            result: None,
            operation: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_default() {
        let result = CapabilityResult::default();
        assert_eq!(result.res, 0);
        assert!(result.errmsg.is_empty());
        assert!(result.result.is_none());
    }

    #[test]
    fn test_success() {
        let result = CapabilityResult::success(
            Some(json!({"key": "value"})),
            Some("操作成功".to_string())
        );

        assert!(result.is_success());
        assert_eq!(result.res, 0);
        assert!(result.result.is_some());
    }

    #[test]
    fn test_failure() {
        let result = CapabilityResult::failure("操作失败", -1, Some("test_op".to_string()));

        assert!(!result.is_success());
        assert_eq!(result.res, -1);
        assert_eq!(result.errmsg, "操作失败");
    }

    #[test]
    fn test_to_dict() {
        let result = CapabilityResult::success(
            Some(json!({"key": "value"})),
            Some("op".to_string())
        );

        let dict = result.to_dict();
        assert_eq!(dict.get("res").and_then(|v| v.as_i64()).unwrap_or(-1), 0);
    }
}