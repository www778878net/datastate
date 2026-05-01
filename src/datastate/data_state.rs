//! DataState - 通用数据状态类
//!
//! 继承 BaseState，用于管理数据库表的同步状态
//!
//! 组合组件：
//! - DataSync: 同步队列管理
//! - DataAudit: 权限检查和审计日志

use crate::data_sync::DataSync;
use crate::dataaudit::DataAudit;
use crate::state::BaseState;
use crate::sync_config::TableConfig;
use crate::localdb::LocalDB;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use serde_json::Value;

/// DataState - 通用数据状态类
///
/// 继承 BaseState，管理数据库表的同步状态
#[derive(Clone, Serialize, Deserialize)]
pub struct DataState {
    /// 基础状态
    #[serde(flatten)]
    pub base: BaseState,

    /// 同步组件（包含数据库实例）
    #[serde(skip)]
    pub datasync: DataSync,

    /// 审计组件（权限检查和日志记录）
    #[serde(skip)]
    pub audit: DataAudit,
}

impl std::fmt::Debug for DataState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DataState")
            .field("base", &self.base)
            .field("datasync", &self.datasync)
            .field("audit", &self.audit)
            .finish()
    }
}

impl DataState {
    /// 从 TableConfig 创建 DataState
    pub fn from_config(config: &TableConfig) -> Self {
        Self {
            base: BaseState::new(&config.name),
            datasync: DataSync::from_config(config),
            audit: DataAudit::new(&config.name),
        }
    }

    /// 使用指定的数据库实例创建 DataState
    pub fn with_db(table_name: &str, db: LocalDB) -> Self {
        Self {
            base: BaseState::new(table_name),
            datasync: DataSync::with_db(table_name, db),
            audit: DataAudit::new(table_name),
        }
    }

    // ========== CRUD 代理方法（带权限检查和审计日志） ==========

    /// 插入记录
    /// - 权限检查：验证caller是否有权限调用此方法
    /// - 审计日志：通过log_action_with_count记录操作摘要
    /// - 自动写 sync_queue：产生待同步记录
    /// - 自动设置 id、cid、upby、uptime
    pub async fn m_add(&self, record: &HashMap<String, Value>, caller: &str, summary: &str) -> Result<String, String> {
        self.audit.check_permission("m_add", caller, summary).await?;
        self.datasync.m_add(record).await
    }

    /// 更新记录
    /// - 权限检查：验证caller是否有权限调用此方法
    /// - 审计日志：通过log_action_with_count记录操作摘要
    /// - 自动写 sync_queue：产生待同步记录
    /// - 自动设置 upby、uptime
    pub async fn m_update(&self, id: &str, record: &HashMap<String, Value>, caller: &str, summary: &str) -> Result<bool, String> {
        self.audit.check_permission("m_update", caller, summary).await?;
        self.datasync.m_update(id, record).await
    }

    /// 保存记录（存在更新，不存在插入）
    /// - 权限检查：验证caller是否有权限调用此方法
    /// - 审计日志：通过log_action_with_count记录操作摘要
    /// - 自动写 sync_queue：产生待同步记录
    pub async fn m_save(&self, record: &HashMap<String, Value>, caller: &str, summary: &str) -> Result<String, String> {
        self.audit.check_permission("m_save", caller, summary).await?;
        self.datasync.m_save(record).await
    }

    /// 删除记录
    /// - 权限检查：验证caller是否有权限调用此方法
    /// - 审计日志：通过log_action_with_count记录操作摘要
    /// - 自动写 sync_queue：产生待同步记录
    pub async fn m_del(&self, id: &str, caller: &str, summary: &str) -> Result<bool, String> {
        self.audit.check_permission("m_del", caller, summary).await?;
        self.datasync.m_del(id).await
    }

    /// 同步保存记录（存在更新，不存在插入）
    /// - 用于从服务器同步数据到本地，或从客户端同步数据到服务器
    /// - 不自动填充 CID、upby、uptime
    /// - 不写 sync_queue（避免循环同步）
    pub async fn m_sync_save(&self, record: &HashMap<String, Value>) -> Result<String, String> {
        self.datasync.m_sync_save(record).await
    }

    /// 同步更新记录
    /// - 用于从服务器同步更新操作到本地
    /// - 不自动填充字段，不写 sync_queue
    pub async fn m_sync_update(&self, id: &str, record: &HashMap<String, Value>) -> Result<bool, String> {
        self.datasync.m_sync_update(id, record).await
    }

