//! SyncConfig - 同步配置模块
//!
//! 定义表同步策略和配置

mod sync_config;

pub use sync_config::{IndexDef, SyncPolicy, TableConfig, get_system_columns};