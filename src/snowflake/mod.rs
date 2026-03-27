//! 雪花算法模块
//!
//! 提供分布式唯一ID生成功能
//!
//! ## 使用方法
//!
//! ```rust
//! use datastate::snowflake::{next_id, next_id_string, get_worker_id, init_worker_id};
//!
//! // 自动生成ID（worker_id基于UUID自动生成）
//! let id = next_id();  // 返回 i64
//! let id_str = next_id_string();  // 返回 String
//! let worker_id = get_worker_id();  // 获取当前worker_id
//!
//! // 或手动设置worker_id
//! init_worker_id(1);  // 设置worker_id为1
//! ```

mod snowflake;

pub use snowflake::{get_worker_id, init_worker_id, next_id, next_id_string};