    /// 同步删除记录
    /// - 用于从服务器同步删除操作到本地
    /// - 不写 sync_queue（避免循环同步）
    pub async fn m_sync_del(&self, id: &str) -> Result<bool, String> {
        self.datasync.m_sync_del(id).await
    }

    // ========== 按天分表支持（动态表名） ==========

    /// 插入记录到指定表（支持按天分表）
    /// - 权限检查：验证caller是否有权限调用此方法
    /// - 审计日志：通过log_action_with_count记录操作摘要
    /// - 自动写 sync_queue：产生待同步记录（tbname=动态表名）
    /// - 自动设置 id、cid、upby、uptime
    pub async fn m_add_to_table(&self, table_name: &str, record: &HashMap<String, Value>, caller: &str, summary: &str) -> Result<String, String> {
        self.audit.check_permission("m_add", caller, summary).await?;
        self.datasync.m_add_to_table(table_name, record).await
    }

    /// 保存记录到指定表（支持按天分表，存在更新，不存在插入）
    /// - 权限检查：验证caller是否有权限调用此方法
    /// - 审计日志：通过log_action_with_count记录操作摘要
    /// - 自动写 sync_queue：产生待同步记录（tbname=动态表名）
    /// - 自动设置 id、cid、upby、uptime
    pub async fn m_save_to_table(&self, table_name: &str, record: &HashMap<String, Value>, caller: &str, summary: &str) -> Result<String, String> {
        self.audit.check_permission("m_save", caller, summary).await?;
        self.datasync.m_save_to_table(table_name, record).await
    }

    /// 查询记录
    /// - 权限检查：验证caller是否有权限调用此方法
    /// - 审计日志：通过log_action_with_count记录操作摘要
    pub async fn get(&self, where_clause: &str, params: Vec<rusqlite::types::Value>, caller: &str, summary: &str) -> Result<Vec<HashMap<String, Value>>, String> {
        self.audit.check_permission("get", caller, summary).await?;
        self.datasync.get(where_clause, params).await
    }

    /// 查询单条记录
    /// - 权限检查：验证caller是否有权限调用此方法
    /// - 审计日志：通过log_action_with_count记录操作摘要
    pub async fn get_one(&self, id: &str, caller: &str, summary: &str) -> Result<Option<HashMap<String, Value>>, String> {
        self.audit.check_permission("get_one", caller, summary).await?;
        self.datasync.get_one(id).await
    }

    /// 统计记录数
    /// - 权限检查：验证caller是否有权限调用此方法
    /// - 审计日志：通过log_action_with_count记录操作摘要
    pub async fn count(&self, caller: &str, summary: &str) -> Result<i32, String> {
        self.audit.check_permission("count", caller, summary).await?;
        self.datasync.count().await
    }

    /// 执行任意 SQL 查询（支持完整 SQL 拼接）
    /// - 权限检查：验证caller是否有权限调用此方法
    /// - 审计日志：通过log_action_with_count记录操作摘要
    pub async fn do_get(&self, sql: &str, params: Vec<rusqlite::types::Value>, caller: &str, summary: &str) -> Result<Vec<HashMap<String, Value>>, String> {
        self.audit.check_permission("do_get", caller, summary).await?;
        self.datasync.db.do_get(sql, params).await
    }

    /// 执行任意 SQL 更新（支持完整 SQL 拼接）
    /// - 权限检查：验证caller是否有权限调用此方法
    /// - 审计日志：通过log_action_with_count记录操作摘要
    /// - 返回影响的行数
    pub async fn do_m(&self, sql: &str, params: Vec<rusqlite::types::Value>, caller: &str, summary: &str) -> Result<usize, String> {
        self.audit.check_permission("do_m", caller, summary).await?;
        self.datasync.db.execute_with_params_affected(sql, params).await
    }
}

