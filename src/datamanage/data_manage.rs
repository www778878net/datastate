//! DataManage - 数据库状态机管理器
//!
//! 管理所有表的同步状态
//! 只负责：验证、日志、统计

use crate::datastate::{DataState, SYNCLOG_CREATE_SQL};
use crate::data_sync::{DataSync, SyncResult};
use crate::localdb::LocalDB;
use crate::sync_config::TableConfig;
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use serde_json::Value;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use base::mylogger;

type AfterSyncCallback = Arc<dyn Fn(&[String]) + Send + Sync>;

/// DataManage 单例
static DATA_MANAGE: Lazy<DataManage> = Lazy::new(|| {
    DataManage::new().expect("Failed to create DataManage singleton")
});

/// 数据库状态机管理器
///
/// 职责：验证、日志、统计、状态管理
/// 注意：worker 过滤等业务逻辑由调用方通过 TableConfig.download_condition 设置
#[derive(Clone)]
pub struct DataManage {
    /// 状态存储
    states: Arc<RwLock<HashMap<String, DataState>>>,
    /// 本地数据库
    db: Arc<LocalDB>,
    /// 注册的表配置
    registered_tables: Arc<RwLock<HashMap<String, TableConfig>>>,
    /// 日志记录器
    logger: Arc<base::mylogger::MyLogger>,
    /// 同步后回调（可选，由 bin 入口注入，用于图同步等）
    after_sync_callback: Arc<RwLock<Option<AfterSyncCallback>>>,
    /// 用户数据（可选，由 bin 入口注入，如 OntologySync 实例）
    user_data: Arc<RwLock<Option<Arc<dyn Any + Send + Sync>>>>,
}

impl DataManage {
    /// 创建新的管理器实例
    pub fn new() -> Result<Self, String> {
        let db = LocalDB::default_instance()?;
        let logger = mylogger!();

        let manager = Self {
            states: Arc::new(RwLock::new(HashMap::new())),
            db: Arc::new(db),
            registered_tables: Arc::new(RwLock::new(HashMap::new())),
            logger,
            after_sync_callback: Arc::new(RwLock::new(None)),
            user_data: Arc::new(RwLock::new(None)),
        };
        Ok(manager)
    }

    /// 初始化系统表（需在 tokio runtime 中调用）
    pub async fn init(&self) -> Result<(), String> {
        self.db.init_system_tables().await?;
        self.ensure_synclog().await?;
        Ok(())
    }

