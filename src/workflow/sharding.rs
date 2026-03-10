//! Sharding - 分表功能模块
//!
//! 参考 koa78-base78 的分表实现
//! 支持 daily（按天）和 monthly（按月）分表

use chrono::{Local, NaiveDate, Duration};
use std::sync::{Arc, Mutex};
use rusqlite::Connection;

/// 分表类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShardType {
    /// 按天分表
    Daily,
    /// 按月分表
    Monthly,
    /// 不分表
    None,
}

impl Default for ShardType {
    fn default() -> Self {
        Self::None
    }
}

/// 分表配置
#[derive(Debug, Clone)]
pub struct ShardingConfig {
    /// 分表类型
    pub shard_type: ShardType,
    /// 建表 SQL 模板，使用 {TABLE_NAME} 作为表名占位符
    pub table_sql: Option<String>,
    /// 保留天数/月数
    pub retention_days: i32,
    /// 基础表名
    pub base_table: String,
}

impl Default for ShardingConfig {
    fn default() -> Self {
        Self {
            shard_type: ShardType::None,
            table_sql: None,
            retention_days: 7,  // 默认保留7天
            base_table: String::new(),
        }
    }
}

impl ShardingConfig {
    /// 创建新的分表配置
    pub fn new(shard_type: ShardType, base_table: &str) -> Self {
        Self {
            shard_type,
            table_sql: None,
            retention_days: 5,
            base_table: base_table.to_string(),
        }
    }

    /// 设置建表 SQL
    pub fn with_table_sql(mut self, sql: &str) -> Self {
        self.table_sql = Some(sql.to_string());
        self
    }

    /// 设置保留天数
    pub fn with_retention(mut self, days: i32) -> Self {
        self.retention_days = days;
        self
    }

    /// 获取动态表名（带日期后缀）
    pub fn get_table_name(&self, date: Option<NaiveDate>) -> String {
        if self.shard_type == ShardType::None {
            return self.base_table.clone();
        }

        let target_date = date.unwrap_or_else(|| Local::now().date_naive());
        let suffix = match self.shard_type {
            ShardType::Daily => target_date.format("%Y%m%d").to_string(),
            ShardType::Monthly => target_date.format("%Y%m").to_string(),
            ShardType::None => return self.base_table.clone(),
        };

        format!("{}_{}", self.base_table, suffix)
    }

    /// 获取当前表名
    pub fn get_current_table_name(&self) -> String {
        self.get_table_name(None)
    }

    /// 获取日期后缀列表（用于查询多表）
    pub fn get_date_suffixes(&self, days_back: i32) -> Vec<String> {
        let mut suffixes = Vec::new();
        let today = Local::now().date_naive();

        for i in 0..=days_back {
            let date = today - Duration::days(i as i64);
            let suffix = match self.shard_type {
                ShardType::Daily => date.format("%Y%m%d").to_string(),
                ShardType::Monthly => date.format("%Y%m").to_string(),
                ShardType::None => return vec![self.base_table.clone()],
            };
            suffixes.push(format!("{}_{}", self.base_table, suffix));
        }

        suffixes
    }
}

/// 分表管理器
pub struct ShardingManager {
    conn: Arc<Mutex<Connection>>,
    config: ShardingConfig,
    /// 最后维护日期（表名 -> 日期字符串）
    last_maintenance: std::collections::HashMap<String, String>,
}

impl ShardingManager {
    /// 创建分表管理器
    pub fn new(conn: Arc<Mutex<Connection>>, config: ShardingConfig) -> Self {
        Self {
            conn,
            config,
            last_maintenance: std::collections::HashMap::new(),
        }
    }

    /// 创建分表
    pub fn create_sharding_table(&self, table_name: &str) -> Result<(), String> {
        let sql = self.config.table_sql.as_ref()
            .ok_or_else(|| "分表建表SQL未定义".to_string())?;

        let final_sql = sql.replace("{TABLE_NAME}", table_name);

        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(&final_sql, [])
            .map_err(|e| format!("创建分表 {} 失败: {}", table_name, e))?;

        Ok(())
    }

