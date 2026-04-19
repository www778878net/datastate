//! SyncConfig - 同步配置模块
//!
//! 定义表同步策略和配置

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 同步策略
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[allow(non_camel_case_types)]
pub enum SyncPolicy {
    /// 实时同步（热数据，如用户消息）
    #[default]
    REALTIME,
    /// 批量同步（冷数据，如日志）
    BATCH,
    /// 仅本地存储（不同步，如草稿）
    LOCAL_ONLY,
}

/// 索引定义
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum IndexDef {
    /// 简单单列索引
    Simple(String),
    /// 复合索引
    Composite {
        name: String,
        columns: Vec<String>,
        #[serde(default)]
        unique: bool,
    },
}

/// 表配置 - 包含建表和同步所有配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableConfig {
    /// 表名
    pub name: String,
    /// API地址
    #[serde(default)]
    pub apiurl: String,
    /// 同步策略
    #[serde(default)]
    pub sync_policy: SyncPolicy,

    /// 下载间隔(秒)
    #[serde(default = "default_download_interval")]
    pub download_interval: i64,
    /// 上传间隔(秒)
    #[serde(default = "default_upload_interval")]
    pub upload_interval: i64,

    /// 初始化时总共下载多少行
    #[serde(default = "default_init_getnumber")]
    pub init_getnumber: i32,
    /// 定时同步时每次下载多少行
    #[serde(default = "default_getnumber")]
    pub getnumber: i32,

    /// 下载查询条件
    #[serde(default)]
    pub download_condition: Option<serde_json::Value>,
    /// 指定字段顺序
    #[serde(default)]
    pub download_cols: Option<Vec<String>>,
    /// 上传字段顺序（必须与服务器 colsImp 一致）
    #[serde(default)]
    pub upload_cols: Option<Vec<String>>,
    /// 最小待处理数量（用于任务表）
    #[serde(default)]
    pub min_pending: i32,

    /// 列定义 {列名: 类型或完整定义}
    #[serde(default)]
    pub columns: HashMap<String, String>,
    /// 主键列名
    #[serde(default = "default_primary_key")]
    pub primary_key: String,
    /// 索引
    #[serde(default)]
    pub indexes: Vec<IndexDef>,

    /// 是否按天分表
    #[serde(default)]
    pub partition_by_day: bool,
    /// 保留天数（0=永久保留，>0=自动清理过期表）
    #[serde(default)]
    pub retention_days: i32,

    /// 隔离字段类型：cid(默认)=公司隔离, uid=用户隔离, 空=公共表
    #[serde(default = "default_uidcid")]
    pub uidcid: String,

    /// 是否启用下载（默认 true）
    #[serde(default = "default_true")]
    pub download_enabled: bool,

    /// 是否启用上传（默认 true）
    #[serde(default = "default_true")]
    pub upload_enabled: bool,

    /// 是否使用 Rust 版本的 synclog_mysql API（默认 false，使用 logsvc）
    #[serde(default)]
    pub use_rust_synclog: bool,

    /// Rust API 地址（当 use_rust_synclog=true 时使用）
    #[serde(default)]
    pub rust_api_url: String,
}

fn default_true() -> bool { true }
fn default_download_interval() -> i64 { 300 }
fn default_upload_interval() -> i64 { 300 }
fn default_init_getnumber() -> i32 { 0 }
fn default_getnumber() -> i32 { 2000 }
fn default_primary_key() -> String { "id".to_string() }
fn default_uidcid() -> String { "cid".to_string() }

/// 获取系统字段（所有表都有的基础字段）
pub fn get_system_columns() -> HashMap<String, String> {
    let mut cols = HashMap::new();
    cols.insert("idpk".to_string(), "INTEGER NOT NULL DEFAULT 0".to_string());
    cols.insert("id".to_string(), "TEXT NOT NULL PRIMARY KEY".to_string());
    cols.insert("cid".to_string(), "TEXT NOT NULL DEFAULT ''".to_string());
    cols.insert("upby".to_string(), "TEXT NOT NULL DEFAULT ''".to_string());
    cols.insert("uptime".to_string(), "TEXT NOT NULL DEFAULT ''".to_string());
    cols
}

impl TableConfig {
    /// 创建新的表配置
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    /// 设置 API URL
    pub fn with_apiurl(mut self, apiurl: impl Into<String>) -> Self {
        self.apiurl = apiurl.into();
        self
    }