    /// 获取单例实例
    pub fn get_singleton() -> &'static Self {
        &DATA_MANAGE
    }

    /// 确保 synclog 表存在
    async fn ensure_synclog(&self) -> Result<(), String> {
        if !self.db.table_exists("synclog").await? {
            self.db.execute(SYNCLOG_CREATE_SQL).await?;
        }
        Ok(())
    }

    /// 生成同步配置的唯一标识符
    fn make_sync_key(&self, name: &str, download_condition: Option<&Value>) -> String {
        match download_condition {
            None => name.to_string(),
            Some(cond) => {
                let cond_str = serde_json::to_string(cond).unwrap_or_default();
                let hash = Self::hash_condition(&cond_str);
                format!("{}_{}", name, hash)
            }
        }
    }

    /// 对条件字符串计算短 hash
    fn hash_condition(cond_str: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        cond_str.hash(&mut hasher);
        format!("{:08x}", hasher.finish())
    }

    /// 注册表状态
    pub async fn register(&self, config: TableConfig) -> Result<DataState, String> {
        let name = config.name.clone();
        if name.is_empty() {
            return Err("表名不能为空".to_string());
        }

        let sync_key = self.make_sync_key(&name, config.download_condition.as_ref());

        // 使用写锁进行双重检查，避免竞态条件
        {
            let mut states = self.states.write();
            // 双重检查：再次检查是否已存在
            if let Some(existing) = states.get(&sync_key) {
                return Ok(existing.clone());
            }

            let state = DataState::from_config(&config);
            self.ensure_table(&config).await?;

            {
                let mut tables = self.registered_tables.write();
                tables.insert(name.clone(), config.clone());
            }

            states.insert(sync_key.clone(), state.clone());
            
            // 在写锁内完成首次上传和下载
            drop(states);

            // 自动首次上传（有待同步数据时）
            if !config.apiurl.is_empty() {
                let pending_count = self.get_pending_count(&sync_key);
                if pending_count > 0 {
                    let result = state.datasync.upload_once().await;
                    if result.res == 0 {
                        self.logger.detail(&format!("首次上传成功: {}, uploaded={}", sync_key,
                            result.datawf.inserted + result.datawf.updated));
                    } else {
                        self.logger.error(&format!("首次上传失败: {}, {}", sync_key, result.errmsg));
                    }
                }
            }

            // 自动首次下载（跳过本地表，apiurl为空的不下载，download_enabled=false的不下载）
            if !config.apiurl.is_empty() && config.download_enabled {
                let result = state.datasync.download_once().await;
                if result.res == 0 {
                    let error_info = if let Some(ref errors) = result.datawf.errors {
                        format!(", errors={:?}", errors)
                    } else {
                        String::new()
                    };
                    self.logger.detail(&format!(
                        "首次下载成功: {}, inserted={}, updated={}, skipped={}, failed={:?}{}",
                        sync_key, result.datawf.inserted, result.datawf.updated, result.datawf.skipped, result.datawf.failed, error_info
                    ));
                } else {
                    self.logger.error(&format!("首次下载失败: {}, {}", sync_key, result.errmsg));
                }
            }

            Ok(state)
        }
    }

    /// 确保表存在
    async fn ensure_table(&self, config: &TableConfig) -> Result<bool, String> {
        let table_name = &config.name;

        if self.db.table_exists(table_name).await? {
            return Ok(false);
        }

        let create_sql = config.get_create_sql();
        self.db.ensure_table(table_name, &create_sql).await?;

        for index_sql in config.get_index_sql() {
            if let Err(e) = self.db.execute(&index_sql).await {
                self.logger.error(&format!("索引创建失败: {}", e));
            }
        }

        Ok(true)
    }

    /// 注销表状态
    pub async fn unregister(&self, name: &str) -> SyncResult {
        let mut states = self.states.write();
        if states.remove(name).is_none() {
            return SyncResult {
                res: -1,
                errmsg: format!("状态不存在: {}", name),
                datawf: Default::default(),
            };
        }
        SyncResult {
            res: 0,
            errmsg: String::new(),
            datawf: Default::default(),
        }
    }

    /// 获取状态
    pub fn get_state(&self, name: &str) -> Option<DataState> {
        let states = self.states.read();
        states.get(name).cloned()
    }

    /// 获取所有状态名称
    pub fn list_states(&self) -> Vec<String> {
        let states = self.states.read();
        states.keys().cloned().collect()
    }

    /// 获取所有状态摘要
    pub async fn list_summary(&self) -> Vec<serde_json::Value> {
        let states = self.states.read();
        states
            .iter()
            .map(|(name, state)| {
                serde_json::json!({
                    "name": name,
                    "status": state.base.status as i32,
                    "status_name": state.base.get_status_name(),
                    "last_download": state.datasync.last_download,
                    "last_upload": state.datasync.last_upload,
                    "error_message": if state.base.is_error() { &state.datasync.error_message } else { "" },
                })
            })
            .collect()
    }

    /// 更新状态
    pub fn update_state<F>(&self, name: &str, f: F) -> bool
    where
        F: FnOnce(&mut DataState),
    {
        let mut states = self.states.write();
        if let Some(state) = states.get_mut(name) {
            f(state);
            true
        } else {
            false
        }
    }

    /// 获取数据库引用
    pub fn db(&self) -> &LocalDB {
        &self.db
    }

    /// 获取数据库路径（用于创建 OntologySync 等）
    pub fn db_path(&self) -> String {
        self.db.get_db_path().to_string_lossy().to_string()
    }

    /// 获取数据库 Arc 引用
    pub fn db_arc(&self) -> Arc<LocalDB> {
        self.db.clone()
    }

    /// 设置同步后回调（由 bin 入口注入，用于图同步等）
    pub fn set_after_sync_callback(&self, callback: AfterSyncCallback) {
        let mut guard = self.after_sync_callback.write();
        *guard = Some(callback);
    }

    /// 设置用户数据（由 bin 入口注入，如 OntologySync 实例）
    pub fn set_user_data(&self, data: Arc<dyn Any + Send + Sync>) {
        let mut guard = self.user_data.write();
        *guard = Some(data);
    }

    /// 获取用户数据（需调用方 downcast 到具体类型）
    pub fn get_user_data<T: 'static + Send + Sync>(&self) -> Option<Arc<T>> {
        let guard = self.user_data.read();
        guard.as_ref().and_then(|data| {
            data.clone().downcast::<T>().ok()
        })
    }

    /// 从 sync_key 提取表名
    fn extract_table_name(&self, sync_key: &str) -> String {
        match sync_key.rfind('_') {
            Some(pos) if pos > 0 => {
                let suffix = &sync_key[pos + 1..];
                // 支持 8 或 16 个字符的十六进制后缀
                if (suffix.len() == 8 || suffix.len() == 16) && suffix.chars().all(|c| c.is_ascii_hexdigit()) {
                    sync_key[..pos].to_string()
                } else {
                    sync_key.to_string()
                }
            }
            _ => sync_key.to_string(),
        }
    }

    /// 获取待同步数据数量（查询所有分表）
    fn get_pending_count(&self, sync_key: &str) -> i32 {
        let table_name = self.extract_table_name(sync_key);
        match crate::data_sync::get_synclog() {
            Ok(synclog) => {
                let synclog: crate::data_sync::Synclog = synclog;
                tokio::task::block_in_place(|| {
                    match tokio::runtime::Handle::current().block_on(synclog.get_pending_count_by_tbname(&table_name)) {
                        Ok(count) => count,
                        Err(_) => 0,
                    }
                })
            }
            Err(_) => 0,
        }
    }

    /// 单步同步检查（按 sync_key）
    /// 检查时间是否到了，到了就执行下载/上传
    /// 返回：(是否执行了同步, 同步结果)
    pub async fn check_and_sync(&self, sync_key: &str) -> (bool, SyncResult) {
        let mut state = match self.get_state(sync_key) {
            Some(s) => s,
            None => {
                return (false, SyncResult {
                    res: -1,
                    errmsg: format!("sync_key 不存在: {}", sync_key),
                    datawf: Default::default(),
                });
            }
        };

        if !state.base.is_idle() {
            return (false, SyncResult::default());
        }

        let table_name = self.extract_table_name(sync_key);
        let local_count = self.db.count(&table_name).await.unwrap_or(0);

        let need_download = state.datasync.need_download(
            state.datasync.download_interval,
            state.datasync.last_download,
            state.datasync.min_pending,
            local_count,
            true,
        );

        let pending_count = self.get_pending_count(sync_key);
        let need_upload = state.datasync.need_upload(
            state.datasync.upload_interval,
            state.datasync.last_upload,
            pending_count,
            true,
        );

        if !need_download && !need_upload {
            return (false, SyncResult::default());
        }

        state.base.set_working();
        {
            let mut states = self.states.write();
            states.insert(sync_key.to_string(), state.clone());
        }

        let mut result = SyncResult::default();

        if need_download {
            let download_result = state.datasync.download_once().await;
            state.datasync.last_download = DataSync::current_time();
            result.datawf.inserted += download_result.datawf.inserted;
            result.datawf.updated += download_result.datawf.updated;
            if download_result.res != 0 {
                result.res = download_result.res;
                result.errmsg = download_result.errmsg;
            }
        }

        if need_upload {
            let upload_result = state.datasync.upload_once().await;
            state.datasync.last_upload = DataSync::current_time();
            result.datawf.inserted += upload_result.datawf.inserted;
            result.datawf.updated += upload_result.datawf.updated;
            if upload_result.res != 0 && result.res == 0 {
                result.res = upload_result.res;
                result.errmsg = upload_result.errmsg;
            }
        }

        state.base.set_idle();
        {
            let mut states = self.states.write();
            states.insert(sync_key.to_string(), state);
        }

        (true, result)
    }

    /// 执行一次同步检查
    pub async fn sync_once(&self) -> (SyncResult, Vec<String>) {
        let mut total_inserted = 0i64;
        let mut total_updated = 0i64;
        let mut total_errors = 0i64;
        let mut affected_tables: Vec<String> = Vec::new();

        let state_keys: Vec<String> = {
            let states = self.states.read();
            let keys: Vec<String> = states.keys().cloned().collect();
            keys
        };

        if state_keys.is_empty() {
            let logger = mylogger!();
            logger.info("[DataManage] sync_once: states 为空，没有注册的表");
        }

        for sync_key in state_keys {
            if let Some(mut state) = self.get_state(&sync_key) {
                if !state.base.is_idle() {
                    continue;
                }

                let table_name = self.extract_table_name(&sync_key);
                let local_count = self.db.count(&table_name).await.unwrap_or(0);

                // 检查是否启用下载
                let need_download = state.datasync.download_enabled && state.datasync.need_download(
                    state.datasync.download_interval,
                    state.datasync.last_download,
                    state.datasync.min_pending,
                    local_count,
                    state.base.is_idle()
                );
                // 检查是否启用上传
                let pending_count = self.get_pending_count(&sync_key);
                let need_upload = state.datasync.upload_enabled && state.datasync.need_upload(
                    state.datasync.upload_interval,
                    state.datasync.last_upload,
                    pending_count,
                    state.base.is_idle()
                );

                if need_download || need_upload {
                    state.base.set_working();
                    state.datasync.error_message.clear();
                    {
                        let mut states = self.states.write();
                        states.insert(sync_key.clone(), state.clone());
                    }

                    let mut inserted = 0i64;
                    let mut updated = 0i64;
                    let mut errors = 0i64;

                    if need_download {
                        // 调用 DataSync 组件的下载方法
                        let result = state.datasync.download_once().await;
                        if result.res == 0 {
                            state.datasync.last_download = DataSync::current_time();
                            state.base.set_idle();
                            if result.datawf.inserted > 0 || result.datawf.updated > 0 {
                                affected_tables.push(table_name.clone());
                            }
                        } else {
                            state.base.set_error();
                            state.datasync.error_message = result.errmsg;
                            state.datasync.error_time = DataSync::current_time();
                        }
                        inserted += result.datawf.inserted as i64;
                        updated += result.datawf.updated as i64;
                        if let Some(e) = result.datawf.failed {
                            errors += e as i64;
                        }
                    }
                    if need_upload {
                        // 调用 DataSync 组件的上传方法
                        let result = state.datasync.upload_once().await;
                        if result.res == 0 {
                            state.datasync.last_upload = DataSync::current_time();
                            state.base.set_idle();
                        } else {
                            state.base.set_error();
                            state.datasync.error_message = result.errmsg;
                            state.datasync.error_time = DataSync::current_time();
                        }
                        inserted += result.datawf.inserted as i64;
                        updated += result.datawf.updated as i64;
                        if let Some(e) = result.datawf.failed {
                            errors += e as i64;
                        }
                    }

                    total_inserted += inserted;
                    total_updated += updated;
                    total_errors += errors;

                    state.base.set_idle();
                    {
                        let mut states = self.states.write();
                        states.insert(sync_key.clone(), state);
                    }
                }
            }
        }

        (SyncResult {
            res: 0,
            errmsg: String::new(),
            datawf: crate::data_sync::SyncData {
                inserted: total_inserted as i32,
                updated: total_updated as i32,
                skipped: 0,
                failed: if total_errors > 0 { Some(total_errors as i32) } else { None },
                total: None,
                errors: None,
            },
        }, affected_tables)
    }

    /// 启动后台同步任务
    /// 每10秒检测上传下载
    pub fn run(&self) -> tokio::task::JoinHandle<()> {
        let manager = self.clone();
        
        tokio::spawn(async move {
            let logger = mylogger!();
            logger.info("[DataManage] 后台同步线程启动");
            
            let (result, affected) = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(manager.sync_once())
            });
            if result.datawf.inserted > 0 || result.datawf.updated > 0 {
                logger.info(&format!(
                    "[DataManage] 初始同步完成: inserted={}, updated={}",
                    result.datawf.inserted, result.datawf.updated
                ));
            }
            manager.invoke_after_sync(&affected);
            
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                
                let (result, affected) = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(manager.sync_once())
                });
                if result.datawf.inserted > 0 || result.datawf.updated > 0 {
                    logger.info(&format!(
                        "[DataManage] 同步完成: inserted={}, updated={}",
                        result.datawf.inserted, result.datawf.updated
                    ));
                }
                manager.invoke_after_sync(&affected);
            }
        })
    }

    fn invoke_after_sync(&self, affected_tables: &[String]) {
        let guard = self.after_sync_callback.read();
        if let Some(callback) = guard.as_ref() {
            callback(affected_tables);
        }
    }
}

