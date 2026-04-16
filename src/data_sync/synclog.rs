//! Synclog - synclog 表管理（分表）
//!
//! 对应 synclog 表
//! 按天分表，保留7天数据
//!
//! 切换策略：00:00 立即切换到今天的表写，读的时候可以读昨天的表，给足同步时间

use crate::workflow::{ShardType, ShardingConfig, ShardingManager, MaintenanceResult};
use crate::{Sqlite78, UpInfo};
use chrono::Timelike;
use serde_json::Value;
use std::collections::HashMap;

/// synclog 表建表 SQL（分表版本）
pub const SQL_CREATE_SYNCLOG: &str = r#"
CREATE TABLE IF NOT EXISTS {TABLE_NAME} (
    idpk INTEGER PRIMARY KEY AUTOINCREMENT,
    apisys TEXT NOT NULL DEFAULT 'v1',
    apimicro TEXT NOT NULL DEFAULT 'iflow',
    apiobj TEXT NOT NULL DEFAULT 'synclog',
    tbname TEXT NOT NULL DEFAULT '',
    action TEXT NOT NULL DEFAULT '',
    cmdtext TEXT NOT NULL DEFAULT '',
    params TEXT NOT NULL DEFAULT '[]',
    idrow TEXT NOT NULL DEFAULT '',
    worker TEXT NOT NULL DEFAULT '',
    synced INTEGER NOT NULL DEFAULT 0,
    lasterrinfo TEXT NOT NULL DEFAULT '',
    cmdtextmd5 TEXT NOT NULL DEFAULT '',
    num INTEGER NOT NULL DEFAULT 0,
    dlong INTEGER NOT NULL DEFAULT 0,
    downlen INTEGER NOT NULL DEFAULT 0,
    id TEXT NOT NULL DEFAULT '',
    upby TEXT NOT NULL DEFAULT '',
    uptime TEXT NOT NULL DEFAULT '',
    cid TEXT NOT NULL DEFAULT ''
)
"#;

/// synclog 索引 SQL
pub const SQL_CREATE_SYNCLOG_INDEX: &str = r#"
CREATE INDEX IF NOT EXISTS idx_{TABLE_NAME}_tbname_synced ON {TABLE_NAME} (tbname, synced)
"#;

/// Synclog - synclog 表管理
pub struct Synclog {
    db: Sqlite78,
    sharding_manager: Option<ShardingManager>,
}

impl Synclog {
    /// 创建新实例（不分表）
    pub fn new(db: Sqlite78) -> Self {
        Self {
            db,
            sharding_manager: None,
        }
    }

    /// 创建分表实例
    pub fn with_sharding(mut db: Sqlite78) -> Result<Self, String> {
        db.initialize()?;

        let config = ShardingConfig::new(ShardType::Daily, "synclog")
            .with_table_sql(SQL_CREATE_SYNCLOG)
            .with_retention(7); // 保留7天

        let conn = db.get_conn()?;
        let sharding_manager = ShardingManager::new(conn, config);

        Ok(Self {
            db,
            sharding_manager: Some(sharding_manager),
        })
    }

    /// 使用默认数据库路径创建分表实例
    pub fn with_default_path() -> Result<Self, String> {
        let db = Sqlite78::with_default_path();
        Self::with_sharding(db)
    }

    /// 使用指定路径创建分表实例
    pub fn with_path(path: &str) -> Result<Self, String> {
        let db = Sqlite78::with_config(path, false, false);
        Self::with_sharding(db)
    }

    /// 获取当前表名（用于写操作：00:00 立即切换到今天的表）
    pub fn get_table_name(&self) -> String {
        if let Some(ref _manager) = self.sharding_manager {
            let config = ShardingConfig::new(ShardType::Daily, "synclog");
            config.get_current_table_name()
        } else {
            "synclog".to_string()
        }
    }

    /// 获取上传表名（用于读操作：延迟切换策略）
    /// 
    /// 延迟切换策略：
    /// - 00:00-00:30：读取昨天的分表（确保昨天的数据有充足时间同步）
    /// - 00:30之后：读取今天的分表
    pub fn get_upload_table_name(&self) -> String {
        if let Some(ref _manager) = self.sharding_manager {
            let config = ShardingConfig::new(ShardType::Daily, "synclog");
            
            // 获取当前时间
            let now = chrono::Local::now();
            let hour = now.hour();
            let minute = now.minute();
            
            // 判断是否在延迟切换窗口期（00:00-00:30）
            let is_in_delay_window = hour == 0 && minute < 30;
            
            if is_in_delay_window {
                // 在延迟窗口期内，使用昨天的日期
                let yesterday = now.date_naive() - chrono::Duration::days(1);
                config.get_table_name(Some(yesterday))
            } else {
                // 其他时间，使用今天的日期
                config.get_current_table_name()
            }
        } else {
            "synclog".to_string()
        }
    }

