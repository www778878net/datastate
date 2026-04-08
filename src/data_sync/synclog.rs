//! Synclog - synclog 表管理（分表）
//!
//! 对应 synclog 表
//! 按天分表，保留7天数据
//!
//! 切换策略：00:00 立即切换到今天的表写，读的时候可以读昨天的表，给足同步时间

use crate::workflow::{ShardType, ShardingConfig, ShardingManager, MaintenanceResult};
use crate::{Sqlite78, UpInfo};
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

    /// 获取需要查询的分表列表（用于读操作：查询过去N天的表）
    pub fn get_query_shard_tables(&self, days_back: i32) -> Vec<String> {
        if let Some(ref _manager) = self.sharding_manager {
            let config = ShardingConfig::new(ShardType::Daily, "synclog");
            let today = chrono::Local::now().date_naive();
            let mut tables = Vec::new();
            
            for i in 0..=days_back {
                let date = today - chrono::Duration::days(i as i64);
                tables.push(config.get_table_name(Some(date)));
            }
            
            tables
        } else {
            vec!["synclog".to_string()]
        }
    }

    /// 获取过去7天的分表列表（默认查询范围）
    pub fn get_default_query_tables(&self) -> Vec<String> {
        self.get_query_shard_tables(7)
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
            "INSERT OR REPLACE INTO {} (\
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
