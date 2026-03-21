//! DataStateMysql - MySQL 版本数据状态类
//!
//! 与 DataState 功能相同，但使用 MySQL 作为后端
//!
//! 组合组件：
//! - DataSyncMysql: 同步队列管理
//! - DataAudit: 权限检查和审计日志

use crate::data_sync::DataSyncMysql;
use crate::dataaudit::DataAudit;
use crate::mysql78::{Mysql78, MysqlConfig};
use crate::state::BaseState;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// DataStateMysql - MySQL 版本数据状态类
///
/// 使用 MySQL 作为数据库后端，管理数据库表的同步状态
#[derive(Clone, Serialize, Deserialize)]
pub struct DataStateMysql {
    /// 基础状态
    #[serde(flatten)]
    pub base: BaseState,

    /// 同步组件（包含数据库实例）
    #[serde(skip)]
    pub datasync: DataSyncMysql,

    /// 审计组件（权限检查和日志记录）
    #[serde(skip)]
    pub audit: DataAudit,
}

impl std::fmt::Debug for DataStateMysql {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DataStateMysql")
            .field("base", &self.base)
            .field("datasync", &self.datasync)
            .field("audit", &self.audit)
            .finish()
    }
}

impl DataStateMysql {
    /// 使用 MySQL 配置创建 DataStateMysql
    pub fn with_config(table_name: &str, mysql_config: MysqlConfig) -> Result<Self, String> {
        let datasync = DataSyncMysql::with_config(table_name, mysql_config)?;
        Ok(Self {
            base: BaseState::new(table_name),
            datasync,
            audit: DataAudit::new(table_name),
        })
    }

    /// 使用已有的 Mysql78 实例创建 DataStateMysql
    pub fn with_db(table_name: &str, db: Mysql78) -> Self {
        Self {
            base: BaseState::new(table_name),
            datasync: DataSyncMysql::new(table_name, db),
            audit: DataAudit::new(table_name),
        }
    }

    // ========== CRUD 方法 ==========

    /// 插入记录
    /// - 权限检查：验证caller是否有权限调用此方法
    /// - 审计日志：通过log_action_with_count记录操作摘要
    /// - 自动写 sync_queue：产生待同步记录
    /// - 自动设置 id、cid、upby、uptime
    pub fn m_add(&self, record: &HashMap<String, Value>, caller: &str, summary: &str) -> Result<String, String> {
        self.audit.check_permission("m_add", caller, summary)?;
        self.datasync.m_add(record)
    }

    /// 更新记录
    /// - 权限检查：验证caller是否有权限调用此方法
    /// - 审计日志：通过log_action_with_count记录操作摘要
    /// - 自动写 sync_queue：产生待同步记录
    /// - 自动设置 upby、uptime
    pub fn m_update(&self, id: &str, record: &HashMap<String, Value>, caller: &str, summary: &str) -> Result<bool, String> {
        self.audit.check_permission("m_update", caller, summary)?;
        self.datasync.m_update(id, record)
    }

    /// 保存记录（存在更新，不存在插入）
    /// - 权限检查：验证caller是否有权限调用此方法
    /// - 审计日志：通过log_action_with_count记录操作摘要
    /// - 自动写 sync_queue：产生待同步记录
    pub fn m_save(&self, record: &HashMap<String, Value>, caller: &str, summary: &str) -> Result<String, String> {
        self.audit.check_permission("m_save", caller, summary)?;
        self.datasync.m_save(record)
    }

    /// 删除记录
    /// - 权限检查：验证caller是否有权限调用此方法
    /// - 审计日志：通过log_action_with_count记录操作摘要
    /// - 自动写 sync_queue：产生待同步记录
    pub fn m_del(&self, id: &str, caller: &str, summary: &str) -> Result<bool, String> {
        self.audit.check_permission("m_del", caller, summary)?;
        self.datasync.m_del(id)
    }

    /// 同步保存记录（存在更新，不存在插入）
    /// - 用于从服务器同步数据到本地，或从客户端同步数据到服务器
    /// - 不自动填充 CID、upby、uptime
    /// - 不写 sync_queue（避免循环同步）
    pub fn m_sync_save(&self, record: &HashMap<String, Value>) -> Result<String, String> {
        self.datasync.m_sync_save(record)
    }

    /// 同步更新记录
    /// - 用于从服务器同步更新操作到本地
    /// - 不自动填充字段，不写 sync_queue
    pub fn m_sync_update(&self, id: &str, record: &HashMap<String, Value>) -> Result<bool, String> {
        self.datasync.m_sync_update(id, record)
    }

    /// 同步删除记录
    /// - 用于从服务器同步删除操作到本地
    /// - 不写 sync_queue（避免循环同步）
    pub fn m_sync_del(&self, id: &str) -> Result<bool, String> {
        self.datasync.m_sync_del(id)
    }

    /// 查询记录
    /// - 权限检查：验证caller是否有权限调用此方法
    /// - 审计日志：通过log_action_with_count记录操作摘要
    pub fn get(&self, where_clause: &str, params: Vec<Value>, caller: &str, summary: &str) -> Result<Vec<HashMap<String, Value>>, String> {
        self.audit.check_permission("get", caller, summary)?;
        self.datasync.get(where_clause, params)
    }

    /// 查询单条记录
    /// - 权限检查：验证caller是否有权限调用此方法
    /// - 审计日志：通过log_action_with_count记录操作摘要
    pub fn get_one(&self, id: &str, caller: &str, summary: &str) -> Result<Option<HashMap<String, Value>>, String> {
        self.audit.check_permission("get_one", caller, summary)?;
        self.datasync.get_one(id)
    }

    /// 统计记录数
    /// - 权限检查：验证caller是否有权限调用此方法
    /// - 审计日志：通过log_action_with_count记录操作摘要
    pub fn count(&self, caller: &str, summary: &str) -> Result<i32, String> {
        self.audit.check_permission("count", caller, summary)?;
        self.datasync.count()
    }

    /// 执行任意 SQL 查询（支持完整 SQL 拼接）
    /// - 权限检查：验证caller是否有权限调用此方法
    /// - 审计日志：通过log_action_with_count记录操作摘要
    pub fn do_get(&self, sql: &str, params: Vec<Value>, caller: &str, summary: &str) -> Result<Vec<HashMap<String, Value>>, String> {
        self.audit.check_permission("do_get", caller, summary)?;
        self.datasync.do_get(sql, params)
    }

    /// 执行任意 SQL 更新（支持完整 SQL 拼接）
    /// - 权限检查：验证caller是否有权限调用此方法
    /// - 审计日志：通过log_action_with_count记录操作摘要
    /// - 返回影响的行数
    pub fn do_m(&self, sql: &str, params: Vec<Value>, caller: &str, summary: &str) -> Result<usize, String> {
        self.audit.check_permission("do_m", caller, summary)?;
        self.datasync.do_m(sql, params)
    }

    /// 初始化同步相关表
    pub fn init_tables(&self) -> Result<(), String> {
        self.datasync.init_tables()
    }
}

impl Default for DataStateMysql {
    fn default() -> Self {
        Self {
            base: BaseState::new(""),
            datasync: DataSyncMysql::default(),
            audit: DataAudit::new(""),
        }
    }
}
