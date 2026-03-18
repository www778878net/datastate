//! 已发送记录表 DataService
//!
//! 用于记录已发送的营销内容，按标题+平台去重

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{DataAudit, DataState, LocalDB, snowflake, DataSync};

/// marketing_sent 表建表 SQL (SQLite版本)
pub const SENT_CREATE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS marketing_sent (
    id TEXT NOT NULL PRIMARY KEY,
    cid TEXT NOT NULL DEFAULT '',
    platform TEXT NOT NULL DEFAULT '',
    title TEXT NOT NULL DEFAULT '',
    url TEXT NOT NULL DEFAULT '',
    content TEXT NOT NULL DEFAULT '',
    keyword TEXT NOT NULL DEFAULT '',
    senttime TEXT NOT NULL DEFAULT '',
    uptime TEXT NOT NULL DEFAULT '',
    upby TEXT NOT NULL DEFAULT ''
)
"#;

/// 已发送记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentRecord {
    pub id: String,
    pub cid: String,
    pub platform: String,
    pub title: String,
    pub url: String,
    pub content: String,
    pub keyword: String,
    pub senttime: String,
    pub uptime: String,
    pub upby: String,
}

/// Sent - 已发送记录表 DataService
pub struct MarketingSent {
    pub db: Local