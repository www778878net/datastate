//! BaseSchema - 基础表结构定义
//!
//! 所有数据表的基础结构，包含系统字段

use serde::{Deserialize, Serialize};

/// 基础表结构
///
/// 所有业务表必须包含这些系统字段
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BaseSchema {
    /// 业务主键 (UUID)
    pub id: String,
    /// 自增主键
    pub idpk: i64,
    /// 操作人
    pub upby: String,
    /// 操作时间
    pub uptime: String,
}

impl BaseSchema {
    /// 创建新的 BaseSchema 实例
    pub fn new() -> Self {
        Self {
            id: crate::snowflake::next_id_string(),
            idpk: 0,
            upby: String::new(),
            uptime: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        }
    }

    /// 生成新 ID（使用雪花算法）
    pub fn new_id() -> String {
        crate::snowflake::next_id_string()
    }

    /// 获取所有系统字段名
    pub fn system_fields() -> &'static [&'static str] {
        &["id", "idpk", "upby", "uptime"]
    }
}

/// 公司级数据隔离 Schema
///
/// 通过 cid 字段实现多租户数据隔离
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CidSchema {
    /// 基础字段
    #[serde(flatten)]
    pub base: BaseSchema,
    /// 公司ID (数据隔离键)
    pub cid: String,
}

impl CidSchema {
    /// 创建新的 CidSchema 实例
    pub fn new(cid: &str) -> Self {
        Self {
            base: BaseSchema::new(),
            cid: cid.to_string(),
        }
    }

    /// 获取隔离字段名
    pub fn isolation_field() -> &'static str {
        "cid"
    }
}

/// 用户级数据隔离 Schema
///
/// 通过 uid 字段实现用户级数据隔离
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UidSchema {
    /// 基础字段
    #[serde(flatten)]
    pub base: BaseSchema,
    /// 用户ID (数据隔离键)
    pub uid: String,
}

impl UidSchema {
    /// 创建新的 UidSchema 实例
    pub fn new(uid: &str) -> Self {
        Self {
            base: BaseSchema::new(),
            uid: uid.to_string(),
        }
    }

    /// 获取隔离字段名
    pub fn isolation_field() -> &'static str {
        "uid"
    }
}

/// Schema 类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaType {
    /// 公司级隔离
    Cid,
    /// 用户级隔离
    Uid,
}

impl SchemaType {
    /// 获取隔离字段名
    pub fn isolation_field(&self) -> &'static str {
        match self {
            SchemaType::Cid => "cid",
            SchemaType::Uid => "uid",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_schema_new() {
        let schema = BaseSchema::new();
        assert!(!schema.id.is_empty());
        assert_eq!(schema.idpk, 0);
    }

    #[test]
    fn test_base_schema_system_fields() {
        let fields = BaseSchema::system_fields();
        assert!(fields.contains(&"id"));
        assert!(fields.contains(&"idpk"));
        assert!(fields.contains(&"upby"));
        assert!(fields.contains(&"uptime"));
    }

    #[test]
    fn test_cid_schema_new() {
        let schema = CidSchema::new("test-cid-123");
        assert_eq!(schema.cid, "test-cid-123");
        assert!(!schema.base.id.is_empty());
    }

    #[test]
    fn test_uid_schema_new() {
        let schema = UidSchema::new("test-uid-456");
        assert_eq!(schema.uid, "test-uid-456");
        assert!(!schema.base.id.is_empty());
    }

    #[test]
    fn test_schema_type() {
        assert_eq!(SchemaType::Cid.isolation_field(), "cid");
        assert_eq!(SchemaType::Uid.isolation_field(), "uid");
    }

    #[test]
    fn test_base_schema_serialize() {
        let schema = BaseSchema::new();
        let json = serde_json::to_string(&schema).unwrap();
        assert!(json.contains("\"id\":"));
        assert!(json.contains("\"idpk\":"));
    }

    #[test]
    fn test_cid_schema_serialize() {
        let schema = CidSchema::new("company-123");
        let json = serde_json::to_string(&schema).unwrap();
        assert!(json.contains("\"cid\":\"company-123\""));
    }
}
