//! 实际业务集成测试
use base::mylogger::mylogger;
//!
//! 测试 Sqlite78 的实际建表和插入功能

use datastate::{Sqlite78, SysWarnSqliteState, SysSqlSqliteState, SysWarnData, UpInfo};
use std::path::PathBuf;

fn get_test_db_path() -> String {
    let path = PathBuf::from("tmp/tmp/test_sqlite78.db");
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    path.to_string_lossy().to_string()
}

#[test]
fn test_sqlite78_initialize() {
    let db_path = get_test_db_path();

    let mut db = Sqlite78::with_config(&db_path, true, true);
    let result = db.initialize();

    mylogger!().detail(&format!("初始化结果: {:?}", result));
    assert!(result.is_ok(), "数据库初始化应该成功: {:?}", result);
}

#[test]
fn test_create_sys_warn_table() {
    let db_path = get_test_db_path();

    let mut db = Sqlite78::with_config(&db_path, true, true);
    db.initialize().expect("初始化失败");

    let state = SysWarnSqliteState::new(db);
    let up = UpInfo::default();

    let result = state.create_table(&up);
    mylogger!().detail(&format!("创建 sys_warn 表结果: {:?}", result));
    assert!(result.is_ok(), "创建 sys_warn 表应该成功: {:?}", result);
}

#[test]
fn test_insert_sys_warn() {
    let db_path = get_test_db_path();

    let mut db = Sqlite78::with_config(&db_path, true, true);
    db.initialize().expect("初始化失败");

    let state = SysWarnSqliteState::new(db);
    let up = UpInfo::default();

    // 先创建表
    state.create_table(&up).expect("创建表失败");

    // 插入数据
    let mut data = SysWarnData::new();
    data.id = SysWarnData::new_id();
    data.kind = "test".to_string();
    data.apimicro = "test_micro".to_string();
    data.apiobj = "test_obj".to_string();
    data.content = "这是一条测试警告".to_string();
    data.uptime = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    let result = state.insert(&data, &up);
    mylogger!().detail(&format!("插入 sys_warn 结果: {:?}", result));
    assert!(result.is_ok(), "插入 sys_warn 应该成功");
    assert!(result.unwrap() > 0, "应该返回有效的 insert_id");
}

#[test]
fn test_create_sys_sql_table() {
    let db_path = get_test_db_path();

    let mut db = Sqlite78::with_config(&db_path, true, true);
    db.initialize().expect("初始化失败");

    let state = SysSqlSqliteState::new(db);
    let up = UpInfo::default();

    let result = state.create_table(&up);
    mylogger!().detail(&format!("创建 sys_sql 表结果: {:?}", result));
    assert!(result.is_ok(), "创建 sys_sql 表应该成功: {:?}", result);
}

#[test]
fn test_insert_and_query_sys_warn() {
    let db_path = get_test_db_path();

    let mut db = Sqlite78::with_config(&db_path, true, true);
    db.initialize().expect("初始化失败");

    let state = SysWarnSqliteState::new(db);
    let up = UpInfo::default();

    // 创建表
    state.create_table(&up).expect("创建表失败");

    // 插入3条数据
    for i in 1..=3 {
        let mut data = SysWarnData::new();
        data.id = SysWarnData::new_id();
        data.kind = format!("test_{}", i);
        data.apimicro = "test_micro".to_string();
        data.apiobj = "test_obj".to_string();
        data.content = format!("测试警告 {}", i);
        data.uptime = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

        let result = state.insert(&data, &up);
        mylogger!().detail(&format!("插入第 {} 条: {:?}", i, result));
        assert!(result.is_ok(), "插入应该成功");
    }

    // 查询
    let query_result = state.get_by_kind("test_1", &up);
    mylogger!().detail(&format!("查询结果: {:?}", query_result));
    assert!(query_result.is_ok(), "查询应该成功");

    let records = query_result.unwrap();
    assert!(!records.is_empty(), "应该查到记录");
    assert_eq!(records[0].kind, "test_1");
}

#[test]
fn test_full_workflow() {
    mylogger!().detail(&format!("========== 开始完整业务测试 =========="));

    let db_path = get_test_db_path();

    // 1. 初始化数据库
    mylogger!().detail(&format!("\n[Step 1] 初始化数据库..."));
    let mut db = Sqlite78::with_config(&db_path, true, true);
    db.initialize().expect("初始化失败");
    mylogger!().detail(&format!("数据库路径: {}", db_path));

    // 2. 创建 sys_warn 表
    mylogger!().detail(&format!("\n[Step 2] 创建 sys_warn 表..."));
    let warn_state = SysWarnSqliteState::new(db);
    let up = UpInfo::default();
    warn_state.create_table(&up).expect("创建 sys_warn 表失败");
    mylogger!().detail(&format!("sys_warn 表创建成功"));

    // 3. 插入警告数据
    mylogger!().detail(&format!("\n[Step 3] 插入警告数据..."));
    let mut warn_data = SysWarnData::new();
    warn_data.id = SysWarnData::new_id();
    warn_data.kind = "debug_test".to_string();
    warn_data.apimicro = "sqlite78".to_string();
    warn_data.apiobj = "test".to_string();
    warn_data.content = "集成测试警告记录".to_string();
    warn_data.uptime = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    let insert_id = warn_state.insert(&warn_data, &up).expect("插入失败");
    mylogger!().detail(&format!("插入成功, insert_id: {}", insert_id));

    // 4. 查询验证
    mylogger!().detail(&format!("\n[Step 4] 查询验证..."));
    let records = warn_state.get_by_kind("debug_test", &up).expect("查询失败");
    mylogger!().detail(&format!("查询到 {} 条记录", records.len()));
    for (i, r) in records.iter().enumerate() {
        mylogger!().detail(&format!("  [{}] id={}, kind={}, content={}", i, r.id, r.kind, r.content));
    }
    assert!(!records.is_empty(), "应该查到刚插入的记录");

    mylogger!().detail(&format!("\n========== 测试全部通过 =========="));
}