    /// 获取需要查询的分表列表（用于读操作：查询过去N天的表）
    /// 返回实际存在的分表
    pub fn get_query_shard_tables(&self, days_back: i32) -> Vec<String> {
        let mut tables = Vec::new();
        
        if let Some(ref _manager) = self.sharding_manager {
            let config = ShardingConfig::new(ShardType::Daily, "synclog");
            let today = chrono::Local::now().date_naive();
            
            for i in 0..=days_back {
                let date = today - chrono::Duration::days(i as i64);
                let table_name = config.get_table_name(Some(date));
                // 只添加实际存在的表
                if self.table_exists(&table_name) {
                    tables.push(table_name);
                }
            }
        }
        
        tables
    }

    /// 检查表是否存在
    fn table_exists(&self, table_name: &str) -> bool {
        let sql = format!(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='{}'",
            table_name
        );
        let up = UpInfo::new();
        match self.db.do_get(&sql, &[], &up) {
            Ok(rows) => !rows.is_empty(),
            Err(_) => false,
        }
    }

    /// 获取过去7天的分表列表（默认查询范围）
    pub fn get_default_query_tables(&self) -> Vec<String> {
        self.get_query_shard_tables(7)
    }

    /// 获取上传查询的分表列表（用于读取待同步记录：延迟切换策略）
    /// 
    /// 延迟切换策略：
    /// - 00:00-00:30：不包含今天的表（只查询昨天及之前的表）
    /// - 00:30之后：包含今天的表
    pub fn get_upload_query_tables(&self, days_back: i32) -> Vec<String> {
        if let Some(ref _manager) = self.sharding_manager {
            let config = ShardingConfig::new(ShardType::Daily, "synclog");
            let now = chrono::Local::now();
            let today = now.date_naive();
            
            // 判断是否在延迟切换窗口期（00:00-00:30）
            let hour = now.hour();
            let minute = now.minute();
            let is_in_delay_window = hour == 0 && minute < 30;
            
            let mut tables = Vec::new();
            
            // 在延迟窗口期内，从昨天开始查询（i=1），否则从今天开始（i=0）
            let start_i = if is_in_delay_window { 1 } else { 0 };
            
            for i in start_i..=days_back {
                let date = today - chrono::Duration::days(i as i64);
                tables.push(config.get_table_name(Some(date)));
            }
            
            tables
        } else {
            vec!["synclog".to_string()]
        }
    }

    /// 获取上传查询的默认分表列表（过去7天，应用延迟切换策略）
    pub fn get_default_upload_query_tables(&self) -> Vec<String> {
        self.get_upload_query_tables(7)
    }

    /// 执行分表维护
    pub fn perform_maintenance(&mut self) -> Result<MaintenanceResult, String> {
        if let Some(ref mut manager) = self.sharding_manager {
            return manager.perform_maintenance();
        }
        Ok(MaintenanceResult::default())
    }

    /// 创建今天的分表
    pub fn create_today_table(&self) -> Result<(), String> {
        if let Some(ref manager) = self.sharding_manager {
            let table_name = self.get_table_name();
            if !manager.table_exists(&table_name)? {
                manager.create_sharding_table(&table_name)?;

                // 创建索引
                let conn = self.db.get_conn()?;
                let conn = conn.lock().map_err(|e| e.to_string())?;
                let index_sql = SQL_CREATE_SYNCLOG_INDEX.replace("{TABLE_NAME}", &table_name);
                conn.execute(&index_sql, [])
                    .map_err(|e| format!("创建索引失败: {}", e))?;
            }
        }
        Ok(())
    }

