//! Config - 配置管理实现
//!
//! 单例模式，支持 JSON 文件加载

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use serde_json::Value;

use crate::table_config::{TableSet, TableConfigManager};

/// 全局配置实例
static CONFIG_INSTANCE: OnceLock<Arc<Config>> = OnceLock::new();

/// 配置错误
#[derive(Debug)]
pub enum ConfigError {
    /// 文件不存在
    FileNotFound(String),
    /// 解析错误
    ParseError(String),
    /// 配置未初始化
    NotInitialized,
    /// 键不存在
    KeyNotFound(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::FileNotFound(path) => write!(f, "配置文件不存在: {}", path),
            ConfigError::ParseError(msg) => write!(f, "配置解析失败: {}", msg),
            ConfigError::NotInitialized => write!(f, "配置未初始化"),
            ConfigError::KeyNotFound(key) => write!(f, "配置键不存在: {}", key),
        }
    }
}

impl std::error::Error for ConfigError {}

/// 配置结构
#[derive(Debug, Clone)]
pub struct Config {
    /// 配置对象
    config_object: HashMap<String, Value>,
    /// 表配置管理器
    tables: TableConfigManager,
    /// 项目根目录
    project_root: PathBuf,
}

impl Config {
    /// 创建新的配置实例
    fn new() -> Self {
        Self {
            config_object: HashMap::new(),
            tables: TableConfigManager::new(),
            project_root: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }

    /// 初始化配置
    pub fn init(&mut self, config_file: Option<&str>) -> Result<(), ConfigError> {
        let config_path = config_file
            .map(|s| s.to_string())
            .or_else(|| std::env::var("CONFIG_FILE").ok())
            .or_else(|| {
                let env = std::env::var("APP_ENV").unwrap_or_else(|_| "development".to_string());
                Some(format!("config/{}.json", env))
            });

        if let Some(path) = config_path {
            let full_path = if Path::new(&path).is_absolute() {
                PathBuf::from(&path)
            } else {
                self.project_root.join(&path)
            };

            if full_path.exists() {
                self.load_config_file(&full_path)?;
            } else {
                self.load_default_tables();
            }
        } else {
            self.load_default_tables();
        }

        Ok(())
    }

    /// 从文件加载配置
    fn load_config_file(&mut self, path: &Path) -> Result<(), ConfigError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| ConfigError::ParseError(format!("读取文件失败: {}", e)))?;

        let json: Value = serde_json::from_str(&content)
            .map_err(|e| ConfigError::ParseError(format!("JSON 解析失败: {}", e)))?;

        if let Value::Object(map) = json.clone() {
            for (k, v) in map {
                if k == "tables" || k == "tableConfigs" {
                    let tables_json = serde_json::to_string(&v).unwrap_or_default();
                    self.tables.load_from_json(&tables_json)
                        .map_err(|e| ConfigError::ParseError(e.to_string()))?;
                } else {
                    self.config_object.insert(k, v);
                }
            }
        }

        if self.tables.table_names().is_empty() {
            self.load_default_tables();
        }

        Ok(())
    }

    /// 加载默认表配置
    fn load_default_tables(&mut self) {
    }

    /// 获取配置项
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.config_object.get(key)
    }

    /// 获取字符串配置
    pub fn get_string(&self, key: &str) -> Option<String> {
        self.get(key).and_then(|v| v.as_str().map(|s| s.to_string()))
    }

    /// 获取整数配置
    pub fn get_int(&self, key: &str) -> Option<i64> {
        self.get(key).and_then(|v| v.as_i64())
    }

    /// 获取布尔配置
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.get(key).and_then(|v| v.as_bool())
    }

    /// 检查配置项是否存在
    pub fn has(&self, key: &str) -> bool {
        self.config_object.contains_key(key)
    }

    /// 获取表配置
    pub fn get_table(&self, table_name: &str) -> Option<&TableSet> {
        self.tables.get(table_name)
    }

    /// 设置配置项
    pub fn set(&mut self, key: &str, value: Value) {
        self.config_object.insert(key.to_string(), value);
    }

    /// 获取所有表名
    pub fn table_names(&self) -> Vec<&String> {
        self.tables.table_names()
    }

    /// 获取项目根目录
    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    /// 创建新的配置实例 (非单例)
    pub fn new_instance() -> Self {
        Self::new()
    }

    /// 获取全局配置实例 (单例)
    pub fn get_instance() -> Arc<Config> {
        CONFIG_INSTANCE.get_or_init(|| {
            let mut config = Config::new();
            config.init(None).ok();
            Arc::new(config)
        }).clone()
    }

    /// 初始化并设置全局实例
    pub fn init_global(config_file: Option<&str>) -> Result<Arc<Config>, ConfigError> {
        let mut config = Config::new();
        config.init(config_file)?;
        let arc_config = Arc::new(config);
        let _ = CONFIG_INSTANCE.set(arc_config.clone());
        Ok(arc_config)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_config_new() {
        let config = Config::new_instance();
        assert!(config.get("notexist").is_none());
    }

    #[test]
    fn test_config_set_get() {
        let mut config = Config::new_instance();
        config.set("key1", json!("value1"));
        config.set("key2", json!(123));
        config.set("key3", json!(true));

        assert_eq!(config.get_string("key1"), Some("value1".to_string()));
        assert_eq!(config.get_int("key2"), Some(123));
        assert_eq!(config.get_bool("key3"), Some(true));
    }

    #[test]
    fn test_config_has() {
        let mut config = Config::new_instance();
        assert!(!config.has("key1"));

        config.set("key1", json!("value1"));
        assert!(config.has("key1"));
    }

    #[test]
    fn test_config_get_table_not_exist() {
        let config = Config::new_instance();
        assert!(config.get_table("notexist").is_none());
    }

    #[test]
    fn test_config_singleton() {
        let config1 = Config::get_instance();
        let config2 = Config::get_instance();
        assert!(Arc::ptr_eq(&config1, &config2));
    }

    #[test]
    fn test_config_table_names() {
        let config = Config::new_instance();
        let names = config.table_names();
        assert!(names.is_empty());
    }

    #[test]
    fn test_config_error_display() {
        let err = ConfigError::FileNotFound("test.json".to_string());
        assert!(err.to_string().contains("test.json"));

        let err = ConfigError::ParseError("invalid json".to_string());
        assert!(err.to_string().contains("invalid json"));

        let err = ConfigError::NotInitialized;
        assert!(err.to_string().contains("未初始化"));
    }
}
