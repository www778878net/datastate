//! Config - 配置管理模块
//!
//! 单例模式，支持 JSON 文件加载

mod config;

pub use config::{Config, ConfigError};