    /// 添加记录到 synclog
    pub fn insert(&self, data: &HashMap<String, Value>, up: &UpInfo) -> Result<String, String> {
        let table_name = self.get_table_name();
        self.create_today_table()?;

        let id = data
            .get("id")
            .and_then(|v: &serde_json::Value| v.as_str())
            .unwrap_or(&UpInfo::new_id())
            .to_string();
        let apisys = data.get("apisys").and_then(|v| v.as_str()).unwrap_or("v1");
        let apimicro = data.get("apimicro").and_then(|v| v.as_str()).unwrap_or("iflow");
        let apiobj = data.get("apiobj").and_then(|v| v.as_str()).unwrap_or("synclog");
        let tbname = data.get("tbname").and_then(|v| v.as_str()).unwrap_or("");
        let action = data.get("action").and_then(|v| v.as_str()).unwrap_or("");
        let cmdtext = data.get("cmdtext").and_then(|v| v.as_str()).unwrap_or("");
        let params = data.get("params").and_then(|v| v.as_str()).unwrap_or("[]");
        let idrow = data.get("idrow").and_then(|v| v.as_str()).unwrap_or("");
        let worker = data.get("worker").and_then(|v| v.as_str()).unwrap_or("");
        let synced = data.get("synced").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let lasterrinfo = data.get("lasterrinfo").and_then(|v| v.as_str()).unwrap_or("");
        let cmdtextmd5 = data.get("cmdtextmd5").and_then(|v| v.as_str()).unwrap_or("");
        let num = data.get("num").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let dlong = data.get("dlong").and_then(|v| v.as_i64()).unwrap_or(0);
        let downlen = data.get("downlen").and_then(|v| v.as_i64()).unwrap_or(0);
        let upby = data.get("upby").and_then(|v| v.as_str()).unwrap_or("");
        let uptime = data.get("uptime").and_then(|v| v.as_str()).unwrap_or("");
        let cid = data.get("cid").and_then(|v| v.as_str()).unwrap_or("");

        let sql = format!(
            "REPLACE INTO {} (\
                apisys, apimicro, apiobj, tbname, action,\
                cmdtext, params, idrow, worker, synced,\
                lasterrinfo, cmdtextmd5, num, dlong, downlen,\
                id, upby, uptime, cid\
            ) VALUES (\
                ?, ?, ?, ?, ?,\
                ?, ?, ?, ?, ?,\
                ?, ?, ?, ?, ?,\
                ?, ?, ?, ?\
            )",
            table_name
        );

        self.db.do_m_add(
            &sql,
            &[
                &apisys as &dyn rusqlite::ToSql,
                &apimicro as &dyn rusqlite::ToSql,
                &apiobj as &dyn rusqlite::ToSql,
                &tbname as &dyn rusqlite::ToSql,
                &action as &dyn rusqlite::ToSql,
                &cmdtext as &dyn rusqlite::ToSql,
                &params as &dyn rusqlite::ToSql,
                &idrow as &dyn rusqlite::ToSql,
                &worker as &dyn rusqlite::ToSql,
                &synced as &dyn rusqlite::ToSql,
                &lasterrinfo as &dyn rusqlite::ToSql,
                &cmdtextmd5 as &dyn rusqlite::ToSql,
                &num as &dyn rusqlite::ToSql,
                &dlong as &dyn rusqlite::ToSql,
                &downlen as &dyn rusqlite::ToSql,
                &id as &dyn rusqlite::ToSql,
                &upby as &dyn rusqlite::ToSql,
                &uptime as &dyn rusqlite::ToSql,
                &cid as &dyn rusqlite::ToSql,
            ],
            up,
        )?;

        Ok(id)
    }

    /// 获取待同步记录（从默认查询范围的所有分表中获取）
    pub fn get_pending_items(&self, limit: i32) -> Result<Vec<HashMap<String, Value>>, String> {
        let tables = self.get_default_query_tables();
        let up = UpInfo::new();
        let mut all_items = Vec::new();

        for table_name in tables {
            let sql = format!("SELECT * FROM {} WHERE synced = 0 ORDER BY idpk ASC LIMIT ?", table_name);
            match self.db.do_get(&sql, &[&limit as &dyn rusqlite::ToSql], &up) {
                Ok(mut items) => {
                    all_items.append(&mut items);
                    if all_items.len() >= limit as usize {
                        break;
                    }
                }
                Err(_) => continue,
            }
        }

        // 限制返回数量
        if all_items.len() > limit as usize {
            all_items.truncate(limit as usize);
        }

        Ok(all_items)
    }