    /// 删除旧表
    pub fn drop_old_table(&self, table_name: &str) -> Result<bool, String> {
        // 先检查表是否存在
        let check_sql = format!(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='{}'",
            table_name
        );

        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let exists: bool = conn.query_row(&check_sql, [], |_| Ok(true)).unwrap_or(false);

        if !exists {
            return Ok(false);
        }

        let drop_sql = format!("DROP TABLE IF EXISTS \"{}\"", table_name);
        conn.execute(&drop_sql, [])
            .map_err(|e| format!("删除表 {} 失败: {}", table_name, e))?;

        Ok(true)
    }

    /// 检查表是否存在
    pub fn table_exists(&self, table_name: &str) -> Result<bool, String> {
        let check_sql = format!(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='{}'",
            table_name
        );

        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let exists = conn.query_row(&check_sql, [], |_| Ok(true)).unwrap_or(false);

        Ok(exists)
    }

    /// 执行分表维护
    /// 1. 创建当前表
    /// 2. 创建未来几天的表
    /// 3. 删除过期表
    pub fn perform_maintenance(&mut self) -> Result<MaintenanceResult, String> {
        if self.config.shard_type == ShardType::None {
            return Ok(MaintenanceResult::default());
        }

        let today = Local::now().format("%Y-%m-%d").to_string();
        let key = self.config.base_table.clone();

        // 如果今天已经执行过维护，跳过
        if self.last_maintenance.get(&key) == Some(&today) {
            return Ok(MaintenanceResult::default());
        }

        let mut result = MaintenanceResult::default();
        let retention = self.config.retention_days;

        // 1. 创建今天到未来 retention_days 天的表
        for i in 0..=retention {
            let date = Local::now().date_naive() + Duration::days(i as i64);
            let table_name = self.config.get_table_name(Some(date));

            if !self.table_exists(&table_name)? {
                self.create_sharding_table(&table_name)?;
                result.tables_created += 1;
            }
        }

        // 2. 删除 retention_days 天前的旧表
        let old_date = Local::now().date_naive() - Duration::days((retention + 1) as i64);
        let old_table = self.config.get_table_name(Some(old_date));

        if self.drop_old_table(&old_table)? {
            result.tables_dropped += 1;
        }

        // 更新维护日期
        self.last_maintenance.insert(key, today);

        Ok(result)
    }

    /// 获取所有分表名称
    pub fn get_all_shard_tables(&self) -> Result<Vec<String>, String> {
        let pattern = format!("{}_%", self.config.base_table);
        let sql = format!(
            "SELECT name FROM sqlite_master WHERE type='table' AND name LIKE '{}' ORDER BY name",
            pattern
        );

        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;

        let tables: Vec<String> = stmt.query_map([], |row| row.get(0))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();

        Ok(tables)
    }
}

/// 维护结果
#[derive(Debug, Default)]
pub struct MaintenanceResult {
    /// 创建的表数量
    pub tables_created: u32,
    /// 删除的表数量
    pub tables_dropped: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sharding_config_daily() {
        let config = ShardingConfig::new(ShardType::Daily, "workflow_instance")
            .with_retention(5);

        let table_name = config.get_current_table_name();
        assert!(table_name.starts_with("workflow_instance_"));

        // 测试指定日期
        let date = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
        let specific_name = config.get_table_name(Some(date));
        assert_eq!(specific_name, "workflow_instance_20260301");
    }

    #[test]
    fn test_sharding_config_monthly() {
        let config = ShardingConfig::new(ShardType::Monthly, "workflow_task")
            .with_retention(3);

        let date = NaiveDate::from_ymd_opt(2026, 3, 15).unwrap();
        let table_name = config.get_table_name(Some(date));
        assert_eq!(table_name, "workflow_task_202603");
    }

    #[test]
    fn test_sharding_config_none() {
        let config = ShardingConfig::new(ShardType::None, "workflow_capability");
        let table_name = config.get_current_table_name();
        assert_eq!(table_name, "workflow_capability");
    }

    #[test]
    fn test_date_suffixes() {
        let config = ShardingConfig::new(ShardType::Daily, "workflow_instance");
        let suffixes = config.get_date_suffixes(3);

        assert_eq!(suffixes.len(), 4); // 今天 + 过去3天
        assert!(suffixes[0].starts_with("workflow_instance_"));
    }
}