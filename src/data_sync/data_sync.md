# DataSync 同步队列组件

## 管理员指示
- 本文档描述 DataSync 组件的设计和实现
- DataSync 是组合组件，被 DataState 组合使用

## 第一性目的
- 同步队列管理（添加、获取、标记已同步）
- 状态变更日志记录
- 同步统计（按天）
- 下载/上传逻辑处理

## 完成标准
- 同步队列操作正常
- 状态变更日志记录正确
- 同步统计数据准确
- 下载/上传功能正常

## 前置依赖
- LocalDB 数据库类
- TableConfig 配置类

## 业务逻辑

### 核心职责
DataSync 是同步队列组件，包含三个核心功能：
1. **sync_queue** - 待同步数据队列（本地变更待上传）
2. **data_state_log** - 状态变更日志
3. **data_sync_stats** - 同步统计（按天）

### 主要数据结构

| 结构体 | 说明 |
|--------|------|
| SynclogItem | 同步日志项（与服务器端 synclog 表一致） |
| StateLog | 状态变更日志 |
| SyncStats | 同步统计 |
| SyncResult | 同步结果 |
| SyncData | 同步数据详情 |
| SyncValidationError | 同步验证错误信息（服务器返回） |
| ProtoSynclogItem | synclog 项（protobuf 编码用） |
| ProtoSynclogBatch | synclog 批量数据（protobuf 编码用） |

### 主要方法

| 方法 | 说明 |
|------|------|
| new(table_name) | 创建新实例 |
| with_db(table_name, db) | 使用指定数据库实例创建 |
| from_config(config) | 从 TableConfig 创建 |
| init_tables(db) | 初始化同步队列相关表 |
| need_download() | 检查是否需要下载 |
| need_upload() | 检查是否需要上传 |
| add_to_queue() | 添加记录到同步队列 |
| current_time() | 获取当前时间戳 |
| extract_table_name(api_url) | 从 URL 提取表名 |

## 测试方案

### 主要逻辑测试

#### 测试1：创建 DataSync 实例
```
输入：table_name = "test_table"
步骤：let datasync = DataSync::new("test_table");
预期：datasync.table_name = "test_table"，db 实例存在
```

#### 测试2：初始化同步队列表
```
输入：已创建的 LocalDB 实例
步骤：DataSync::init_tables(&db)
预期：synclog、data_state_log、data_sync_stats 表创建成功
```

#### 测试3：添加记录到同步队列
```
输入：record = {"id": "test_001", "data": "test"}
步骤：datasync.add_to_queue(&record, "insert", "test_table")
预期：sync_queue 表中新增一条记录
```

#### 测试4：检查下载/上传条件
```
输入：设置 last_download_time、last_upload_time
步骤：need_download()、need_upload()
预期：根据时间间隔返回正确的布尔值
```

### 其它测试（边界、异常等）

#### 测试5：空表名处理
```
输入：table_name = ""
步骤：DataSync::new("")
预期：实例创建成功，但 CRUD 操作需要表名
```

#### 测试6：从配置创建实例
```
输入：TableConfig { name: "config_table", ... }
步骤：DataSync::from_config(&config)
预期：返回正确的实例，table_name = "config_table"
```

#### 测试7：使用指定数据库实例
```
输入：LocalDB 实例
步骤：DataSync::with_db("my_table", db)
预期：返回使用指定数据库的实例
```

#### 测试8：状态变更日志
```
输入：old_status = "idle", new_status = "working"
步骤：datasync.log_status_change("idle", "working", "reason", "worker")
预期：data_state_log 表中新增一条记录
```

## 知识库

### 创建实例
```rust
// 创建新实例
let datasync = DataSync::new("my_table");

// 使用指定数据库
let db = LocalDB::new(None)?;
let datasync = DataSync::with_db("my_table", db);

// 从配置创建
let config = TableConfig::new("my_table");
let datasync = DataSync::from_config(&config);
```

### 初始化表
```rust
DataSync::init_tables(&db)?;
```

## 好坏示例

### 好示例
- 使用 from_config 从 TableConfig 创建
- 初始化时调用 init_tables
- 通过 DataSync 管理同步队列

### 坏示例
- 直接操作 synclog 表
- 不通过 DataSync 记录状态变更
- 忽略下载/上传条件检查
