//! TableSet - 表配置定义
//!
//! 定义数据库表的元信息

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 数据隔离类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UidCid {
    /// 公司级隔离
    Cid,
    /// 用户级隔离
    Uid,
}

impl Default for UidCid {
    fn default() -> Self {
        UidCid::Cid
    }
}

impl UidCid {
    /// 获取字段名
    pub fn field_name(&self) -> &'static str {
        match self {
            UidCid::Cid => "cid",
            UidCid::Uid => "uid",
        }
    }

    /// 从字符串解析
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "uid" => UidCid::Uid,
            _ => UidCid::Cid,
        }
    }
}

/// 表配置集合
///
/// 包含表的完整配置信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSet {
    /// 表名 (小写)
    pub tbname: String,
    /// 所有列 (包含系统列)
    pub cols: Vec<String>,
    /// 重要业务列 (不含系统列)
    pub cols_imp: Vec<String>,
    /// 数据隔离类型
    pub uidcid: UidCid,
}

impl TableSet {
    /// 创建新的表配置
    pub fn new(tbname: &str, cols_imp: Vec<String>, uidcid: UidCid) -> Self {
        let cols = cols_imp.clone();

        Self {
            tbname: tbname.to_lowercase(),
            cols,
            cols_imp,
            uidcid,
        }
    }

    /// 获取隔离字段名
    pub fn isolation_field(&self) -> &'static str {
        self.uidcid.field_name()
    }

    /// 检查列是否在配置中
    pub fn has_col(&self, col: &str) -> bool {
        self.cols.contains(&col.to_string()) || Self::system_cols().contains(&col)
    }

    /// 获取系统列
    pub fn system_cols() -> &'static [&'static str] {
        &["id", "idpk", "upby", "uptime"]
    }

    /// 获取 INSERT 语句的列部分
    pub fn insert_columns(&self, colp: &[&str]) -> String {
        let quoted: Vec<String> = colp.iter().map(|c| format!("`{}`", c)).collect();
        quoted.join(",")
    }

    /// 获取 INSERT 语句的值占位符
    pub fn insert_placeholders(count: usize) -> String {
        vec!["?"; count].join(",")
    }
}

/// 单个表配置 (从 JSON 加载)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableConfigJson {
    /// 重要业务列
    pub cols_imp: Vec<String>,
    /// 数据隔离类型
    pub uidcid: UidCid,
    /// API 系统名
    #[serde(default)]
    pub apisys: String,
    /// API 微服务名
    #[serde(default)]
    pub apimicro: String,
}

impl TableConfigJson {
    /// 转换为 TableSet
    pub fn to_table_set(&self, tbname: &str) -> TableSet {
        TableSet::new(tbname, self.cols_imp.clone(), self.uidcid)
    }
}

/// 表配置管理器
///
/// 管理所有表的配置
#[derive(Debug, Clone, Default)]
pub struct TableConfigManager {
    configs: HashMap<String, TableSet>,
}

impl TableConfigManager {
    /// 创建新的配置管理器
    pub fn new() -> Self {
        Self {
            configs: HashMap::new(),
        }
    }

    /// 注册表配置
    pub fn register(&mut self, table_set: TableSet) {
        self.configs.insert(table_set.tbname.clone(), table_set);
    }

    /// 获取表配置
    pub fn get(&self, tbname: &str) -> Option<&TableSet> {
        self.configs.get(&tbname.to_lowercase())
    }

    /// 从 JSON 加载配置
    pub fn load_from_json(&mut self, json: &str) -> Result<(), serde_json::Error> {
        let configs: HashMap<String, TableConfigJson> = serde_json::from_str(json)?;
        for (name, config) in configs {
            let table_set = config.to_table_set(&name);
            self.configs.insert(table_set.tbname.clone(), table_set);
        }
        Ok(())
    }

    /// 获取所有表名
    pub fn table_names(&self) -> Vec<&String> {
        self.configs.keys().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TableConfig;

    #[test]
    fn test_uidcid() {
        assert_eq!(UidCid::Cid.field_name(), "cid");
        assert_eq!(UidCid::Uid.field_name(), "uid");
        assert_eq!(UidCid::from_str("cid"), UidCid::Cid);
        assert_eq!(UidCid::from_str("uid"), UidCid::Uid);
    }

    #[test]
    fn test_table_set_new() {
        let ts = TableSet::new(
            "test_table",
            vec!["field1".to_string(), "field2".to_string()],
            UidCid::Cid,
        );

        assert_eq!(ts.tbname, "test_table");
        assert!(ts.cols_imp.contains(&"field1".to_string()));
        assert_eq!(ts.isolation_field(), "cid");
    }

    #[test]
    fn test_table_set_columns() {
        let ts = TableSet::new(
            "user",
            vec!["name".to_string(), "email".to_string()],
            UidCid::Uid,
        );

        let insert_cols = ts.insert_columns(&["name", "email"]);
        assert_eq!(insert_cols, "`name`,`email`");

        let placeholders = TableSet::insert_placeholders(3);
        assert_eq!(placeholders, "?,?,?");
    }

    #[test]
    fn test_table_config() {
        let config = crate::sync_config::TableConfig {
            name: "test_table".to_string(),
            apiurl: "http://test".to_string(),
            download_cols: Some(vec!["field1".to_string()]),
            uidcid: "cid".to_string(),
            ..Default::default()
        };
        assert_eq!(config.name, "test_table");
        assert_eq!(config.download_cols, Some(vec!["field1".to_string()]));
        assert_eq!(config.uidcid, "cid");
    }

    #[test]
    fn test_table_config_manager() {
        let mut manager = TableConfigManager::new();

        let ts = TableSet::new(
            "test",
            vec!["field1".to_string()],
            UidCid::Cid,
        );

        manager.register(ts);

        assert!(manager.get("test").is_some());
        assert!(manager.get("TEST").is_some());
        assert!(manager.get("notexist").is_none());
    }

    #[test]
    fn test_table_config_load_json() {
        let mut manager = TableConfigManager::new();

        let json = r#"{
            "sys_ip": {
                "cols_imp": ["ip"],
                "uidcid": "cid",
                "apisys": "apitest",
                "apimicro": "testmenu"
            }
        }"#;

        manager.load_from_json(json).unwrap();

        let ts = manager.get("sys_ip").unwrap();
        assert_eq!(ts.cols_imp, vec!["ip"]);
    }

    #[test]
    fn test_system_cols() {
        let cols = TableSet::system_cols();
        assert!(cols.contains(&"id"));
        assert!(cols.contains(&"idpk"));
        assert!(cols.contains(&"uptime"));
    }
}
