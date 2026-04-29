//! Storage - 工作流存储层
//!
//! ## 表关系说明
//! - workflow_capability: 能力【定义表】- 存储能力模板、配置、价格等静态定义
//! - workflow_task: 任务【实例表】- 每次能力执行产生的任务实例记录
//! - workflow_instance: 工作流【实例表】- 工作流执行实例
//!
//! 包含分表管理和工作流相关的数据表

mod sharding;
mod workflow_capability;
mod workflow_instance;
mod workflow_task;

pub use sharding::{ShardingConfig, ShardType, ShardingManager, MaintenanceResult};
pub use workflow_capability::{WorkflowCapability, SQL_CREATE_WORKFLOW_CAPABILITY};
pub use workflow_instance::{WorkflowInstance, SQL_CREATE_WORKFLOW_INSTANCE};
pub use workflow_task::{WorkflowTask, SQL_CREATE_WORKFLOW_TASK};

use crate::{Sqlite78, UpInfo};

/// 初始化工作流表（在正式数据库中创建表）
///
/// 创建 workflow_capability 表（固定表）
/// workflow_instance 和 workflow_task 是分表，会在首次使用时自动创建
pub async fn init_workflow_tables(db: &mut Sqlite78, _up: &UpInfo) -> Result<String, String> {
    db.initialize()?;

    let conn = db.get_conn()?;
    let conn = conn.lock().await;

    // 创建 workflow_capability 表（固定表，不分表）
    conn.execute(SQL_CREATE_WORKFLOW_CAPABILITY, [])
        .map_err(|e| format!("创建 workflow_capability 表失败: {}", e))?;

    Ok("ok".to_string())
}

/// 使用默认数据库路径初始化工作流表
pub async fn init_workflow_tables_with_default_path() -> Result<String, String> {
    let mut db = Sqlite78::with_default_path();
    let up = UpInfo::new();
    init_workflow_tables(&mut db, &up).await
}