# DataState

Rust 数据库库，提供 SQLite/MySQL 操作、数据状态管理和工作流编排能力。

[English](./README.md)

## 功能模块

### 数据库模块
- **SQLite78** - SQLite 本地数据库，内置日志
- **MySQL78** - MySQL 数据库，支持连接池和重试
- **DataState** - 数据操作状态机，支持审计
- **QueryBuilder** - SQL 查询构建器，链式 API
- **DataSync** - 数据同步组件
- **Schema** - 基础 Schema 定义

### 工作流模块
- **BaseCapability** - 工作流能力基类
- **BaseInstance** - 工作流实例基类
- **CapabilityResult** - 能力执行结果类型
- **Workflow Storage** - 工作流存储，支持分片
- **Components** - 实体、经济、生命周期管理器

## 安装

在 `Cargo.toml` 中添加：

```toml
[dependencies]
datastate = { git = "https://github.com/www778878net/rustdatastate.git" }
```

## 使用示例

### DataState（核心）

```rust
use datastate::DataState;

let db = DataState::new(None)?;
let items = db.query("SELECT * FROM users")?;
```

### MySQL

```rust
use datastate::{Mysql78, MysqlConfig};

let config = MysqlConfig {
    host: "localhost".to_string(),
    port: 3306,
    user: "root".to_string(),
    password: "your_password".to_string(),
    database: "mydb".to_string(),
    ..Default::default()
};

let mysql = Mysql78::new(config)?;
let results = mysql.query("SELECT * FROM users")?;
```

### 工作流

```rust
use datastate::{BaseCapability, CapabilityResult, BaseInstance};

struct MyCapability {
    base: CapabilityBase,
}

impl BaseCapability for MyCapability {
    fn execute(&self, context: &serde_json::Value) -> CapabilityResult {
        CapabilityResult::success(serde_json::json!({"result": "done"}))
    }
}

let instance = BaseInstance::new("my_workflow");
let result = instance.run(&serde_json::json!({"input": "data"}))?;
```

### 查询构建器

```rust
use datastate::QueryBuilder;

let (sql, values) = QueryBuilder::new()
    .select(&["id", "name", "email"])
    .from("users")
    .where_clause("status", "=", serde_json::json!("active"))
    .order_by_desc("created_at")
    .page(0, 10)
    .build();
```

## 模块说明

| 模块 | 说明 |
|------|------|
| `datastate` | 核心数据操作状态机 |
| `sqlite78` | SQLite 操作（内部使用） |
| `mysql78` | MySQL 操作，连接池支持 |
| `query_builder` | SQL 查询构建器 |
| `data_sync` | 数据同步组件 |
| `dataaudit` | 审计日志 |
| `capability` | 工作流能力基类 |
| `instance` | 工作流实例基类 |
| `workflow` | 工作流存储，支持分片 |

## 许可证

Apache License 2.0 - 详见 [LICENSE](./LICENSE)

## 仓库地址

- GitHub: https://github.com/www778878net/rustdatastate
- CNB: https://cnb.cool/778878/rustdatastate
