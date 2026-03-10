//! 测试审计日志计数方式
//!
//! 验证新的计数方式：每天每个调用方对每个方法只有一条记录
//! 如果当天已有记录，则增加计数；否则创建新记录

use database::{
    LocalDB,
    dataaudit::{
        DataAudit, get_audit_logs, get_stats_by_date_range,
        AuditLogDataState,
    },
};

/// TestTbCount - 使用计数方式记录审计日志的测试表
struct TestTbCount {
    db: LocalDB,
    audit: DataAudit,
    audit_log_state: AuditLogDataState,
}

impl TestTbCount {
    fn new() -> Self {
        let db = LocalDB::default_instance().expect("获取数据库失败");
        let mut audit = DataAudit::new("testtb_count");
        audit.set_audit_enabled(false); // 关闭审计，只记录日志
        let audit_log_state = AuditLogDataState::new();

        Self { db, audit, audit_log_state }
    }

    fn init_tables(&self) -> Result<(), String> {
        // 初始化审计日志表（计数方式）
        self.audit_log_state.init_table()?;

        Ok(())
    }

    /// 使用计数方式记录调用
    fn do_action_with_count<F, T>(&self, ability: &str, caller: &str, summary: &str, action: F) -> Result<T, String>
    where
        F: FnOnce() -> Result<T, String>,
    {
        self.audit.do_action_with_count(&self.db, ability, caller, summary, action)
    }
}