impl Default for DataState {
    fn default() -> Self {
        Self {
            base: BaseState::new(""),
            datasync: DataSync::new(""),
            audit: DataAudit::new(""),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_sync::{SyncData, SyncResult};
    use crate::datastate::{
        DATA_STATE_LOG_CREATE_SQL, DATA_SYNC_STATS_CREATE_SQL, SYNCLOG_CREATE_SQL,
    };
    use crate::sync_config::TableConfig;

    /// 辅助函数：创建带唯一表名的 DataState 实例
    fn create_test_state(table_name: &str) -> DataState {
        let db = LocalDB::default_instance().expect("数据库创建失败");
        DataState::with_db(table_name, db)
    }

    /// 测试1：从配置创建实例
    #[test]
    fn test_from_config() {
        let config = TableConfig::new("test_table_from_config");
        let state = DataState::from_config(&config);

        assert_eq!(state.datasync.table_name, "test_table_from_config");
        assert_eq!(state.base.name, "test_table_from_config");
    }

    /// 测试2：使用指定数据库创建实例
    #[test]
    fn test_with_db() {
        let db = LocalDB::default_instance().expect("数据库创建失败");
        let state = DataState::with_db("test_table_with_db", db);

        assert_eq!(state.datasync.table_name, "test_table_with_db");
    }

    /// 测试3：CRUD 操作 - 插入记录
    #[tokio::test]
    async fn test_m_add() {
        let unique_table = format!(
            "test_crud_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        );

        let state = create_test_state(&unique_table);

        // 创建测试表
        let conn = state.datasync.db.get_conn();
        let conn_guard = conn.lock().await;
        conn_guard
            .execute(
                &format!(
                    "CREATE TABLE IF NOT EXISTS {} (id TEXT PRIMARY KEY, name TEXT)",
                    unique_table
                ),
                [],
            )
            .expect("创建表失败");
        drop(conn_guard);

        let mut record = HashMap::new();
        record.insert("id".to_string(), Value::String("test_001".to_string()));
        record.insert("name".to_string(), Value::String("测试数据".to_string()));

        let result = state.m_add(&record, "test_caller", "测试插入");
        assert!(result.is_ok(), "插入应该成功: {:?}", result);
    }

    /// 测试4：CRUD 操作 - 查询记录
    #[tokio::test]
    async fn test_get_one() {
        let unique_table = format!(
            "test_get_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        );

        let state = create_test_state(&unique_table);

        // 创建测试表并插入数据
        let conn = state.datasync.db.get_conn();
        let conn_guard = conn.lock().await;
        conn_guard
            .execute(
                &format!(
                    "CREATE TABLE IF NOT EXISTS {} (id TEXT PRIMARY KEY, name TEXT)",
                    unique_table
                ),
                [],
            )
            .expect("创建表失败");
        conn_guard
            .execute(
                &format!("INSERT INTO {} (id, name) VALUES ('get_001', '查询测试')", unique_table),
                [],
            )
            .expect("插入数据失败");
        drop(conn_guard);

        let result = state.get_one("get_001", "test_caller", "测试查询");
        assert!(result.is_ok(), "查询应该成功: {:?}", result);
        let record = result.unwrap();
        assert!(record.is_some(), "应该找到记录");
        let record = record.unwrap();
        assert_eq!(record.get("id").and_then(|v: &Value| v.as_str()), Some("get_001"));
    }

    /// 测试5：CRUD 操作 - 更新记录
    #[tokio::test]
    async fn test_m_update() {
        let unique_table = format!(
            "test_update_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        );

        let state = create_test_state(&unique_table);

        // 创建测试表并插入数据（包含必要的元数据列）
        let conn = state.datasync.db.get_conn();
        let conn_guard = conn.lock().await;
        conn_guard
            .execute(
                &format!(
                    "CREATE TABLE IF NOT EXISTS {} (id TEXT PRIMARY KEY, name TEXT, cid TEXT, upby TEXT, uptime TEXT)",
                    unique_table
                ),
                [],
            )
            .expect("创建表失败");
        conn_guard
            .execute(
                &format!("INSERT INTO {} (id, name, cid, upby, uptime) VALUES ('update_001', '原始名称', '', '', '')", unique_table),
                [],
            )
            .expect("插入数据失败");
        drop(conn_guard);

        let mut record = HashMap::new();
        record.insert("name".to_string(), Value::String("更新后名称".to_string()));

        let result = state.m_update("update_001", &record, "test_caller", "测试更新");
        assert!(result.is_ok(), "更新应该成功: {:?}", result);
        assert!(result.unwrap(), "更新应该返回 true");
    }

    /// 测试6：同步操作 - 同步保存
    #[tokio::test]
    async fn test_m_sync_save() {
        let unique_table = format!(
            "test_sync_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        );

        let state = create_test_state(&unique_table);

        // 创建测试表
        let conn = state.datasync.db.get_conn();
        let conn_guard = conn.lock().await;
        conn_guard
            .execute(
                &format!(
                    "CREATE TABLE IF NOT EXISTS {} (id TEXT PRIMARY KEY, name TEXT)",
                    unique_table
                ),
                [],
            )
            .expect("创建表失败");
        drop(conn_guard);

        let pending_before = state.datasync.get_pending_count();

        let mut record = HashMap::new();
        record.insert("id".to_string(), Value::String("sync_001".to_string()));
        record.insert("name".to_string(), Value::String("同步数据".to_string()));

        let result = state.m_sync_save(&record);
        assert!(result.is_ok(), "同步保存应该成功: {:?}", result);

        let pending_after = state.datasync.get_pending_count();
        assert_eq!(pending_after, pending_before, "同步操作不应产生待同步记录");
    }

    /// 测试7：默认实例创建
    #[test]
    fn test_default() {
        let state = DataState::default();

        assert_eq!(state.base.name, "");
        assert_eq!(state.datasync.table_name, "");
    }

    /// 测试8：空表名实例创建
    #[test]
    fn test_empty_table_name() {
        let state = create_test_state("");

        assert_eq!(state.datasync.table_name, "");
    }

    /// 测试9：雪花ID生成
    #[test]
    fn test_snowflake_id() {
        let id = crate::snowflake::next_id_string();

        assert!(!id.is_empty());
        assert!(id.len() >= 18);
    }

    /// 测试10：删除记录
    #[tokio::test]
    async fn test_m_del() {
        let unique_table = format!(
            "test_del_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        );

        let state = create_test_state(&unique_table);

        // 创建测试表并插入数据
        let conn = state.datasync.db.get_conn();
        let conn_guard = conn.lock().await;
        conn_guard
            .execute(
                &format!(
                    "CREATE TABLE IF NOT EXISTS {} (id TEXT PRIMARY KEY, name TEXT)",
                    unique_table
                ),
                [],
            )
            .expect("创建表失败");
        conn_guard
            .execute(
                &format!("INSERT INTO {} (id, name) VALUES ('del_001', '待删除')", unique_table),
                [],
            )
            .expect("插入数据失败");
        drop(conn_guard);

        let result = state.m_del("del_001", "test_caller", "测试删除");
        assert!(result.is_ok(), "删除应该成功: {:?}", result);
        assert!(result.unwrap(), "删除应该返回 true");
    }

    /// 测试11：统计记录数
    #[tokio::test]
    async fn test_count() {
        let unique_table = format!(
            "test_count_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        );

        let state = create_test_state(&unique_table);

        // 创建测试表
        let conn = state.datasync.db.get_conn();
        let conn_guard = conn.lock().await;
        conn_guard
            .execute(
                &format!(
                    "CREATE TABLE IF NOT EXISTS {} (id TEXT PRIMARY KEY, name TEXT)",
                    unique_table
                ),
                [],
            )
            .expect("创建表失败");
        drop(conn_guard);

        let result = state.count("test_caller", "测试统计");
        assert!(result.is_ok(), "统计应该成功: {:?}", result);
    }

    #[test]
    fn test_sync_data_default() {
        let data = SyncData::default();
        assert_eq!(data.inserted, 0);
        assert_eq!(data.updated, 0);
        assert_eq!(data.skipped, 0);
    }

    #[test]
    fn test_sync_result_default() {
        let result = SyncResult {
            res: 0,
            errmsg: String::new(),
            datawf: SyncData::default(),
        };

        assert_eq!(result.res, 0);
        assert!(result.errmsg.is_empty());
    }

    #[test]
    fn test_ability_id_generation() {
        let id = crate::snowflake::next_id_string();

        assert!(!id.is_empty());
        assert!(id.len() >= 18); // 雪花ID长度约18-19位
    }

    /// DEMO 测试: 验证 DataState 组合 DataSync 功能
    /// 对应任务: 20260303200000
    /// 验证完成标准：
    /// 1. DataState 包含 db 成员变量
    /// 2. 方法签名不再接收 db 参数
    /// 3. 方法可以直接使用 self.db 访问数据库
    /// 4. DataSync 组件正确初始化
    #[tokio::test]
    async fn demo_20260303200000() {
        use base::mylogger;
        use std::sync::Arc;

        // 测试结构体，演示 mylogger!() 正确用法
        struct DemoTest {
            logger: Arc<base::mylogger::MyLogger>,
        }
        impl DemoTest {
            fn new() -> Self {
                Self {
                    logger: mylogger!(),
                }
            }
        }

        let tester = DemoTest::new();
        tester
            .logger
            .detail("=== 开始测试：demo_20260303200000 ===");
        tester.logger.detail("任务：验证 DataState db 成员变量修改");
        tester
            .logger
            .detail("完成标准：1.DataState包含db成员变量 2.方法签名移除db参数 3.编译通过");

        // 使用唯一表名避免数据冲突
        let unique_table = format!(
            "test_table_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        );

        // 1. 验证 DataState 包含 db 成员变量
        tester
            .logger
            .detail("步骤1: 验证 DataState 包含 db 成员变量");
        let mut state = DataState::default();
        // 使用唯一表名
        state.datasync = DataSync::new(&unique_table);
        tester
            .logger
            .detail("DataState::default() 创建成功，db 成员变量存在");

        // 验证 db 成员变量存在且可用（通过 datasync.db 访问）
        {
            let conn = state.datasync.db.get_conn();
            let conn_guard = conn.lock().await;
            tester.logger.detail("成功获取数据库连接");

            // 创建测试表
            conn_guard
                .execute(SYNCLOG_CREATE_SQL, [])
                .expect("创建 synclog 表失败");
            conn_guard
                .execute(DATA_STATE_LOG_CREATE_SQL, [])
                .expect("创建 data_state_log 表失败");
            conn_guard
                .execute(DATA_SYNC_STATS_CREATE_SQL, [])
                .expect("创建 data_sync_stats 表失败");

            // 清空测试表
            conn_guard
                .execute("DELETE FROM data_state_log", [])
                .expect("清空 data_state_log 表失败");
            conn_guard
                .execute("DELETE FROM data_sync_stats", [])
                .expect("清空 data_sync_stats 表失败");
            tester.logger.detail("测试表创建成功并清空");
        } // 释放锁

        // 2. 验证 log_status_change 方法
        tester.logger.detail("步骤2: 验证 log_status_change 方法");
        let result = state
            .datasync
            .log_status_change("idle", "working", "test reason", "test_worker");
        assert!(result.is_ok(), "log_status_change 调用失败: {:?}", result);
        tester.logger.detail("log_status_change 调用成功");

        // 3. 验证 get_status_logs 方法
        tester.logger.detail("步骤3: 验证 get_status_logs 方法");
        let logs = state.datasync.get_status_logs(10);
        assert_eq!(logs.len(), 1, "应该有1条日志记录");
        assert_eq!(logs[0].old_status, "idle");
        assert_eq!(logs[0].new_status, "working");
        tester
            .logger
            .detail(&format!("get_status_logs 返回 {} 条记录", logs.len()));

        // 4. 验证 update_sync_stats 方法
        tester.logger.detail("步骤4: 验证 update_sync_stats 方法");
        let result = state.datasync.update_sync_stats(10, 5, 2, 1, "test_worker");
        assert!(result.is_ok(), "update_sync_stats 调用失败: {:?}", result);
        tester.logger.detail("update_sync_stats 调用成功");

        // 5. 验证 get_sync_stats 方法
        tester.logger.detail("步骤5: 验证 get_sync_stats 方法");
        let stats = state.datasync.get_sync_stats(7);
        assert_eq!(stats.len(), 1, "应该有1条统计记录");
        assert_eq!(stats[0].downloaded, 10);
        assert_eq!(stats[0].updated, 5);
        assert_eq!(stats[0].skipped, 2);
        assert_eq!(stats[0].failed, 1);
        tester
            .logger
            .detail(&format!("get_sync_stats 返回 {} 条记录", stats.len()));

        // 6. 验证 DataSync 组件正确初始化
        tester.logger.detail("步骤6: 验证 DataSync 组件");
        let sync_queue = DataSync::new(&unique_table);
        assert_eq!(sync_queue.table_name, unique_table);
        tester.logger.detail("DataSync 创建成功");

        // 7. 验证 get_pending_count 方法
        tester.logger.detail("步骤7: 验证 get_pending_count 方法");
        let count = sync_queue.get_pending_count();
        assert_eq!(count, 0, "初始待同步数量应为0");
        tester
            .logger
            .detail(&format!("get_pending_count 返回 {}", count));

        tester.logger.detail("=== 所有验证通过 ===");
        tester.logger.detail("完成标准验证结果:");
        tester.logger.detail("1. DataState 包含 db 成员变量 - 通过");
        tester.logger.detail("2. 方法签名不再接收 db 参数 - 通过");
        tester.logger.detail("3. 编译通过 - 通过");
    }
}
