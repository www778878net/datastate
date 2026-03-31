# Sqlite78 - SQLite 数据库操作类

## 第一性目的

提供 Local-First 存储的本地数据库操作能力，基于 koa78-base78 Sqlite78.ts 的 Rust 实现。

## 核心功能

- 数据库连接管理
- CRUD 操作（查询、插入、更新）
- 事务支持
- 系统表自动创建

## 使用方式

```rust
use crate::sqlite78::{Sqlite78, UpInfo};

// 使用默认路径（docs/config/local.db）
let mut db = Sqlite78::with_default_path();
db.initialize().expect("初始化失败");

// 或指定路径
let mut db = Sqlite78::with_config("path/to/db.db", false, false);
db.initialize().expect("初始化失败");

// 创建系统表
let up = UpInfo::new();
db.creat_tb(&up).expect("创建表失败");

// 查询
let rows = db.do_get("SELECT * FROM sys_warn", &[], &up).expect("查询失败");

// 插入
let result = db.do_m_add("INSERT INTO sys_warn (id, content, uptime) VALUES (?, ?, ?)", &[&id, &content, &uptime], &up);

// 更新
let result = db.do_m("UPDATE sys_warn SET content = ? WHERE id = ?", &[&new_content, &id], &up);

// 事务
let cmds = vec!["INSERT ...".to_string(), "UPDATE ...".to_string()];
let values = vec![vec![&param1 as &dyn rusqlite::ToSql], vec![&param2 as &dyn rusqlite::ToSql]];
let errtexts = vec!["插入失败".to_string(), "更新失败".to_string()];
db.do_t(&cmds, values, &errtexts, "", &[], &up);
```

## API 列表

### 构造函数

| 方法 | 说明 |
|------|------|
| `new()` | 创建空实例 |
| `with_default_path()` | 使用默认路径 docs/config/local.db |
| `with_config(filename, is_log, is_count)` | 指定配置创建 |

### 核心方法

| 方法 | 输入 | 输出 | 说明 |
|------|------|------|------|
| `initialize()` | 无 | `Result<(), String>` | 初始化数据库连接 |
| `creat_tb(&up)` | UpInfo | `Result<String, String>` | 创建系统表 |
| `do_get(cmdtext, values, &up)` | SQL, 参数 | `Result<Vec<HashMap<String, Value>>, String>` | 查询数据 |
| `do_m_add(cmdtext, values, &up)` | SQL, 参数 | `Result<InsertResult, String>` | 插入数据 |
| `do_m(cmdtext, values, &up)` | SQL, 参数 | `Result<UpdateResult, String>` | 更新数据 |
| `do_t(cmds, values, errtexts, logtext, logvalue, &up)` | 事务参数 | `Result<String, String>` | 执行事务 |
| `close()` | 无 | 无 | 关闭连接 |

## 数据结构

### UpInfo

```rust
pub struct UpInfo {
    pub apisys: String,    // 系统标识
    pub apimicro: String,  // 微服务标识
    pub apiobj: String,    // 对象标识
    pub uname: String,     // 用户名
    pub upid: String,      // 操作ID
    pub uptime: String,    // 操作时间
    pub debug: bool,       // 调试模式
}
```

### InsertResult

```rust
pub struct InsertResult {
    pub insert_id: i64,        // 插入的自增ID
    pub error: Option<String>, // 错误信息
}
```

### UpdateResult

```rust
pub struct UpdateResult {
    pub affected_rows: i64,    // 影响行数
    pub error: Option<String>, // 错误信息
}
```

## 系统表结构

### sys_warn

预警信息表，用于记录系统预警。

| 字段 | 类型 | 说明 |
|------|------|------|
| idpk | INTEGER | 自增主键 |
| id | TEXT | 业务ID |
| uid | TEXT | 用户ID |
| kind | TEXT | 类型 |
| apimicro | TEXT | 微服务 |
| apiobj | TEXT | 对象 |
| content | TEXT | 内容 |
| uptime | DATETIME | 时间 |

### sys_sql

SQL 记录表，用于记录 SQL 执行日志。

| 字段 | 类型 | 说明 |
|------|------|------|
| idpk | INTEGER | 自增主键 |
| id | TEXT | 业务ID |
| apisys | TEXT | 系统 |
| apimicro | TEXT | 微服务 |
| apiobj | TEXT | 对象 |
| cmdtext | TEXT | SQL 语句 |
| cmdtextmd5 | TEXT | SQL MD5 |
| num | INTEGER | 执行次数 |
| dlong | INTEGER | 执行时长 |
| uptime | DATETIME | 时间 |

## 默认路径查找规则

`find_default_db_path()` 方法会：

1. 从当前目录开始
2. 向上查找 `docs` 或 `.claude` 目录
3. 返回 `docs/config/local.db` 路径
4. 如果未找到，使用当前目录下的 `docs/config/local.db`

## 注意事项

- 所有表字段必须有默认值
- 启用 WAL 模式和 30 秒超时
- 线程安全（使用 Arc<Mutex<Connection>>）

## 测试方案

### 主要逻辑测试

#### 测试1：创建实例
```
输入：Sqlite78::new(), with_default_path(), with_config()
步骤：
  let db = Sqlite78::new();
  assert!(db.get_filename().is_empty());

  let db = Sqlite78::with_default_path();
  assert!(!db.get_filename().is_empty());

  let db = Sqlite78::with_config("test.db", true, true);
  assert_eq!(db.get_filename(), "test.db");
预期：实例创建成功，属性正确
```

#### 测试2：初始化数据库
```
输入：指定路径 "tmp/tmp/test_init.db"
步骤：
  let mut db = Sqlite78::with_config("tmp/tmp/test_init.db", false, false);
  let result = db.initialize();
预期：result.is_ok()
```

#### 测试3：创建系统表
```
输入：已初始化的数据库实例
步骤：
  let mut db = Sqlite78::with_config("tmp/tmp/test_creat_tb.db", false, false);
  db.initialize().expect("初始化失败");
  let up = UpInfo::new();
  let result = db.creat_tb(&up);
预期：result.is_ok()
```

#### 测试4：查询空表
```
输入：已创建系统表的数据库
步骤：
  let mut db = Sqlite78::with_config("tmp/tmp/test_do_get.db", false, false);
  db.initialize().expect("初始化失败");
  db.creat_tb(&UpInfo::new()).expect("创建表失败");
  let up = UpInfo::new();
  let result = db.do_get("SELECT * FROM sys_warn", &[], &up);
预期：result.is_ok()，返回空数组
```

### 其它测试（边界、异常等）

#### 测试5：未初始化时获取连接
```
输入：未初始化的实例
步骤：
  let db = Sqlite78::new();
  let result = db.get_conn();
预期：result.is_err()
```

#### 测试6：关闭连接后获取连接
```
输入：已初始化并关闭的实例
步骤：
  let mut db = Sqlite78::with_config("tmp/tmp/test_close.db", false, false);
  db.initialize().expect("初始化失败");
  db.close();
  let result = db.get_conn();
预期：result.is_err()
```

#### 测试7：默认路径查找
```
输入：当前项目环境
步骤：
  let result = Sqlite78::find_default_db_path();
预期：result.is_ok() 或 result.is_err()（取决于项目环境）
```