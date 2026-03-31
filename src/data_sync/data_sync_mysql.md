# data_sync_mysql

## 概述

DataSyncMysql - MySQL 版本同步组件，提供同步队列管理、状态变更日志、同步统计功能。

## 第一性目的

基于 MySQL 的数据同步组件：
- 同步队列管理
- 状态变更日志
- 同步统计
- CRUD 操作封装

## 核心类型

### DataSyncMysql

MySQL 版本同步队列组件：
- `table_name` - 表名
- `db: Mysql78` - MySQL 数据库实例
- `apiurl` - API URL
- `download_interval` - 下载间隔(秒)
- `upload_interval` - 上传间隔(秒)
- `download_enabled` - 是否启用下载
- `upload_enabled` - 是否启用上传

### 辅助类型
- `SynclogItemMysql` - 同步日志项
- `StateLogMysql` - 状态变更日志
- `SyncStatsMysql` - 同步统计
- `SyncResultMysql` - 同步结果

## 核心方法

### 创建方法
- `new(table_name, db)` - 创建新实例
- `with_config(table_name, mysql_config)` - 使用配置创建
- `init_tables()` - 初始化同步相关表

### CRUD 方法
- `m_add(record)` - 插入记录
- `m_update(id, record)` - 更新记录
- `m_save(record)` - 保存记录
- `m_del(id)` - 删除记录
- `get(where_clause, params)` - 查询记录
- `get_one(id)` - 查询单条
- `count()` - 统计记录数

### 同步方法
- `m_sync_save(record)` - 同步保存（不写 synclog）
- `m_sync_update(id, record)` - 同步更新
- `m_sync_del(id)` - 同步删除
- `get_pending_count()` - 获取待同步记录数
- `mark_synced(idpk_list)` - 标记已同步

## 测试方案

### 主要逻辑测试

#### 测试1：创建实例
```
输入：table_name="test_table", MysqlConfig
步骤：DataSyncMysql::new() 或 with_config()
预期：返回有效实例，table_name 正确
```

#### 测试2：默认值
```
输入：DataSyncMysql::default()
步骤：检查各字段默认值
预期：download_interval=300, upload_interval=300, download_enabled=true
```

### 其它测试（边界、异常等）

#### 测试3：当前时间戳
```
输入：DataSyncMysql::current_time()
步骤：获取当前时间戳
预期：返回有效的时间戳（大于 0）
```

#### 测试4：Debug 输出
```
输入：DataSyncMysql 实例
步骤：format!("{:?}", sync)
预期：输出包含 table_name、apiurl、download_enabled 等字段
```

## 注意事项

- 需要 MySQL 数据库环境才能进行完整测试
- 测试时应使用临时数据库避免数据污染