#[test]
fn test_audit_log_count_mode() {
    println!("=== 测试审计日志计数方式 ===\n");

    // 使用唯一表名避免测试冲突
    let unique_table = format!("testtb_count_{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis());
    
    let db = LocalDB::default_instance().expect("获取数据库失败");
    let mut audit = DataAudit::new(&unique_table);
    audit.set_audit_enabled(false); // 关闭审计，只记录日志
    let audit_log_state = AuditLogDataState::new();
    audit_log_state.init_table().expect("初始化表失败");

    // 1. 第一次调用：创建新记录
    println!("【1】第一次调用：创建新记录");
    let result = audit.do_action_with_count(&db, "getone", "CallerA", "查询单条记录用于测试", || {
        Ok("result1".to_string())
    });
    assert!(result.is_ok(), "第一次调用应该成功");
    println!("  - 第一次调用成功: {}", result.unwrap());

    // 2. 第二次调用：增加计数
    println!("\n【2】第二次调用（同一天，同一调用方，同一方法）：增加计数");
    let result = audit.do_action_with_count(&db, "getone", "CallerA", "再次查询单条记录", || {
        Ok("result2".to_string())
    });
    assert!(result.is_ok(), "第二次调用应该成功");
    println!("  - 第二次调用成功: {}", result.unwrap());

    // 3. 第三次调用：再次增加计数
    println!("\n【3】第三次调用：再次增加计数");
    let result = audit.do_action_with_count(&db, "getone", "CallerA", "第三次查询", || {
        Ok("result3".to_string())
    });
    assert!(result.is_ok(), "第三次调用应该成功");
    println!("  - 第三次调用成功: {}", result.unwrap());

    // 4. 不同调用方：创建新记录
    println!("\n【4】不同调用方：创建新记录");
    let result = audit.do_action_with_count(&db, "getone", "CallerB", "CallerB查询记录", || {
        Ok("result4".to_string())
    });
    assert!(result.is_ok(), "不同调用方应该成功");
    println!("  - 不同调用方成功: {}", result.unwrap());

    // 5. 不同方法：创建新记录
    println!("\n【5】不同方法：创建新记录");
    let result = audit.do_action_with_count(&db, "save", "CallerA", "保存数据", || {
        Ok("result5".to_string())
    });
    assert!(result.is_ok(), "不同方法应该成功");
    println!("  - 不同方法成功: {}", result.unwrap());

    // 6. 查看审计日志
    println!("\n【6】查看审计日志");
    let logs = get_audit_logs(Some(&unique_table), 7);
    println!("  - 审计日志数量: {}", logs.len());
    for (i, log) in logs.iter().enumerate() {
        println!("  [{}] {}/{}/{} - 计数: {} - 摘要: {} - 时间: {}",
            i + 1, log.tablename, log.ability, log.caller, log.call_count, log.summary, log.last_call_time);
    }

    // 验证：应该有3条记录（CallerA/getone, CallerB/getone, CallerA/save）
    assert_eq!(logs.len(), 3, "应该有3条审计日志记录");

    // 验证：CallerA/getone 的计数应该是3
    let caller_a_getone = logs.iter()
        .find(|l| l.caller == "CallerA" && l.ability == "getone");
    assert!(caller_a_getone.is_some(), "应该找到 CallerA/getone 的记录");
    assert_eq!(caller_a_getone.unwrap().call_count, 3, "CallerA/getone 的计数应该是3");

    // 验证：CallerB/getone 的计数应该是1
    let caller_b_getone = logs.iter()
        .find(|l| l.caller == "CallerB" && l.ability == "getone");
    assert!(caller_b_getone.is_some(), "应该找到 CallerB/getone 的记录");
    assert_eq!(caller_b_getone.unwrap().call_count, 1, "CallerB/getone 的计数应该是1");

    // 验证：CallerA/save 的计数应该是1
    let caller_a_save = logs.iter()
        .find(|l| l.caller == "CallerA" && l.ability == "save");
    assert!(caller_a_save.is_some(), "应该找到 CallerA/save 的记录");
    assert_eq!(caller_a_save.unwrap().call_count, 1, "CallerA/save 的计数应该是1");

    // 7. 使用组件方法记录日志
    println!("\n【7】使用组件方法记录日志");
    let result = audit.do_action_with_count(&db, "delete", "CallerC", "删除测试数据", || Ok(()));
    assert!(result.is_ok(), "组件方法应该成功");
    println!("  - 组件方法调用成功");

    let logs = get_audit_logs(Some(&unique_table), 7);
    println!("  - 更新后的审计日志数量: {}", logs.len());
    assert_eq!(logs.len(), 4, "应该有4条审计日志记录");

    println!("\n=== 测试完成 ===");
}

#[test]
fn test_audit_log_date_range() {
    println!("=== 测试审计日志日期范围查询 ===\n");

    let test_tb = TestTbCount::new();
    test_tb.init_tables().expect("初始化表失败");

    // 清理之前的测试数据
    test_tb.db.execute("DELETE FROM datastate_audit_log WHERE tablename = 'testtb_count'").expect("清理数据失败");

    // 记录一些调用
    test_tb.do_action_with_count("getone", "CallerA", "查询测试", || Ok("result1".to_string())).unwrap();
    test_tb.do_action_with_count("getone", "CallerA", "再次查询", || Ok("result2".to_string())).unwrap();

    // 查询今天的记录
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let stats = get_stats_by_date_range(&today, &today);

    println!("【1】查询今天的记录");
    println!("  - 日期范围: {} 到 {}", today, today);
    println!("  - 记录数量: {}", stats.len());

    for (i, stat) in stats.iter().enumerate() {
        println!("  [{}] {}/{}/{} - 计数: {} - 摘要: {} - 日期: {}",
            i + 1, stat.tablename, stat.ability, stat.caller, stat.call_count, stat.summary, stat.stat_date);
    }

    assert!(stats.len() >= 1, "应该至少有一条记录");

    println!("\n=== 测试完成 ===");
}

#[test]
fn test_audit_perm_datastate() {
    println!("=== 测试 AuditPermDataState ===\n");

    use database::dataaudit::{AuditPermDataState};

    let perm_state = AuditPermDataState::new();

    // 初始化表
    perm_state.init_table().expect("初始化权限表失败");

    // 注册权限
    println!("【1】注册权限");
    perm_state.register_ability("testtb", "getone", "CallerA", "测试权限1").unwrap();
    perm_state.register_ability("testtb", "save", "CallerB", "测试权限2").unwrap();
    println!("  - 注册成功");

    // 检查权限
    println!("\n【2】检查权限");
    let result = perm_state.check_permission("testtb", "getone", "CallerA", true);
    assert!(result.is_ok(), "CallerA 应该有 getone 权限");
    println!("  - CallerA 对 getone: {}", result.unwrap());

    let result = perm_state.check_permission("testtb", "getone", "CallerB", true);
    assert!(result.is_err(), "CallerB 不应该有 getone 权限");
    println!("  - CallerB 对 getone: {}", result.unwrap_err());

    // 列出所有权限
    println!("\n【3】列出所有权限");
    let perms = perm_state.list_permissions();
    println!("  - 权限数量: {}", perms.len());
    for (i, perm) in perms.iter().enumerate() {
        println!("  [{}] {}/{} - caller: {} - description: {}",
            i + 1, perm.tablename, perm.ability, perm.caller, perm.description);
    }

    println!("\n=== 测试完成 ===");
}