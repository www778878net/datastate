//! SysSqlData - SQL 统计数据结构
//!
//! SQLite 和 MySQL 共用，字段名一致

use serde::{Deserialize, Serialize};

/// sys_sql 表数据结构
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SysSqlData {
    /// 业务主键
    pub id: String,
    /// 公司ID
    pub cid: String,
    /// 系统名
    pub apisys: String,
    /// 微服务名
    pub apimicro: String,
    /// 对象名（表名）
    pub apiobj: String,
    /// SQL 语句
    pub cmdtext: String,
    /// 用户名
    pub uname: String,
    /// 执行次数
    pub num: i64,
    /// 总耗时(ms)
    pub dlong: i64,
    /// 下载数据量
    pub downlen: i64,
    /// 上传者
    pub upby: String,
    /// SQL MD5
    pub cmdtextmd5: String,
    /// 更新时间
    pub uptime: String,
}
