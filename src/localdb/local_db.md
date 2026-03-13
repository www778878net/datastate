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

### 基础功能测试
- [ ] 创建数据库连接成功
- [ ] 插入数据成功
- [ ] 查询数据成功
- [ ] 更新数据成功
- [ ] 删除数据成功

### 表管理测试
- [ ] 表存在检查正确
- [ ] 确保表存在功能正常
- [ ] 按天分表名称正确
- [ ] 清理过期表功能正常

### 边界测试
- [ ] 空表查询返回空数组
- [ ] 不存在的记录更新返回 false
- [ ] 不存在的记录删除返回 false

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