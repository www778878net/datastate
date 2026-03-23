//! 雪花算法 ID 生成器
//!
//! 生成分布式唯一ID，64位整数，字符串形式存储
//! 结构：时间戳(41位) + 机器ID(10位) + 序列号(12位)
//!
//! worker_id基于UUID自动生成，确保不同进程有不同的worker_id

#![allow(static_mut_refs)]

use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const EPOCH: i64 = 1704067200000; // 2024-01-01 00:00:00 UTC
const WORKER_ID_BITS: i64 = 10;
const SEQUENCE_BITS: i64 = 12;

const MAX_WORKER_ID: i64 = (1 << WORKER_ID_BITS) - 1;
const MAX_SEQUENCE: i64 = (1 << SEQUENCE_BITS) - 1;

const WORKER_ID_SHIFT: i64 = SEQUENCE_BITS;
const TIMESTAMP_SHIFT: i64 = SEQUENCE_BITS + WORKER_ID_BITS;

static LAST_TIMESTAMP: AtomicI64 = AtomicI64::new(0);
static SEQUENCE: AtomicI64 = AtomicI64::new(0);
static WORKER_ID: AtomicU64 = AtomicU64::new(0);

/// 初始化worker_id（基于UUID自动生成）
fn init_worker_id_auto() {
    let uuid = uuid::Uuid::new_v4();
    let bytes = uuid.as_bytes();
    let hash = ((bytes[0] as u64) << 24) | ((bytes[1] as u64) << 16) | ((bytes[2] as u64) << 8) | (bytes[3] as u64);
    let worker_id = hash % ((MAX_WORKER_ID + 1) as u64);
    WORKER_ID.store(worker_id, Ordering::SeqCst);
}

/// 手动设置机器ID
/// 
/// # 参数
/// - `worker_id`: 机器ID，范围 0-1023
/// 
/// # Panics
/// 如果 worker_id 超出范围会 panic
pub fn init_worker_id(worker_id: u64) {
    if worker_id > MAX_WORKER_ID as u64 {
        panic!("worker_id 超出范围: {}", worker_id);
    }
    WORKER_ID.store(worker_id, Ordering::SeqCst);
}

/// 获取当前worker_id
pub fn get_worker_id() -> u64 {
    WORKER_ID.load(Ordering::SeqCst)
}

/// 生成下一个雪花ID（i64）
/// 
/// # 返回
/// 64位整数ID
/// 
/// # Panics
/// 如果时钟回拨会 panic
pub fn next_id() -> i64 {
    let now = current_millis();
    
    if WORKER_ID.load(Ordering::SeqCst) == 0 {
        init_worker_id_auto();
    }
    
    let last = LAST_TIMESTAMP.load(Ordering::SeqCst);
    
    if now < last {
        panic!("时钟回拨，拒绝生成ID");
    }
    
    if now == last {
        let seq = SEQUENCE.fetch_add(1, Ordering::SeqCst);
        if seq > MAX_SEQUENCE {
            wait_next_millis(last);
            SEQUENCE.store(0, Ordering::SeqCst);
            LAST_TIMESTAMP.store(now, Ordering::SeqCst);
        }
    } else {
        SEQUENCE.store(0, Ordering::SeqCst);
        LAST_TIMESTAMP.store(now, Ordering::SeqCst);
    }
    
    let worker_id = WORKER_ID.load(Ordering::SeqCst) as i64;
    let seq = SEQUENCE.load(Ordering::SeqCst);
    
    ((now - EPOCH) << TIMESTAMP_SHIFT) | (worker_id << WORKER_ID_SHIFT) | seq
}

/// 生成下一个雪花ID（String）
/// 
/// # 返回
/// 字符串形式的ID
pub fn next_id_string() -> String {
    next_id().to_string()
}

fn current_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

fn wait_next_millis(last: i64) -> i64 {
    let mut now = current_millis();
    while now <= last {
        now = current_millis();
    }
    now
}

#[cfg(test)]
mod tests {
    use super::*;
    use base::mylogger;

    #[test]
    fn test_next_id() {
        let id1 = next_id();
        let id2 = next_id();
        assert!(id2 > id1);
        let logger = mylogger!();
        logger.detail(&format!("ID1: {}", id1));
        logger.detail(&format!("ID2: {}", id2));
        logger.detail(&format!("worker_id: {}", get_worker_id()));
    }

    #[test]
    fn test_next_id_string() {
        let id = next_id_string();
        assert!(!id.is_empty());
        let logger = mylogger!();
        logger.detail(&format!("ID String: {}", id));
        logger.detail(&format!("worker_id: {}", get_worker_id()));
    }

    #[test]
    fn test_worker_id_auto() {
        let id = next_id();
        let worker_id = get_worker_id();
        assert!(worker_id <= MAX_WORKER_ID as u64);
        let logger = mylogger!();
        logger.detail(&format!("自动生成的worker_id: {}", worker_id));
        logger.detail(&format!("生成的ID: {}", id));
    }
}
