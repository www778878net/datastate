# Mysql78 - MySQL 数据库操作类

## 第一性目的

提供 MySQL 数据库的连接池管理、预处理语句缓存、重试机制和事务操作，基于 koa78-base78 Mysql78.ts 的 Rust 实现。

## 主要类型

### MysqlConfig

MySQL 连接配置：
- `host` - 主机地址，默认 "127.0.0.1"
- `port` - 端口号，默认 3306
- `user` - 用户名，默认 "root"
- `password` - 密码
- `database` - 数据库名
- `max_connections` - 最大连接数，默认 10
- `is_log` - 是否记录日志
- `is_count` - 是否统计查询次数

### MysqlUpInfo

用户上传信息结构：
- `apisys` - 系统标识
- `apimicro` - 微服务标识
- `apiobj` - 对象标识
- `uname` - 用户名
- `upid` - 更新ID
- `uptime` - 更新时间
- `debug` - 调试模式

### Mysql78

核心数据库操作类，提供：
- 连接池管理
- 预处理语句缓存
- 自动重试机制
- 事务操作支持
- CRUD 操作封装

## 核心方法

### 连接管理
- `new(config: MysqlConfig) -> Result<Self, String>` - 创建连接池
- `get_conn() -> Result<PooledConn, String>` - 获取连接
- `close()` - 关闭连接池

### 查询操作
- `query<T>(sql: &str) -> Result<Vec<T>, String>` - 执行查询
- `query_one<T>(sql: &str) -> Result<Option<T>, String>` - 查询单条
- `execute(sql: &str) -> Result<u64, String>` - 执行语句
- `insert(table: &str, data: &HashMap<&str, &str>) -> Result<String, String>` - 插入数据
- `update(table: &str, data: &HashMap<&str, &str>, where_clause: &str) -> Result<u64, String>` - 更新数据

### 事务操作
- `begin_transaction() -> Result<MysqlTransaction, String>` - 开始事务

## 示例

```rust
use datastate::{Mysql78, MysqlConfig};

// 创建配置
let config = MysqlConfig {
    host: "localhost".to_string(),
    port: 3306,
    user: "root".to_string(),
    password: "password".to_string(),
    database: "mydb".to_string(),
    ..Default::default()
};

// 连接数据库
let mysql = Mysql78::new(config)?;

// 查询数据
let rows: Vec<(i32, String)> = mysql.query("SELECT id, name FROM users")?;

// 插入数据
let mut data = HashMap::new();
data.insert("name", "test");
data.insert("email", "test@example.com");
let id = mysql.insert("users", &data)?;
```
