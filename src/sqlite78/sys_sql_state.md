# SysSqlSqliteState - SQLite 版 SQL 统计状态管理

## 概述

SysSqlSqliteState 管理 sys_sql 表的 SQLite 操作，用于 SQL 效率统计。

## 第一性目的

提供 SQL 效率统计功能：
- 记录 SQL 执行时间和次数
- 获取慢 SQL 列表
- 获取高频 SQL 列表

## 核心类型

### SysSqlSqliteState

SQL 统计状态管理：
- `db: Sqlite78` - SQLite 数据库连接

### SysSqlData

SQL 统计数据：
- `cmdtext` - SQL 语句
- `cmdtextmd5` - SQL MD5（去重）
- `num` - 执行次数
- `dlong` - 总耗时（毫秒）
- `downlen` - 下行数据量

## 核心方法

### 初始化方法
- `new(db)` - 创建新实例
- `default()` - 默认实例
- `create_table(up)` - 创建表

### 统计方法
- `log_sql(data, up)` - 记录 SQL 执行统计
- `get_slow_sql(min_dlong, limit, up)` - 获取慢 SQL 列表
- `get_hot_sql(min_num, limit, up)` - 获取高频 SQL 列表

## 表结构

```sql
CREATE TABLE IF NOT EXISTS sys_sql (
    cid TEXT NOT NULL DEFAULT '',
    apisys TEXT NOT NULL DEFAULT '',
    apimicro TEXT NOT NULL DEFAULT '',
    apiobj TEXT NOT NULL DEFAULT '',
    cmdtext TEXT NOT NULL,
    num INTEGER NOT NULL DEFAULT 0,
    dlong INTEGER NOT NULL DEFAULT 0,
    downlen INTEGER NOT NULL DEFAULT 1,
    upby TEXT NOT NULL DEFAULT '',
    cmdtextmd5 TEXT NOT NULL DEFAULT '',
    uptime TEXT NOT NULL DEFAULT '',
    idpk INTEGER PRIMARY KEY AUTOINCREMENT,
    id TEXT NOT NULL,
    UNIQUE(cmdtextmd5),
    UNIQUE(id)
)
```

## 测试方案

### 主要逻辑测试

#### 测试1：创建实例
```
输入：Sqlite78 实例
步骤：SysSqlSqliteState::new(db)
预期：返回有效实例
```

#### 测试2：创建表
```
输入：UpInfo
步骤：create_table(up)
预期：返回 "ok"
```

#### 测试3：记录 SQL 统计
```
输入：SysSqlData { cmdtext, cmdtextmd5, dlong, ... }
步骤：log_sql(data, up)
预期：记录成功，或更新已有记录
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

#### 测试6：默认实例
```
输入：无
步骤：SysSqlSqliteState::default()
预期：返回使用默认 Sqlite78 的实例
```
