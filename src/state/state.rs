//! State - 状态基类模块
//!
//! 包含 StateStatus 枚举和 BaseState 基类

use serde::{Deserialize, Serialize};

/// 状态常量（3种状态）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum StateStatus {
    /// 空闲
    IDLE = 0,
    /// 工作中
    WORKING = 1,
    /// 错误
    ERROR = 2,
}

impl Default for StateStatus {
    fn default() -> Self {
        Self::IDLE
    }
}

impl StateStatus {
    /// 获取状态名称
    pub fn name(&self) -> &'static str {
        match self {
            StateStatus::IDLE => "IDLE",
            StateStatus::WORKING => "WORKING",
            StateStatus::ERROR => "ERROR",
        }
    }

    /// 从整数转换
    pub fn from_i32(value: i32) -> Self {
        match value {
            0 => StateStatus::IDLE,
            1 => StateStatus::WORKING,
            2 => StateStatus::ERROR,
            _ => StateStatus::IDLE,
        }
    }
}

/// 基类状态
///
/// 所有状态类的基类，只包含最基础的字段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseState {
    /// 实例名称（唯一标识）
    pub name: String,
    /// 状态值
    #[serde(default)]
    pub status: StateStatus,
}

impl BaseState {
    /// 创建新的状态实例
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: StateStatus::IDLE,
        }
    }

    /// 检查是否空闲
    pub fn is_idle(&self) -> bool {
        self.status == StateStatus::IDLE
    }

    /// 检查是否工作中
    pub fn is_working(&self) -> bool {
        self.status == StateStatus::WORKING
    }

    /// 检查是否错误
    pub fn is_error(&self) -> bool {
        self.status == StateStatus::ERROR
    }

    /// 获取状态名称
    pub fn get_status_name(&self) -> &'static str {
        self.status.name()
    }

    /// 设置状态为工作中
    pub fn set_working(&mut self) {
        self.status = StateStatus::WORKING;
    }

    /// 设置状态为空闲
    pub fn set_idle(&mut self) {
        self.status = StateStatus::IDLE;
    }

    /// 设置状态为错误
    pub fn set_error(&mut self) {
        self.status = StateStatus::ERROR;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_status() {
        let state = BaseState::new("test");
        assert!(state.is_idle(), "默认状态应该是 IDLE");
        assert!(!state.is_working());
        assert!(!state.is_error());
    }

    #[test]
    fn test_status_name() {
        assert_eq!(StateStatus::IDLE.name(), "IDLE");
        assert_eq!(StateStatus::WORKING.name(), "WORKING");
        assert_eq!(StateStatus::ERROR.name(), "ERROR");
    }

    #[test]
    fn test_status_from_i32() {
        assert_eq!(StateStatus::from_i32(0), StateStatus::IDLE);
        assert_eq!(StateStatus::from_i32(1), StateStatus::WORKING);
        assert_eq!(StateStatus::from_i32(2), StateStatus::ERROR);
        assert_eq!(StateStatus::from_i32(99), StateStatus::IDLE);
    }

    #[test]
    fn test_state_transitions() {
        let mut state = BaseState::new("test");

        state.set_working();
        assert!(state.is_working());
        assert!(!state.is_idle());

        state.set_idle();
        assert!(state.is_idle());
        assert!(!state.is_working());

        state.set_error();
        assert!(state.is_error());
        assert!(!state.is_idle());
    }

    #[test]
    fn test_get_status_name() {
        let mut state = BaseState::new("test");
        assert_eq!(state.get_status_name(), "IDLE");

        state.set_working();
        assert_eq!(state.get_status_name(), "WORKING");

        state.set_error();
        assert_eq!(state.get_status_name(), "ERROR");
    }

    #[test]
    fn test_serialize() {
        let state = BaseState::new("test_instance");
        let json = serde_json::to_string(&state).unwrap();
        assert!(json.contains("test_instance"));
        // StateStatus serializes as integer
        assert!(json.contains("IDLE") || json.contains("0"));
    }

    #[test]
    fn test_deserialize() {
        let json = r#"{"name":"deser_test","status":"WORKING"}"#;
        let state: BaseState = serde_json::from_str(json).unwrap();
        assert_eq!(state.name, "deser_test");
        assert!(state.is_working());
    }
}