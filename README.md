# DataState

A Rust library providing database operations (SQLite/MySQL), data state management, and workflow orchestration capabilities.

[中文文档](./README_CN.md)

## Features

### Database Modules
- **SQLite78** - SQLite local database with built-in logging
- **MySQL78** - MySQL database with connection pooling and retry
- **DataState** - State machine for data operations with audit support
- **QueryBuilder** - SQL query builder with chainable API
- **DataSync** - Data synchronization components
- **Schema** - Base schema definitions

### Workflow Modules
- **BaseCapability** - Base class for workflow capabilities
- **BaseInstance** - Base class for workflow instances
- **CapabilityResult** - Result type for capability execution
- **Workflow Storage** - Workflow storage with sharding support
- **Components** - Entity, Economic, Lifecycle managers

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
datastate = { git = "https://github.com/www778878net/rustdatastate.git" }
```

## Usage

### DataState (Core)

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

### Workflow

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

### Query Builder

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

## Modules

| Module | Description |
|--------|-------------|
| `datastate` | Core state machine for data operations |
| `sqlite78` | SQLite operations (internal use) |
| `mysql78` | MySQL operations with connection pool |
| `query_builder` | SQL query builder |
| `data_sync` | Data synchronization components |
| `dataaudit` | Audit logging |
| `capability` | Workflow capability base class |
| `instance` | Workflow instance base class |
| `workflow` | Workflow storage with sharding |

## License

Apache License 2.0 - see [LICENSE](./LICENSE) for details.

## Repository

- GitHub: https://github.com/www778878net/rustdatastate
- CNB: https://cnb.cool/778878/rustdatastate
