//! SysWarnData - 警告日志数据结构
//!
//! SQLite 和 MySQL 共用，字段名一致

use serde::{Deserialize, Serialize};

/// sys_warn 表数据结构
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SysWarnData {
    /// 业务主键
    pub id: String,
    /// 用户ID
    pub uid: String,
    /// 日志类型
    pub kind: String,
    /// 微服务名
    pub apimicro: String,
    /// 对象名
    pub apiobj: String,
    /// 内容
    pub content: String,
    /// 上传者ID
    pub upid: String,
    /// 上传者
    pub upby: String,
    /// 更新时间
    pub uptime: String,
    /// 备注1
    pub remark: String,
    /// 备注2
    pub remark2: String,
    /// 备注3
    pub remark3: String,
    /// 备注4
    pub remark4: String,
    /// 备注5
    pub remark5: String,
    /// 备注6
    pub remark6: String,
}

impl SysWarnData {
    /// 创建新实例
    pub fn new() -> Self {
        Self::default()
    }

    /// 生成新ID
    pub fn new_id() -> String {
        format!("{}{:06x}",
            chrono::Local::now().format("%Y%m%d%H%M%S"),
            rand_suffix()
        )
    }
}

fn rand_suffix() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    ns % 0xFFFFFF
}