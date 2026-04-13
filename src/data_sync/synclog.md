# Synclog 同步日志分表管理

## 管理员指示
- 本文档描述 Synclog 组件的设计和实现
- Synclog 管理 synclog 表的分表逻辑，支持按天分表

## 第一性目的
- synclog 表按天分表管理（保留7天）
- 支持延迟切换策略（0:00-0:30读昨天，其他时间读今天）
- 提供读写分离的表名获取方法
- 支持进度文件优化查询

## 完成标准
- 分表创建和管理正常
- 延迟切换策略正确执行
- 待同步记录查询准确
- 进度文件正确更新

## 前置依赖
- Sqlite78 数据库类
- ShardingManager 分表管理器
- ShardingConfig 分表配置

## 业务逻辑

### 分表策略
- 按天分表，表名格式：`synclog_YYYYMMDD`
- 保留7天数据，自动清理过期表
- 写操作：00:00 立即切换到今天的表
- 读操作：延迟切换策略（0:00-0:30读昨天的表）

### 延迟切换策略
```
时间窗口       | 读表策略
-------------|------------------
00:00-00:30  | 读昨天的分表
00:30-23:59  | 读今天的分表
```

### 进度文件优化
- 文件路径：`tmp/synclog/[表名].txt`
- 存储最后处理的日期
- 避免每次查询所有分表

### 主要方法

| 方法 | 说明 |
|------|------|
| with_sharding(db) | 创建分表实例 |
| with_default_path() | 使用默认路径创建 |
| get_table_name() | 获取当前表名（写操作用） |
| get_upload_table_name() | 获取上传表名（读操作用，延迟切换） |
| get_business_table_name_for_write() | 获取业务表写表名（始终今天） |
| get_business_table_name_for_read() | 获取业务表读表名（延迟切换） |
| get_pending_count_by_tbname() | 获取指定表的待同步记录数 |
| get_pending_items_by_tbname() | 获取指定表的待同步记录 |
| add_to_synclog() | 添加到同步队列 |
| mark_synced_by_ids() | 按id列表标记已同步 |
| mark_synced_by_idrows() | 按idrow列表标记已同步 |
| mark_failed_by_id() | 标记失败记录 |
| save_progress_date() | 保存进度日期 |

### synclog 表结构

| 字段 | 类型 | 说明 |
|------|------|------|
| idpk | INTEGER | 主键自增 |
| id | TEXT | 雪花ID |
| tbname | TEXT | 业务表名 |
| action | TEXT | 操作类型（insert/update/delete） |
| cmdtext | TEXT | SQL语句 |
| params | TEXT | 参数JSON |
| idrow | TEXT | 业务记录ID |
| worker | TEXT | 操作者 |
| synced | INTEGER | 同步状态（0待同步，1已同步，-1失败） |
| lasterrinfo | TEXT | 最后错误信息 |
| cmdtextmd5 | TEXT | SQL的MD5 |
| cid | TEXT | 客户端ID |
| upby | TEXT | 更新人 |
| uptime | TEXT | 更新时间 |

## 测试方案

### 主要逻辑测试

#### 测试1：创建分表实例
```
输入：无
步骤：let synclog = Synclog::with_default_path()
预期：返回 Synclog 实例，表名格式为 synclog_YYYYMMDD
```

#### 测试2：获取写表名
```
输入：当前日期 2026-04-12
步骤：synclog.get_table_name()
预期：返回 "synclog_20260412"
```

#### 测试3：延迟切换策略（00:00-00:30）
```
输入：当前时间 00:15
步骤：synclog.get_upload_table_name()
预期：返回昨天的分表名 "synclog_20260411"
```

#### 测试4：延迟切换策略（00:30之后）
```
输入：当前时间 01:00
步骤：synclog.get_upload_table_name()
预期：返回今天的分表名 "synclog_20260412"
```

#### 测试5：添加同步记录
```
输入：tbname="test_table", record_id="123", action="insert"
步骤：synclog.add_to_synclog("test_table", "123", "insert", "INSERT INTO...", "[]", "worker", "cid")
预期：synclog 表中新增一条记录，synced=0
```

#### 测试6：获取待同步记录
```
输入：tbname="test_table", limit=10
步骤：synclog.get_pending_items_by_tbname("test_table", 10)
预期：返回该表的待同步记录列表
```

### 其它测试（边界、异常等）

#### 测试7：进度文件保存和读取
```
输入：base_table="test_table", date="20260412"
步骤：Synclog::save_progress_date("test_table", "20260412")
     Synclog::read_progress_date("test_table")
预期：文件写入成功，读取返回 "20260412"
```

#### 测试8：标记已同步
```
输入：id_list=["id1", "id2"]
步骤：synclog.mark_synced_by_ids(&["id1".to_string(), "id2".to_string()])
预期：对应记录的 synced 字段更新为 1
```

#### 测试9：标记失败
```
输入：id="id1", errinfo="连接超时"
步骤：synclog.mark_failed_by_id("id1", "连接超时")
预期：对应记录的 synced=-1，lasterrinfo="连接超时"
```

#### 测试10：错误信息截断
```
输入：超长错误信息（>500字符）
步骤：Synclog::truncate_errinfo(long_error)
预期：截断为500字符，末尾添加 "...[TRUNCATED]"
```

## 知识库

### 创建实例
```rust
// 使用默认路径创建分表实例
let synclog = Synclog::with_default_path()?;

// 使用指定路径创建
let synclog = Synclog::with_path("/path/to/db")?;

// 不分表模式
let db = Sqlite78::with_default_path();
let synclog = Synclog::new(db);
```

### 获取表名
```rust
// 写操作：始终返回今天的表名
let write_table = synclog.get_table_name();

// 读操作：延迟切换策略
let read_table = synclog.get_upload_table_name();

// 业务表写表名
let business_write = synclog.get_business_table_name_for_write("workflow_instance");

// 业务表读表名
let business_read = synclog.get_business_table_name_for_read("workflow_instance");
```

### 同步操作
```rust
// 添加到同步队列
let idpk = synclog.add_to_synclog(
    "my_table",      // 业务表名
    "record_id",     // 记录ID
    "insert",        // 操作类型
    "INSERT INTO...", // SQL
    "[]",            // 参数JSON
    "worker",        // 操作者
    "cid"            // 客户端ID
)?;

// 获取待同步记录
let items = synclog.get_pending_items_by_tbname("my_table", 100)?;

// 标记已同步
synclog.mark_synced_by_ids(&["id1".to_string(), "id2".to_string()])?;

// 标记失败
synclog.mark_failed_by_id("id1", "错误信息")?;
```

### 进度文件
```rust
// 保存进度
Synclog::save_progress_date("my_table", "20260412")?;

// 进度文件路径：tmp/synclog/my_table.txt
```

## 好坏示例

### 好示例
- 使用 `get_business_table_name_for_write()` 获取写表名
- 使用 `get_business_table_name_for_read()` 获取读表名
- 通过进度文件优化查询
- 错误信息自动截断防止膨胀

### 坏示例
- 写操作使用读表名（导致数据写入错误的表）
- 忽略延迟切换策略（导致0点附近数据丢失）
- 不使用进度文件（每次查询所有分表）
- 直接操作 synclog 表（绕过分表逻辑）

## 原始文档

- 代码: crates/datastate/src/data_sync/synclog.rs
- 测试: crates/datastate/src/data_sync/synclog.rs#tests
- 相关: crates/datastate/src/data_sync/data_sync.md