    /// 获取待上传记录（应用延迟切换策略：00:00-00:30 不查询今天的表）
    pub fn get_pending_upload_items(&self, limit: i32) -> Result<Vec<HashMap<String, Value>>, String> {
        let tables = self.get_default_upload_query_tables();
        let up = UpInfo::new();
        let mut all_items = Vec::new();

        for table_name in tables {
            let sql = format!("SELECT * FROM {} WHERE synced = 0 ORDER BY idpk ASC LIMIT ?", table_name);
            match self.db.do_get(&sql, &[&limit as &dyn rusqlite::ToSql], &up) {
                Ok(mut items) => {
                    all_items.append(&mut items);
                    if all_items.len() >= limit as usize {
                        break;
                    }
                }
                Err(_) => continue,
            }
        }

        // 限制返回数量
        if all_items.len() > limit as usize {
            all_items.truncate(limit as usize);
        }

        Ok(all_items)
    }

    /// 获取待同步记录数（统计默认查询范围的所有分表）
    pub fn get_pending_count(&self) -> Result<i32, String> {
        let tables = self.get_default_query_tables();
        let up = UpInfo::new();
        let mut total = 0;

        for table_name in tables {
            let sql = format!("SELECT COUNT(*) as cnt FROM {} WHERE synced = 0", table_name);
            if let Ok(rows) = self.db.do_get(&sql, &[], &up) {
                if let Some(row) = rows.first() {
                    if let Some(cnt) = row.get("cnt").and_then(|v| v.as_i64()) {
                        total += cnt as i32;
                    }
                }
            }
        }

        Ok(total)
    }

    /// 获取待上传记录数（应用延迟切换策略：00:00-00:30 不统计今天的表）
    pub fn get_pending_upload_count(&self) -> Result<i32, String> {
        let tables = self.get_default_upload_query_tables();
        let up = UpInfo::new();
        let mut total = 0;

        for table_name in tables {
            let sql = format!("SELECT COUNT(*) as cnt FROM {} WHERE synced = 0", table_name);
            if let Ok(rows) = self.db.do_get(&sql, &[], &up) {
                if let Some(row) = rows.first() {
                    if let Some(cnt) = row.get("cnt").and_then(|v| v.as_i64()) {
                        total += cnt as i32;
                    }
                }
            }
        }

        Ok(total)
    }

    /// 标记已同步（根据记录所在的分表分别更新）
    pub fn mark_synced(&self, items: &[HashMap<String, Value>]) -> Result<(), String> {
        if items.is_empty() {
            return Ok(());
        }

        // 按表名分组
        let mut table_groups: HashMap<String, Vec<i64>> = HashMap::new();
        
        for item in items {
            if let Some(idpk) = item.get("idpk").and_then(|v| v.as_i64()) {
                // 这里简化处理：实际应用中需要知道记录属于哪个分表
                // 为了简单起见，我们尝试在所有分表中更新
                let tables = self.get_default_query_tables();
                for table in tables {
                    table_groups.entry(table).or_default().push(idpk);
                }
            }
        }

        let up = UpInfo::new();

        for (table_name, idpk_list) in table_groups {
            if idpk_list.is_empty() {
                continue;
            }

            let placeholders: Vec<String> = idpk_list.iter().map(|_| "?".to_string()).collect();
            let sql = format!(
                "UPDATE {} SET synced = 1 WHERE idpk IN ({})",
                table_name,
                placeholders.join(", ")
            );

            let mut params: Vec<&dyn rusqlite::ToSql> = Vec::new();
            for id in &idpk_list {
                params.push(id);
            }

            let _ = self.db.do_m(&sql, &params, &up);
        }

        Ok(())
    }

    /// 获取底层数据库引用
    pub fn get_db(&self) -> &Sqlite78 {
        &self.db
    }

    /// 获取所有 synclog 分表
    pub fn get_all_shard_tables(&self) -> Result<Vec<String>, String> {
        if let Some(ref manager) = self.sharding_manager {
            return manager.get_all_shard_tables();
        }
        Ok(vec!["synclog".to_string()])
    }

    // ========== DataSync 配合使用的方法 ==========

