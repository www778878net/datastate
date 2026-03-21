//! Database - SQLite78、MySQL78 数据库封装 + Workflow 工作流
//!
//! ## 数据库模块
//! - sqlite78: SQLite 本地数据库操作（⚠️ 内部使用，请通过 DataState 访问）
//! - mysql78: MySQL 数据库操作（连接池、重试、事务）
//! - shared: 共享数据结构
//! - localdb: LocalDB 本地数据库封装（⚠️ 内部使用，禁止外部访问）
//! - datastate: 数据状态机
//! - datamanage: 数据管理器
//! - sync_config: 同步配置
//! - state: 状态基类
//! - schema: 数据表结构定义
//! - query_builder: SQL 查询构建器
//! - snowflake: 雪花算法ID生成器
//!
//! ## 工作流模块
//! - capability: 能力基类
//! - instance: 实例基类
//! - capability_result: 能力结果
//! - storage: 工作流存储
//! - components: 组件（实体、经济、生命周期）
//!
//! ## 访问控制设计
//!
//! LocalDB 使用 pub(crate) 限制，只允许 database crate 内部访问。

/// 包名（编译时固定，用于权限验证）
pub const PKG_NAME: &str = env!("CARGO_PKG_NAME");

// ============ 数据库模块 ============
pub mod mysql78;

/// ⚠️ sqlite78 - 内部模块，禁止外部访问
pub(crate) mod sqlite78;

pub mod shared;

/// ⚠️ localdb - 内部模块，禁止外部访问
pub(crate) mod localdb;
pub mod datastate;
pub mod dataaudit;
pub mod datamanage;
pub mod sync_config;
pub mod state;
pub mod data_sync;
pub mod schema;
pub mod query_builder;
pub mod table_config;
pub mod config;

/// 雪花算法ID生成器
pub mod snowflake;

// ============ 工作流模块 ============
pub mod capability_result;
mod components;
pub mod capability;
pub mod instance;
pub mod workflow;

// ============ 数据库导出 ============

// sqlite78 导出（⚠️ 内部使用）
pub use sqlite78::{Sqlite78, InsertResult, UpdateResult, WarnHandler};

// sqlite78 状态类导出
pub use sqlite78::{
    SysWarnSqliteState, SYS_WARN_TABLE as SYS_WARN_TABLE_SQLITE, SYS_WARN_CREATE_SQL as SYS_WARN_CREATE_SQL_SQLITE,
    SysSqlSqliteState, SYS_SQL_TABLE as SYS_SQL_TABLE_SQLITE, SYS_SQL_CREATE_SQL as SYS_SQL_CREATE_SQL_SQLITE,
};

// shared 导出
pub use shared::{SysWarnData, SysSqlData};

// 重导出 base::UpInfo
pub use base::UpInfo;

// mysql78 导出
pub use mysql78::{
    Mysql78, MysqlConfig, MysqlUpInfo, MysqlInsertResult, MysqlUpdateResult,
};

// mysql78 状态类导出
pub use mysql78::{
    SysWarnMysqlState, SYS_WARN_TABLE as SYS_WARN_TABLE_MYSQL, SYS_WARN_CREATE_SQL as SYS_WARN_CREATE_SQL_MYSQL,
    SysSqlMysqlState, SYS_SQL_TABLE as SYS_SQL_TABLE_MYSQL, SYS_SQL_CREATE_SQL as SYS_SQL_CREATE_SQL_MYSQL,
};

// localdb 导出（临时兼容）
pub use localdb::LocalDB;

// snowflake 导出
pub use snowflake::{next_id, next_id_string};

// datastate 导出（包含权限相关方法和内部访问trait）
pub use datastate::{
    DataState, DataSync, SynclogItem, SYNCLOG_CREATE_SQL,
    DATA_STATE_LOG_CREATE_SQL, DATA_SYNC_STATS_CREATE_SQL,
    StateLog, SyncStats,
    // MySQL 版本
    DataStateMysql, DataSyncMysql, SynclogItemMysql, SyncStatsMysql, StateLogMysql,
    SyncResultMysql, SyncDataMysql,
    SYNCLOG_CREATE_SQL_MYSQL, DATA_STATE_LOG_CREATE_SQL_MYSQL, DATA_SYNC_STATS_CREATE_SQL_MYSQL,
    // 审计相关
    DataAudit, DATA_ABILITY_LOG_CREATE_SQL, AbilityLog,
    AuditLogDataState, AuditLogRecord, AUDIT_LOG_CREATE_SQL,
    AuditPermDataState, AuditPermRecord, AUDIT_PERM_CREATE_SQL,
};

// state 导出
pub use state::BaseState;

// datamanage 导出
pub use datamanage::DataManage;

// sync_config 导出
pub use sync_config::{IndexDef, TableConfig, get_system_columns};

// workflow 导出
pub use workflow::{
    WorkflowCapability, WorkflowTask, WorkflowInstance,
    SQL_CREATE_WORKFLOW_CAPABILITY, SQL_CREATE_WORKFLOW_TASK, SQL_CREATE_WORKFLOW_INSTANCE,
    init_workflow_tables, init_workflow_tables_with_default_path,
    ShardingConfig, ShardType, ShardingManager, MaintenanceResult,
};

// capability 导出
pub use capability::{BaseCapability, CapabilityBase};

// capability_result 导出
pub use capability_result::CapabilityResult;

// instance 导出
pub use instance::{BaseInstance, InstanceBase, InstanceResult};

// components 导出
pub use components::{BaseEntity, LifecycleManager, EconomicManager};
