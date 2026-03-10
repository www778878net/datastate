//! 组件模块
//!
//! 包含能力基类的核心组件

mod base_entity;
mod lifecycle_manager;
mod economic_manager;

pub use base_entity::BaseEntity;
pub use lifecycle_manager::LifecycleManager;
pub use economic_manager::EconomicManager;