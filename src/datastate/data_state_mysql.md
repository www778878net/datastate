# DataStateMysql - MySQL 版本数据状态类

## 概述

DataStateMysql 是 MySQL 版本的数据状态类，与 DataState 功能相同，但使用 MySQL 作为后端。

## 第一性目的

组合组件提供 MySQL 数据库操作：
- DataSyncMysql: 同步队列管理
- DataAudit: 权限检查和审计日志
- BaseState: 基础状态管理

## 核心类型

### DataStateMysql

MySQL 版本数据状态类：
- `base: BaseState` - 基础状态
- `datasync: DataSyncMysql` - 同步组件（MySQL 版本）
- `audit: DataAudit` - 审计组件

## 核心方法

### 创建方法
- `with_config(table_name, mysql_config)` - 使用 MySQL 配置创建
- `with_db(table_name, db)` - 使用已有 Mysql78 实例创建
- `default()` - 默认创建

### CRUD 方法（带权限检查）
- `m_add(record, caller, summary)` - 插入记录
- `m_update(id, record, caller, summary)` - 更新记录
- `m_save(record, caller, summary)` - 保存记录
- `m_del(id, caller, summary)` - 删除记录
- `get(where_clause, params, caller, summary)` - 查询记录
- `get_one(id, caller, summary)` - 查询单条
- `count(caller, summary)` - 统计记录数

### 同步方法（不写 sync_queue）
- `m_sync_save(record)` - 同步保存
- `m_sync_update(id, record)` - 同步更新
- `m_sync_del(id)` - 同步删除

## 测试方案

### 主要逻辑测试

#### 测试1：创建实例
```
输入：table_name="test_table", MysqlConfig
步骤：DataStateMysql::with_config()
预期：返回有效实例，base.name = "test_table"
```

#### 测试2：默认实例
```
输入：DataStateMysql::default()
步骤：创建默认实例
预期：base.name = ""
```

### 其它测试（边界、异常等）

#### 测试3：Debug 输出
```
输入：DataStateMysql 实例
步骤：format!("{:?}", state)
预期：输出包含 base、datasync、audit 字段
```

#### 测试4：初始化同步表
```
输入：有效 MySQL 连接
步骤：init_tables()
预期：同步相关表创建成功
```

## 注意事项

- 需要 MySQL 数据库环境才能进行完整测试
- CRUD 方法自动进行权限检查和审计日志记录
- 同步方法不写 sync_queue，避免循环同步
