//! testtb 下载和上传测试
//!
//! 对比 Python 用法：
//! # Python（最简示例）
//! class TesttbState(DataState):
//!     @classmethod
//!     def get_table_config(cls) -> TableConfig:
//!         return TableConfig(
//!             name='testtb',
//!             apiurl='http://api.example.com/test/testtb',
//!             columns={**cls.get_system_columns()}
//!         )
//! state = await data_manage.register(TesttbState)
//! result = await data_manage.sync_once()  # DM 自动控制下载和上传
use database::{DataManage, LocalDB, TableConfig, get_system_columns};

/// 测试注册 - 只要注册，下载和上传由 DM 自动控制
#[test]
fn test_tb_register() {
    println!("\n=== testtb 注册测试 ===");

    // 使用默认 DataManage（内部已有 LocalDB）
    let dm = DataManage::default();

    // 配置 columns 是必须的
    let mut columns = get_system_columns();
    columns.insert("kind".to_string(), "TEXT NOT NULL DEFAULT ''".to_string());
    columns.insert("item".to_string(), "TEXT NOT NULL DEFAULT ''".to_string());
    columns.insert("data".to_string(), "TEXT NOT NULL DEFAULT ''".to_string());
    let config = TableConfig {
        name: "testtb".to_string(),
        apiurl: "http://api.example.com/test/testtb".to_string(),
        columns,
        ..Default::default()
    };

    // 注册到 DataManage（DM 自动控制下载和上传）
    let _state = dm.register(config).expect("注册失败");
    println!("注册成功");
}