    /// 获取指定表名的待同步记录数
    /// 直接用基础表名查询 synclog（兼容分表和非分表）
    /// 如果昨天的数据已全部同步，更新进度文件
    pub fn get_pending_count_by_tbname(&self, tbname: &str) -> Result<i32, String> {
        let synclog_tables = self.get_default_upload_query_tables();
        let up = UpInfo::new();
        let mut total = 0;

        for synclog_table in &synclog_tables {
            let sql = format!("SELECT COUNT(*) as cnt FROM {} WHERE tbname = ? AND synced = 0", synclog_table);
            if let Ok(rows) = self.db.do_get(&sql, &[&tbname as &dyn rusqlite::ToSql], &up) {
                if let Some(row) = rows.first() {
                    if let Some(cnt) = row.get("cnt").and_then(|v| v.as_i64()) {
                        total += cnt as i32;
                    }
                }
            }
        }

        let now = chrono::Local::now();
        let today = now.date_naive();
        let yesterday = today - chrono::Duration::days(1);
        let yesterday_str = yesterday.format("%Y%m%d").to_string();

        if total == 0 {
            if let Some(progress_date) = Self::read_progress_date(tbname) {
                if progress_date != yesterday_str {
                    let _ = Self::save_progress_date(tbname, &yesterday_str);
                }
            } else {
                let _ = Self::save_progress_date(tbname, &yesterday_str);
            }
        }

        Ok(total)
    }

    /// 获取指定表名的待同步记录
    /// 直接用基础表名查询 synclog（兼容分表和非分表）
    pub fn get_pending_items_by_tbname(&self, tbname: &str, limit: i32) -> Result<Vec<HashMap<String, Value>>, String> {
        let synclog_tables = self.get_default_upload_query_tables();
        let up = UpInfo::new();
        let mut all_items = Vec::new();

        for synclog_table in synclog_tables {
            let sql = format!(
                "SELECT * FROM {} WHERE tbname = ? AND synced = 0 ORDER BY idpk ASC LIMIT ?",
                synclog_table
            );
            let remaining = limit as usize - all_items.len();
            if remaining == 0 {
                break;
            }
            match self.db.do_get(&sql, &[&tbname as &dyn rusqlite::ToSql, &(remaining as i32) as &dyn rusqlite::ToSql], &up) {
                Ok(mut items) => {
                    all_items.append(&mut items);
                    if all_items.len() >= limit as usize {
                        break;
                    }
                }
                Err(_) => continue,
            }
            if all_items.len() >= limit as usize {
                break;
            }
        }

        Ok(all_items)
    }

    /// 获取业务表的写表名（始终返回今天的分表名）
    pub fn get_business_table_name_for_write(&self, base_table: &str) -> String {
        let today = chrono::Local::now().date_naive();
        format!("{}_{}", base_table, today.format("%Y%m%d"))
    }

    /// 获取业务表的读表名（使用延迟切换策略）
    /// - 00:00-00:30：返回昨天的分表名
    /// - 00:30之后：返回今天的分表名
    pub fn get_business_table_name_for_read(&self, base_table: &str) -> String {
        let now = chrono::Local::now();
        let today = now.date_naive();
        
        let hour = now.hour();
        let minute = now.minute();
        let is_in_delay_window = hour == 0 && minute < 30;
        
        if is_in_delay_window {
            let yesterday = today - chrono::Duration::days(1);
            format!("{}_{}", base_table, yesterday.format("%Y%m%d"))
        } else {
            format!("{}_{}", base_table, today.format("%Y%m%d"))
        }
    }

    /// 获取进度文件路径
    fn get_progress_file_path(base_table: &str) -> String {
        format!("tmp/synclog/{}.txt", base_table)
    }

