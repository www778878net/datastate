# SysWarnSqliteState - SQLite 版警告日志状态管理

## 概述

SysWarnSqliteState 管理 sys_warn 表的 SQLite 操作，用于调试跟踪和错误记录。

## 第一性目的

提供警告日志记录功能：
- 记录调试日志、错误日志
- 按类型查询日志
- 清理旧记录

## 核心类型

### SysWarnSqliteState

警告日志状态管理：
- `db: Sqlite78` - SQLite 数据库连接

### SysWarnData

警告日志数据：
- `kind` - 日志类型（debug_xxx, err_xxx）
- `apimicro` - 微服务名
- `apiobj` - API对象名
- `content` - 日志内容
- `upby` - 操作者

## 核心方法

### 初始化方法
- `new(db)` - 创建新实例
- `default()` - 默认实例
- `create_table(up)` - 创建表

### 日志方法
- `insert(data, up)` - 插入警告记录
- `get_by_kind(kind, up)` - 查询指定类型的警告
- `clean_old(keep_count, up)` - 删除旧记录（保留最近N条）

## 测试方案

### 主要逻辑测试

#### 测试1：创建实例
```
输入：Sqlite78 实例
步骤：SysWarnSqliteState::new(db)
预期：返回有效实例
```

#### 测试2：创建表
```
输入：UpInfo
步骤：create_table(up)
预期：返回 "ok"
```

#### 测试3：插入警告记录
```
输入：SysWarnData { kind="debug_test", content="测试内容", ... }
步骤：insert(data, up)
预期：返回 insert_id > 0
```

### 其它测试（边界、异常等）

#### 测试4：按类型查询
```
输入：kind="debug_test"
步骤：get_by_kind("debug_test", up)
预期：返回匹配的记录列表
```

#### 测试5：清理旧记录
```
输入：keep_count=10
步骤：插入多条记录 -> clean_old(10, up)
预期：只保留最近 10 条记录
```

#### 测试6：默认实例
```
输入：无
步骤：SysWarnSqliteState::default()
预期：返回使用默认 Sqlite78 的实例
```
