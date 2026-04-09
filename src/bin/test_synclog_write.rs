//! 测试 synclog 分表写入功能

use datastate::data_sync::Synclog;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 测试 synclog 分表写入功能 ===\n");

    // 1. 使用 Synclog 直接插入
    println!("使用 Synclog 直接插入...");
    let synclog = Synclog::with_default_path()?;
    let idpk1 = synclog.add_to_synclog(
        "test_table1",
        "test_001",
        "insert",
        "INSERT INTO test_table1 (id, name) VALUES (?, ?)",
        "[\"test_001\", \"测试数据1\"]",
        "test_worker",
        "cid-001",
    )?;
    println!("✓ Synclog 插入1成功，idpk: {}", idpk1);

    let idpk2 = synclog.add_to_synclog(
        "test_table2",
        "test_002",
        "insert",
        "INSERT INTO test_table2 (id, name) VALUES (?, ?)",
        "[\"test_002\", \"测试数据2\"]",
        "test_worker",
        "cid-002",
    )?;
    println!("✓ Synclog 插入2成功，idpk: {}", idpk2);

    // 2. 查询所有分表
    println!("\n查询现有的 synclog 分表...");
    let tables = synclog.get_all_shard_tables()?;
    println!("✓ 找到 {} 个 synclog 分表:", tables.len());
    for table in tables {
        println!("  - {}", table);
    }

    // 3. 查询待同步记录数
    let count1 = synclog.get_pending_count_by_tbname("test_table1")?;
    let count2 = synclog.get_pending_count_by_tbname("test_table2")?;
    println!("\ntest_table1 待同步记录数: {}", count1);
    println!("test_table2 待同步记录数: {}", count2);

    // 4. 查询待同步记录
    let pending1 = synclog.get_pending_items_by_tbname("test_table1", 10)?;
    let pending2 = synclog.get_pending_items_by_tbname("test_table2", 10)?;
    println!("\ntest_table1 待同步记录: {}", pending1.len());
    for item in pending1 {
        println!("  - idpk: {:?}, tbname: {:?}, idrow: {:?}",
            item.get("idpk"), item.get("tbname"), item.get("idrow"));
    }

    println!("\ntest_table2 待同步记录: {}", pending2.len());
    for item in pending2 {
        println!("  - idpk: {:?}, tbname: {:?}, idrow: {:?}",
            item.get("idpk"), item.get("tbname"), item.get("idrow"));
    }

    println!("\n=== 完成 ===");
    Ok(())
}