    /// 读取进度文件中的日期
    fn read_progress_date(base_table: &str) -> Option<String> {
        let path = Self::get_progress_file_path(base_table);
        if std::path::Path::new(&path).exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                let date_str = content.trim().to_string();
                if !date_str.is_empty() {
                    return Some(date_str);
                }
            }
        }
        None
    }

    /// 保存进度日期到文件
    pub fn save_progress_date(base_table: &str, date: &str) -> Result<(), String> {
        let path = Self::get_progress_file_path(base_table);
        if let Some(parent) = std::path::Path::new(&path).parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("创建目录失败: {}", e))?;
            }
        }
        std::fs::write(&path, date)
            .map_err(|e| format!("写入进度文件失败: {}", e))
    }

    /// 获取需要查询的业务表分表名列表（使用进度文件优化）
    /// 
    /// 逻辑：
    /// 1. 检查进度文件，如果昨天已处理完，只查今天
    /// 2. 否则查昨天和今天
    /// 3. 0:30 之后，昨天可以标记为已处理，只查今天
    fn get_existing_business_table_names(&self, base_table: &str) -> Vec<String> {
        let now = chrono::Local::now();
        let today = now.date_naive();
        let today_str = today.format("%Y%m%d").to_string();
        let yesterday = today - chrono::Duration::days(1);
        let yesterday_str = yesterday.format("%Y%m%d").to_string();
        
        let hour = now.hour();
        let minute = now.minute();
        let is_after_delay_window = hour > 0 || minute >= 30;
        
        let mut tables = Vec::new();
        
        if let Some(progress_date) = Self::read_progress_date(base_table) {
            if progress_date == yesterday_str {
                if is_after_delay_window {
                    tables.push(format!("{}_{}", base_table, today_str));
                } else {
                    tables.push(format!("{}_{}", base_table, yesterday_str));
                    tables.push(format!("{}_{}", base_table, today_str));
                }
            } else if progress_date == today_str {
                tables.push(format!("{}_{}", base_table, today_str));
            } else {
                tables.push(format!("{}_{}", base_table, yesterday_str));
                tables.push(format!("{}_{}", base_table, today_str));
            }
        } else {
            tables.push(format!("{}_{}", base_table, yesterday_str));
            tables.push(format!("{}_{}", base_table, today_str));
        }
        
        tables
    }

    /// 标记已同步（接受 idpk 列表）
    pub fn mark_synced_by_idpks(&self, idpk_list: &[i64]) -> Result<(), String> {
        if idpk_list.is_empty() {
            return Ok(());
        }

        // 尝试在所有分表中更新
        let tables = self.get_default_query_tables();
        let up = UpInfo::new();

        let placeholders: Vec<String> = idpk_list.iter().map(|_| "?".to_string()).collect();

        for table_name in tables {
            let sql = format!(
                "UPDATE {} SET synced = 1 WHERE idpk IN ({})",
                table_name,
                placeholders.join(", ")
            );
            let mut params: Vec<&dyn rusqlite::ToSql> = Vec::new();
            for id in idpk_list {
                params.push(id);
            }
            let _ = self.db.do_m(&sql, &params, &up);
        }

        Ok(())
    }

    /// 标记已同步（接受 id 列表）
    pub fn mark_synced_by_ids(&self, id_list: &[String]) -> Result<(), String> {
        if id_list.is_empty() {
            return Ok(());
        }

        let tables = self.get_default_query_tables();
        let up = UpInfo::new();
        let placeholders: Vec<String> = id_list.iter().map(|_| "?".to_string()).collect();

        for table_name in tables {
            let sql = format!(
                "UPDATE {} SET synced = 1 WHERE id IN ({})",
                table_name,
                placeholders.join(", ")
            );
            let mut params: Vec<&dyn rusqlite::ToSql> = Vec::new();
            for id in id_list {
                params.push(id);
            }
            let _ = self.db.do_m(&sql, &params, &up);
        }

        Ok(())
    }

    /// 标记失败（接受 id 和错误信息）
    pub fn mark_failed_by_id(&self, id: &str, errinfo: &str) -> Result<(), String> {
        let tables = self.get_default_query_tables();
        let up = UpInfo::new();
        // 截取错误信息，避免递归膨胀（MySQL错误中可能包含完整SQL+原始lasterrinfo）
        let truncated = Self::truncate_errinfo(errinfo);

        for table_name in tables {
            let sql = format!(
                "UPDATE {} SET synced = -1, lasterrinfo = '{}' WHERE id = '{}'",
                table_name, truncated, id
            );
            let _ = self.db.do_m(&sql, &[], &up);
        }

        Ok(())
    }

    /// 标记失败（接受 idrow 和错误信息）
    pub fn mark_failed_by_idrow(&self, idrow: &str, errinfo: &str) -> Result<(), String> {
        let tables = self.get_default_query_tables();
        let up = UpInfo::new();
        // 截取错误信息，避免递归膨胀（MySQL错误中可能包含完整SQL+原始lasterrinfo）
        let truncated = Self::truncate_errinfo(errinfo);

        for table_name in tables {
            let sql = format!(
                "UPDATE {} SET synced = -1, lasterrinfo = '{}' WHERE idrow = '{}'",
                table_name, truncated, idrow
            );
            let _ = self.db.do_m(&sql, &[], &up);
        }

        Ok(())
    }

    /// 将 UPDATE 失败的记录转换为 INSERT（用于重试）
    /// 当服务器返回"没有找到匹配的记录"时，尝试改为 INSERT
    pub fn convert_update_to_insert(&self, id: &str) -> Result<(), String> {
        let tables = self.get_default_query_tables();
        let up = UpInfo::new();

        for table_name in &tables {
            // 查找 synclog 记录
            let select_sql = format!(
                "SELECT idpk, tbname, idrow FROM {} WHERE id = ? AND action = 'update' AND synced = -1 LIMIT 1",
                table_name
            );
            
            if let Ok(rows) = self.db.do_get(&select_sql, &[&id as &dyn rusqlite::ToSql], &up) {
                if let Some(row) = rows.first() {
                    let idpk: i64 = row.get("idpk").and_then(|v| v.as_i64()).unwrap_or(0);
                    let tbname: String = row.get("tbname").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    // idrow 可能是 String 或 Number 类型（JSON 解析时数字字符串会被转为 Number）
                    let idrow: String = row.get("idrow")
                        .and_then(|v| v.as_str().map(|s| s.to_string()))
                        .or_else(|| row.get("idrow").and_then(|v| v.as_i64().map(|n| n.to_string())))
                        .unwrap_or_default();
                    
                    if idpk > 0 && !tbname.is_empty() && !idrow.is_empty() {
                        // 从本地数据库查询原始数据
                        let data_sql = format!("SELECT * FROM `{}` WHERE id = ? LIMIT 1", tbname);
                        if let Ok(data_rows) = self.db.do_get(&data_sql, &[&idrow as &dyn rusqlite::ToSql], &up) {
                            if let Some(data_row) = data_rows.first() {
                                // 构建 INSERT SQL
                                let (cmdtext, params) = Self::build_insert_sql_from_row(&tbname, data_row);
                                let params_json = serde_json::to_string(&params).unwrap_or_default();
                                let cmdtextmd5 = format!("{:x}", md5::compute(&cmdtext));
                                
                                // 更新 synclog 记录
                                let update_sql = format!(
                                    "UPDATE {} SET action = 'insert', cmdtext = ?, params = ?, cmdtextmd5 = ?, synced = 0, lasterrinfo = '' WHERE idpk = ?",
                                    table_name
                                );
                                let _ = self.db.do_m(
                                    &update_sql,
                                    &[
                                        &cmdtext as &dyn rusqlite::ToSql,
                                        &params_json as &dyn rusqlite::ToSql,
                                        &cmdtextmd5 as &dyn rusqlite::ToSql,
                                        &idpk as &dyn rusqlite::ToSql,
                                    ],
                                    &up,
                                );
                                
                                return Ok(());
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// 从数据库行构建 INSERT SQL
    /// 排除系统列（created_at, updated_at, deleted），这些列服务器端不存在
    fn build_insert_sql_from_row(tbname: &str, row: &std::collections::HashMap<String, serde_json::Value>) -> (String, Vec<serde_json::Value>) {
        let exclude_cols = ["created_at", "updated_at", "deleted"];
        
        let mut columns: Vec<&str> = row.keys()
            .map(|s| s.as_str())
            .filter(|c| !exclude_cols.contains(c))
            .collect();
        columns.sort();
        
        let placeholders: Vec<&str> = columns.iter().map(|_| "?").collect();
        let cmdtext = format!(
            "INSERT INTO `{}` ({}) VALUES ({})",
            tbname,
            columns.iter().map(|c| format!("`{}`", c)).collect::<Vec<_>>().join(", "),
            placeholders.join(", ")
        );
        
        let params: Vec<serde_json::Value> = columns.iter()
            .filter_map(|c| row.get(*c).cloned())
            .collect();
        
        (cmdtext, params)
    }

    /// 截取错误信息，防止递归膨胀
    /// MySQL错误信息可能包含完整SQL（含原始lasterrinfo），导致每次失败后lasterrinfo越来越大
    fn truncate_errinfo(errinfo: &str) -> String {
        const MAX_LEN: usize = 500;
        let escaped = errinfo.replace("'", "''");
        if escaped.len() > MAX_LEN {
            // 使用字符边界截取，避免在多字节UTF-8字符中间截断
            let truncated: String = escaped.chars().take(MAX_LEN).collect();
            format!("{}...[TRUNCATED]", truncated)
        } else {
            escaped
        }
    }

    /// 标记成功（接受 idrow 列表）
    pub fn mark_synced_by_idrows(&self, idrow_list: &[String]) -> Result<(), String> {
        if idrow_list.is_empty() {
            return Ok(());
        }

        let tables = self.get_default_query_tables();
        let up = UpInfo::new();
        let placeholders: Vec<String> = idrow_list.iter().map(|_| "?".to_string()).collect();

        for table_name in tables {
            let sql = format!(
                "UPDATE {} SET synced = 1 WHERE idrow IN ({})",
                table_name,
                placeholders.join(", ")
            );
            let mut params: Vec<&dyn rusqlite::ToSql> = Vec::new();
            for idrow in idrow_list {
                params.push(idrow);
            }
            let _ = self.db.do_m(&sql, &params, &up);
        }

        Ok(())
    }

    /// 添加到同步队列（配合 DataSync 使用）
    pub fn add_to_synclog(
        &self,
        tbname: &str,
        record_id: &str,
        action: &str,
        cmdtext: &str,
        params: &str,
        worker: &str,
        cid: &str,
    ) -> Result<i64, String> {
        let table_name = self.get_table_name();
        self.create_today_table()?;

        let uptime = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let cmdtextmd5 = format!("{:x}", md5::compute(cmdtext));
        let id = crate::snowflake::next_id_string();
        let up = UpInfo::new();

        // 首先检查是否已有未同步的记录
        let check_tables = self.get_default_query_tables();
        let mut existing_idpk: Option<i64> = None;

        for check_table in &check_tables {
            let check_sql = format!(
                "SELECT idpk FROM {} WHERE tbname = ? AND idrow = ? AND synced = 0 LIMIT 1",
                check_table
            );
            let result = self.db.do_get(
                &check_sql,
                &[&tbname as &dyn rusqlite::ToSql, &record_id as &dyn rusqlite::ToSql],
                &up,
            );
            if let Ok(rows) = result {
                if let Some(row) = rows.first() {
                    if let Some(idpk) = row.get("idpk").and_then(|v| v.as_i64()) {
                        existing_idpk = Some(idpk);
                        break;
                    }
                }
            }
        }

        if let Some(idpk) = existing_idpk {
            // 更新现有记录（找到它所在的表）
            let update_tables = self.get_default_query_tables();
            let mut updated = false;

            for update_table in update_tables {
                let update_sql = format!(
                    "UPDATE {} SET action = ?, cmdtext = ?, params = ?, cmdtextmd5 = ?, upby = ?, uptime = ? WHERE idpk = ?",
                    update_table
                );
                let result = self.db.do_m(
                    &update_sql,
                    &[
                        &action as &dyn rusqlite::ToSql,
                        &cmdtext as &dyn rusqlite::ToSql,
                        &params as &dyn rusqlite::ToSql,
                        &cmdtextmd5 as &dyn rusqlite::ToSql,
                        &worker as &dyn rusqlite::ToSql,
                        &uptime as &dyn rusqlite::ToSql,
                        &idpk as &dyn rusqlite::ToSql,
                    ],
                    &up,
                );
                if result.is_ok() {
                    updated = true;
                    break;
                }
            }

            if updated {
                Ok(idpk)
            } else {
                Err("更新失败".to_string())
            }
        } else {
            // 插入新记录到今天的表
            let insert_sql = format!(
                "REPLACE INTO {} (\
                    id, apisys, apimicro, apiobj, tbname, action,\
                    cmdtext, params, idrow, worker, synced, cmdtextmd5, cid, upby, uptime\
                ) VALUES (?, 'v1', 'iflow', 'synclog', ?, ?, ?, ?, ?, ?, 0, ?, ?, ?, ?)",
                table_name
            );

            self.db.do_m_add(
                &insert_sql,
                &[
                    &id as &dyn rusqlite::ToSql,
                    &tbname as &dyn rusqlite::ToSql,
                    &action as &dyn rusqlite::ToSql,
                    &cmdtext as &dyn rusqlite::ToSql,
                    &params as &dyn rusqlite::ToSql,
                    &record_id as &dyn rusqlite::ToSql,
                    &worker as &dyn rusqlite::ToSql,
                    &cmdtextmd5 as &dyn rusqlite::ToSql,
                    &cid as &dyn rusqlite::ToSql,
                    &worker as &dyn rusqlite::ToSql,
                    &uptime as &dyn rusqlite::ToSql,
                ],
                &up,
            )?;

            // 获取插入的 idpk
            let conn = self.db.get_conn()?;
            let conn_guard = conn.lock().map_err(|e| e.to_string())?;
            Ok(conn_guard.last_insert_rowid())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_synclog_basic() {
        let synclog = Synclog::with_default_path()
            .expect("创建失败");

        let table_name = synclog.get_table_name();
        assert!(table_name.starts_with("synclog_"), "表名应该是分表格式");
    }
}