impl Default for DataManage {
    fn default() -> Self {
        DataManage::get_singleton().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync_config::get_system_columns;
    use base::mylogger::MyLogger;

    #[tokio::test]
    async fn test_new() {
        let dm = DataManage::new();
        assert!(dm.is_ok());
    }

    #[test]
    fn test_default() {
        let dm = DataManage::default();
        // Just verify it creates successfully
        let _ = dm;
    }

    #[tokio::test]
    async fn test_register_table() {
        // 使用默认 DataManage（内部已有 LocalDB）- 按 plan2log.md 正确示例
        let dm = DataManage::default();

        // 配置 columns 是必须的
        let mut columns = get_system_columns();
        columns.insert("name".to_string(), "TEXT NOT NULL DEFAULT ''".to_string());

        // 创建 TableConfig
        let config = TableConfig {
            name: "test_table".to_string(),
            apiurl: "http://api.example.com/test/test_table".to_string(),
            columns,
            ..Default::default()
        };

        // 注册到 DataManage（DM 自动控制下载和上传）
        let result = dm.register(config).await;
        assert!(result.is_ok(), "注册失败");
    }

    /// DEMO 测试：注册本地表
    /// 任务：注册本地表 (steam_scan_queue, steam_item_history) 用于 Steam 市场扫描微服务
    /// 方式：DataManage::default() + 配置 columns + dm.register() + dm.sync_once()
    #[tokio::test]
    async fn demo_20260301203728_Step0() {
        use base::project_path::ProjectPath;
        use serde_json::json;

        let logger = MyLogger::new(
            "demo_20260301203728_Step0",
            7,
        );
        logger.detail("=== 开始测试：demo_20260301203728_Step0 ===");
        logger.detail("任务：注册本地表 (steam_scan_queue, steam_item_history) 并下载数据");
        logger.detail("方式：DataManage::default() + 配置 columns + dm.register() + dm.sync_once()");

        // 1. 使用默认 DataManage（内部已有 LocalDB）
        let dm = DataManage::default();
        logger.detail("DataManage::default() 实例创建成功");

        // 2. 从配置文件读取 worker 标识，用于 download_condition
        let worker = ProjectPath::find()
            .ok()
            .and_then(|p| p.worker_name())
            .unwrap_or_else(|| "unknown".to_string());
        logger.detail(&format!("当前 worker: {}", worker));

        // 3. 配置 steam_scan_queue 的 columns
        let mut columns = get_system_columns();
        columns.insert("cid".to_string(), "TEXT NOT NULL DEFAULT ''".to_string());
        columns.insert("idworkflow".to_string(), "TEXT NOT NULL DEFAULT ''".to_string());
        columns.insert("hashname".to_string(), "TEXT NOT NULL DEFAULT ''".to_string());
        columns.insert("state".to_string(), "INTEGER NOT NULL DEFAULT 0".to_string());
        columns.insert("requesttype".to_string(), "TEXT NOT NULL DEFAULT ''".to_string());
        columns.insert("price".to_string(), "INTEGER NOT NULL DEFAULT 0".to_string());
        columns.insert("worker".to_string(), "TEXT NOT NULL DEFAULT ''".to_string());
        columns.insert("nexttime".to_string(), "TEXT NOT NULL DEFAULT ''".to_string());
        columns.insert("applyreason".to_string(), "TEXT NOT NULL DEFAULT ''".to_string());
        columns.insert("lasterrortime".to_string(), "TEXT NOT NULL DEFAULT ''".to_string());
        columns.insert("lastoktime".to_string(), "TEXT NOT NULL DEFAULT ''".to_string());
        columns.insert("lasterrinfo".to_string(), "TEXT NOT NULL DEFAULT ''".to_string());
        columns.insert("lastokinfo".to_string(), "TEXT NOT NULL DEFAULT ''".to_string());
        columns.insert("errorcount".to_string(), "INTEGER NOT NULL DEFAULT 0".to_string());
        columns.insert("runcount".to_string(), "INTEGER NOT NULL DEFAULT 0".to_string());
        columns.insert("lastruntime".to_string(), "TEXT NOT NULL DEFAULT ''".to_string());

        // 4. 创建 TableConfig 并注册 steam_scan_queue（带 worker 过滤条件）
        // 过滤条件格式：&requesttype=steam_rent&worker=mypc
        let download_condition = json!({"requesttype": "steam_rent", "worker": worker.clone()});
        logger.detail(&format!("steam_scan_queue download_condition: {:?}", download_condition));

        let config_scan_queue = TableConfig {
            name: "steam_scan_queue".to_string(),
            apiurl: "http://api.example.com/steam/scan/steam_scan_queue".to_string(),
            columns,
            download_interval: 60,
            upload_interval: 300,
            download_condition: Some(download_condition),
            ..Default::default()
        };
        let result = dm.register(config_scan_queue).await;
        if result.is_ok() {
            logger.detail("steam_scan_queue 表注册成功（带 worker 过滤条件）");
        } else {
            logger.detail(&format!("steam_scan_queue 表注册失败: {:?}", result.err()));
        }

        // 5. 配置 steam_item_history 的 columns
        let mut columns = get_system_columns();
        columns.insert("cid".to_string(), "TEXT NOT NULL DEFAULT ''".to_string());
        columns.insert("hashname".to_string(), "TEXT NOT NULL DEFAULT ''".to_string());
        columns.insert("ddate".to_string(), "TEXT NOT NULL DEFAULT '1900-01-01'".to_string());
        columns.insert("onsalenum".to_string(), "INTEGER NOT NULL DEFAULT 0".to_string());
        columns.insert("onbuynum".to_string(), "INTEGER NOT NULL DEFAULT 0".to_string());
        columns.insert("steambuymax".to_string(), "INTEGER NOT NULL DEFAULT 0".to_string());
        columns.insert("steambuymin".to_string(), "INTEGER NOT NULL DEFAULT 0".to_string());
        columns.insert("steamsellmax".to_string(), "INTEGER NOT NULL DEFAULT 0".to_string());
        columns.insert("steamsellmin".to_string(), "INTEGER NOT NULL DEFAULT 0".to_string());
        columns.insert("steambuy2".to_string(), "INTEGER NOT NULL DEFAULT 0".to_string());
        columns.insert("steambuy3".to_string(), "INTEGER NOT NULL DEFAULT 0".to_string());
        columns.insert("steamsell2".to_string(), "INTEGER NOT NULL DEFAULT 0".to_string());
        columns.insert("steamsell3".to_string(), "INTEGER NOT NULL DEFAULT 0".to_string());
        columns.insert("steambuynum".to_string(), "INTEGER NOT NULL DEFAULT 0".to_string());
        columns.insert("steambuynum2".to_string(), "INTEGER NOT NULL DEFAULT 0".to_string());
        columns.insert("steambuynum3".to_string(), "INTEGER NOT NULL DEFAULT 0".to_string());
        columns.insert("steamsellnum".to_string(), "INTEGER NOT NULL DEFAULT 0".to_string());
        columns.insert("steamsellnum2".to_string(), "INTEGER NOT NULL DEFAULT 0".to_string());
        columns.insert("steamsellnum3".to_string(), "INTEGER NOT NULL DEFAULT 0".to_string());
        columns.insert("sellmax".to_string(), "TEXT NOT NULL DEFAULT '0'".to_string());
        columns.insert("sellmin".to_string(), "TEXT NOT NULL DEFAULT '0'".to_string());
        columns.insert("sellnum".to_string(), "INTEGER NOT NULL DEFAULT 0".to_string());
        columns.insert("isdown".to_string(), "INTEGER NOT NULL DEFAULT 0".to_string());

        // 6. 创建 TableConfig 并注册 steam_item_history
        let config_item_history = TableConfig {
            name: "steam_item_history".to_string(),
            apiurl: "http://api.example.com/steam/scan/steam_item_history".to_string(),
            columns,
            download_interval: 300,
            upload_interval: 300,
            ..Default::default()
        };
        let result = dm.register(config_item_history).await;
        if result.is_ok() {
            logger.detail("steam_item_history 表注册成功");
        } else {
            logger.detail(&format!("steam_item_history 表注册失败: {:?}", result.err()));
        }

        // 7. 验证已注册的表
        let registered = dm.list_states();
        logger.detail(&format!("已注册的表: {:?}", registered));

        // 8. 验证本地表结构
        let tables = dm.db().query_sync("SELECT name FROM sqlite_master WHERE type='table' AND name LIKE 'steam_%'", &[]).unwrap_or_default();
        logger.detail(&format!("本地 steam 表: {:?}", tables.iter().map(|r| r.get("name").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("")).collect::<Vec<_>>()));

        // 9. 执行同步，下载数据
        logger.detail("开始执行 dm.sync_once() 自动同步...");
        let (sync_result, _) = dm.sync_once().await;
        logger.detail(&format!("sync_once() 结果: res={}, inserted={}, updated={}, errmsg={}",
            sync_result.res,
            sync_result.datawf.inserted,
            sync_result.datawf.updated,
            sync_result.errmsg
        ));

        // 10. 验证下载数据
        let scan_queue_count = dm.db().count("steam_scan_queue").await.unwrap_or(0);
        let item_history_count = dm.db().count("steam_item_history").await.unwrap_or(0);
        logger.detail(&format!("steam_scan_queue 本地记录数: {}", scan_queue_count));
        logger.detail(&format!("steam_item_history 本地记录数: {}", item_history_count));

        // 11. 直接调用 download_from_server 获取详细 HTTP 日志
        logger.detail("=== 直接调用 download_from_server 获取详细日志 ===");
        let db = dm.db();

        // 构造完整 URL（模拟 download_from_server 内部逻辑）
        let api_url = "http://api.example.com/steam/scan/steam_scan_queue";
        let getnumber = 10;
        let download_condition = json!(["scan", worker.clone()]);

        // 日志输出请求信息
        logger.detail(&format!("API URL: {}/get", api_url));
        logger.detail(&format!("参数: getnumber={}, download_condition={:?}", getnumber, download_condition));

        // 直接下载
        let result = db.download_from_server(
            "steam_scan_queue",
            api_url,
            getnumber,
            0,  // getstart
            Some(&download_condition),
            None,  // download_cols
        );

        match result {
            Ok(records) => {
                logger.detail(&format!("下载成功，记录数: {}", records.len()));
                if !records.is_empty() {
                    logger.detail(&format!("第一条记录: {:?}", records[0]));
                }
            }
            Err(e) => {
                logger.detail(&format!("下载失败: {}", e));
            }
        }

        logger.detail("=== 测试通过：demo_20260301203728_Step0 ===");
    }
}