    /// 设置下载间隔
    pub fn with_download_interval(mut self, interval: i64) -> Self {
        self.download_interval = interval;
        self
    }

    /// 设置上传间隔
    pub fn with_upload_interval(mut self, interval: i64) -> Self {
        self.upload_interval = interval;
        self
    }

    /// 设置列定义
    pub fn with_columns(mut self, columns: HashMap<String, String>) -> Self {
        self.columns = columns;
        self
    }

    /// 设置索引
    pub fn with_indexes(mut self, indexes: Vec<IndexDef>) -> Self {
        self.indexes = indexes;
        self
    }

    /// 获取系统字段（固定，所有表都有）
    pub fn system_columns() -> HashMap<String, String> {
        let mut cols = HashMap::new();
        cols.insert("idpk".to_string(), "INTEGER NOT NULL DEFAULT 0".to_string());
        cols.insert("id".to_string(), "TEXT NOT NULL PRIMARY KEY".to_string());
        cols.insert("upby".to_string(), "TEXT NOT NULL DEFAULT ''".to_string());
        cols
    }

    /// 生成 CREATE TABLE SQL
    ///
    /// 规则：
    /// - 所有的表必须和logsvc服务器上的sqlfile字段名称和类型完全一致
    /// - json可换TEXT
    /// - 必须都要有默认值NOT NULL
    pub fn get_create_sql(&self) -> String {
        let columns_sql: Vec<String> = self.columns
            .iter()
            .map(|(col_name, col_type)| format!("    {} {}", col_name, col_type))
            .collect();

        let columns_str = columns_sql.join(",\n");

        format!(
            "CREATE TABLE IF NOT EXISTS {} (\n{}\n)",
            self.name, columns_str
        )
    }

    /// 生成 CREATE INDEX SQL，支持简单索引和复合索引
    pub fn get_index_sql(&self) -> Vec<String> {
        let mut sqls = Vec::new();
        for idx in &self.indexes {
            match idx {
                IndexDef::Simple(col) => {
                    sqls.push(format!(
                        "CREATE INDEX IF NOT EXISTS idx_{}_{} ON {}({})",
                        self.name, col, self.name, col
                    ));
                }
                IndexDef::Composite { name, columns, unique } => {
                    let unique_str = if *unique { "UNIQUE " } else { "" };
                    let columns_str = columns.join(", ");
                    sqls.push(format!(
                        "CREATE {}INDEX IF NOT EXISTS {} ON {}({})",
                        unique_str, name, self.name, columns_str
                    ));
                }
            }
        }
        sqls
    }
}

impl Default for TableConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            apiurl: String::new(),
            sync_policy: SyncPolicy::default(),
            download_interval: default_download_interval(),
            upload_interval: default_upload_interval(),
            init_getnumber: default_init_getnumber(),
            getnumber: default_getnumber(),
            download_condition: None,
            download_cols: None,
            upload_cols: None,
            min_pending: 0,
            columns: get_system_columns(),
            primary_key: default_primary_key(),
            indexes: Vec::new(),
            partition_by_day: false,
            retention_days: 0,
            uidcid: default_uidcid(),
            download_enabled: true,
            upload_enabled: true,
            use_rust_synclog: false,
            rust_api_url: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = TableConfig::default();
        assert_eq!(config.name, "");
        assert_eq!(config.download_interval, 300);
        assert_eq!(config.upload_interval, 300);
        assert_eq!(config.init_getnumber, 0);  // 默认值是 0
        assert_eq!(config.getnumber, 2000);    // 默认值是 2000
    }

    #[test]
    fn test_default_sync_policy() {
        let policy = SyncPolicy::default();
        assert_eq!(policy, SyncPolicy::REALTIME);
    }

    #[test]
    fn test_new_config() {
        let config = TableConfig::new("my_table");
        assert_eq!(config.name, "my_table");
    }

    #[test]
    fn test_chain_methods() {
        let config = TableConfig::new("my_table")
            .with_apiurl("http://example.com/api")
            .with_download_interval(60)
            .with_upload_interval(120);

        assert_eq!(config.apiurl, "http://example.com/api");
        assert_eq!(config.download_interval, 60);
        assert_eq!(config.upload_interval, 120);
    }

    #[test]
    fn test_system_columns() {
        let cols = TableConfig::system_columns();
        assert!(cols.contains_key("id"));
        assert!(cols.contains_key("idpk"));
        assert!(cols.contains_key("upby"));
    }
}