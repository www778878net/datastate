# LocalDB 本地数据库封装文档

## 管理员指示
- 本文档描述 LocalDB 模块的设计和实现
- 禁止把代码写在 mod.rs, lib.rs，应该在子模块中实现
- **LocalDB 应该只通过 DataState 及其组件（DataSync/DataAudit）访问，禁止直接使用**
- 直接访问 LocalDB 会绕过审计和权限控制

## 第一性目的
- SQLite 本地数据库封装
- 提供 Local-First 存储的本地封装
- 支持基本的 CRUD 操作
- 为 DataState 及其组件提供数据库访问能力
- **提供 SQL 效率统计和慢查询警告功能**

## 完成标准
- 数据库连接成功
- CRUD 操作正常
- SQL 效率统计功能正常
- 慢查询警告功能正常

## 配置选项
| 选项 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| is_log | bool | false | 是否记录警告日志（调试跟踪、错误记录） |
| is_count | bool | false | 是否统计 SQL 效率 |

## 统计表
### sys_sql 表（SQL 效率统计）
| 字段 | 说明 |
|------|------|
| cmdtext | SQL 语句 |
| cmdtextmd5 | SQL MD5（去重） |
| num | 执行次数 |
| dlong | 总耗时（毫秒） |
| downlen | 下行数据量 |

### sys_warn 表（调试跟踪、错误记录）
| 字段 | 说明 |
|------|------|
| kind | 日志类型（debug_xxx, err_xxx） |
| apimicro | 微服务名 |
| apiobj | API对象名 |
| content | 日志内容 |
| upby | 操作者 |

## 方法
- `add_warn(kind, apimicro, apiobj, content, upby)` - 记录警告日志
- `add_debug(apimicro, apiobj, content)` - 记录调试日志
- `add_error(apimicro, apiobj, content)` - 记录错误日志

## 报表方法
报表方法在 `sys_sql_state.rs` 中实现：
- `get_slow_sql(min_dlong, limit)` - 获取慢 SQL 列表
- `get_hot_sql(min_num, limit)` - 获取高频 SQL 列表

## 完成标准
- 数据库连接成功创建
- WAL 模式正确设置
- CRUD 操作正常工作
- 表存在检查正确
- 分表功能正常

## 前置依赖
- rusqlite 库
- serde_json 库
- chrono 库

## 业务逻辑

### 核心功能
- `new()`: 创建新的数据库连接
- `default_instance()`: 获取默认数据库实例
- `insert()`: 插入数据
- `update()`: 更新数据
- `query()`: 查询数据
- `delete()`: 删除数据
- `execute()`: 执行 SQL
- `count()`: 获取表记录数
- `table_exists()`: 检查表是否存在
- `ensure_table()`: 确保表存在
- `get_daily_table_name()`: 获取按天分表的表名
- `cleanup_old_tables()`: 清理过期表

### 数据库配置
- WAL 模式：提高并发性能
- busy_timeout: 30 秒
- 自动创建父目录

### JSON 处理
- 查询结果自动转换为 HashMap<String, Value>
- 支持字符串、整数、浮点数类型自动识别
- JSON 字符串自动解析

## 测试方案

### 主要逻辑测试

#### 测试1：创建数据库连接
```
输入：数据库路径（或 None 使用默认路径）
步骤：LocalDB::with_path(&db_path) 或 LocalDB::default_instance()
预期：返回有效的数据库实例
```

#### 测试2：插入数据
```
输入：表名、数据 HashMap
步骤：db.insert("test_table", &data)
预期：返回新记录的 ID
```

#### 测试3：查询数据
```
输入：SQL 语句和参数
步骤：db.query("SELECT * FROM test_table WHERE id = ?", &[&id])
预期：返回匹配的记录列表
```

#### 测试4：更新数据
```
输入：表名、ID、更新数据
步骤：db.update("test_table", "test_001", &data)
预期：返回 true（更新成功）
```

#### 测试5：删除数据
```
输入：表名、ID
步骤：db.delete("test_table", "test_001")
预期：返回 true（删除成功）
```

### 其它测试（边界、异常等）

#### 测试6：表存在检查
```
输入：不存在的表名
步骤：db.table_exists("non_existent_table")
预期：返回 Ok(false)
```

#### 测试7：按天分表名称
```
输入：表名、可选日期
步骤：LocalDB::get_daily_table_name("my_table", None)
预期：返回 "my_table_YYYYMMDD" 格式
```

#### 测试8：空表查询
```
输入：空表的查询 SQL
步骤：db.query("SELECT * FROM empty_table", &[])
预期：返回空数组 []
```

#### 测试9：更新不存在的记录
```
输入：不存在的 ID
步骤：db.update("test_table", "non_existent_id", &data)
预期：返回 false
```

#### 测试10：删除不存在的记录
```
输入：不存在的 ID
步骤：db.delete("test_table", "non_existent_id")
预期：返回 false
```

#### 测试11：配置读取
```
输入：无
步骤：LocalDBConfig::default()
预期：返回默认配置实例
```

## 知识库

### 创建数据库
```rust
let db = LocalDB::new("config/local.db")?;
let db = LocalDB::default_instance()?;
```

### CRUD 操作
```rust
// 插入
let mut data = HashMap::new();
data.insert("id".to_string(), json!("test_001"));
data.insert("name".to_string(), json!("测试"));
db.insert("my_table", &data)?;

// 查询
let results = db.query("SELECT * FROM my_table", &[])?;

// 更新
db.update("my_table", "test_001", &data)?;

// 删除
db.delete("my_table", "test_001")?;
```

## 好坏示例

### 好示例
- 使用 default_instance() 获取默认实例
- 使用 ensure_table() 确保表存在
- 使用 WAL 模式提高并发

### 坏示例
- 直接执行 SQL 拼接字符串（SQL 注入风险）
- 不检查表是否存在就操作
- 不处理错误返回