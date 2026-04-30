//! 测试创建 synclog 分表

use datastate::data_sync::Synclog;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 创建 synclog 分表 ===\n");

    // 1. 创建 Synclog 实例
    let mut synclog = Synclog::with_default_path()?;
    println!("✓ Synclog 实例创建成功");

    // 2. 执行维护
    println!("\n执行分表维护...");
    let result = synclog.perform_maintenance().await?;
    println!("✓ 维护结果: 创建 {} 个表，删除 {} 个表", result.tables_created, result.tables_dropped);

    // 3. 获取所有分表
    println!("\n查询现有的 synclog 分表...");
    let tables = synclog.get_all_shard_tables().await?;
    println!("✓ 找到 {} 个 synclog 分表:", tables.len());
    for table in tables {
        println!("  - {}", table);
    }

    println!("\n=== 完成 ===");
    Ok(())
}
