# audit_manager

## 概述

AuditManager - 审计日志管理器，管理全局的 AuditLogDataState 单例，提供统一的审计日志记录接口。

## 第一性目的

提供统一的审计日志记录接口：
- 管理全局 AuditLogDataState 单例
- 记录审计日志（计数方式）
- 获取审计日志统计

## 核心类型

### AuditManager

审计日志管理器：
- `audit_log_state: AuditLogDataState` - 审计日志状态机
- `logger: Arc<MyLogger>` - 日志器

## 核心方法

### 实例方法
- `log_audit(tablename, ability, caller)` - 记录审计日志
- `get_audit_logs(tablename, days)` - 获取审计日志
- `get_stats_by_date_range(start_date, end_date)` - 获取指定日期范围的统计

### 全局函数
- `log_audit(tablename, ability, caller)` - 全局审计日志记录
- `get_audit_logs(tablename, days)` - 全局获取审计日志
- `get_stats_by_date_range(start_date, end_date)` - 全局获取日期范围统计

## 示例

```rust
use datastate::dataaudit::audit_manager;

// 记录审计日志
audit_manager::log_audit("testtb", "getone", "TestTb")?;

// 获取审计日志
let logs = audit_manager::get_audit_logs(Some("testtb"), 7);
```

## 测试方案

### 主要逻辑测试

#### 测试1：记录审计日志
```
输入：tablename="testtb", ability="getone", caller="TestTb"
步骤：log_audit()
预期：返回 Ok(())，记录成功
```

#### 测试2：获取审计日志
```
输入：tablename=Some("testtb"), days=7
步骤：get_audit_logs()
预期：返回日志数组（长度 >= 0）
```

### 其它测试（边界、异常等）

#### 测试3：空表名查询
```
输入：tablename=None, days=7
步骤：get_audit_logs(None, 7)
预期：返回所有表的日志
```

#### 测试4：日期范围查询
```
输入：start_date, end_date
步骤：get_stats_by_date_range()
预期：返回指定范围内的统计
```
