# SysSqlMysqlState - MySQL 版 SQL 统计状态管理

## 概述

SysSqlMysqlState 管理 sys_sql 表的 MySQL 操作，用于 SQL 效率统计。

## 第一性目的

提供 MySQL 版本的 SQL 效率统计功能：
- 记录 SQL 执行时间和次数
- 获取慢 SQL 列表
- 获取高频 SQL 列表

## 核心类型

### SysSqlMysqlState

MySQL 版 SQL 统计状态管理：
- `db: Mysql78` - MySQL 数据库连接

## 核心方法

### 初始化方法
- `new(db)` - 创建新实例
- `default()` - 默认实例
- `create_table(up)` - 创建表

### 统计方法
- `log_sql(data, up)` - 记录 SQL 执行统计
- `get_slow_sql(min_dlong, limit, up)` - 获取慢 SQL 列表
- `get_hot_sql(min_num, limit, up)` - 获取高频 SQL 列表

## 测试方案

### 主要逻辑测试

#### 测试1：创建实例
```
输入：Mysql78 实例
步骤：SysSqlMysqlState::new(db)
预期：返回有效实例
```

#### 测试2：创建表
```
输入：MysqlUpInfo
步骤：create_table(up)
预期：返回 "ok"
```

#### 测试3：记录 SQL 统计
```
输入：SysSqlData { cmdtext, cmdtextmd5, dlong, ... }
步骤：log_sql(data, up)
预期：记录成功
```

### 其它测试（边界、异常等）

#### 测试4：获取慢 SQL
```
输入：min_dlong=100, limit=10
步骤：get_slow_sql(100, 10, up)
预期：返回耗时 > 100ms 的 SQL 列表
```

#### 测试5：获取高频 SQL
```
输入：min_num=10, limit=10
步骤：get_hot_sql(10, 10, up)
预期：返回执行次数 > 10 的 SQL 列表
```

## 注意事项

- 需要 MySQL 数据库环境才能进行完整测试
