# audit_log_state

## 概述

AuditLogDataState - 审计日志数据状态机，管理审计日志表，支持上传下载同步。

## 第一性目的

审计日志记录的核心组件：
- 记录审计日志（计数方式）
- 每天每个调用方对每个方法只有一条记录
- 支持按日期范围查询

## 核心类型

### AuditLogDataState

审计日志数据状态机：
- `base: BaseState` - 基础状态
- `datasync: DataSync` - 同步组件
- `cid` - 公司ID
- `upby` - 操作者

### AuditLogRecord

审计日志记录：
- `apisys` - 系统标识
- `apimicro` - 微服务标识
- `apiobj` - 对象标识
- `ability` - 方法名
- `caller` - 调用方
- `num` - 调用次数
- `dlong` - 总耗时
- `downlen` - 下行数据量

## 核心方法

### 初始化方法
- `new()` - 创建实例
- `from_config(config)` - 从配置创建
- `set_context(cid, upby)` - 设置上下文
- `init_table()` - 初始化表

### 日志方法
- `log_audit(apisys, apimicro, apiobj, ability, caller, elapsed_ms)` - 记录审计日志
- `get_audit_logs(apiobj, days)` - 获取审计日志
- `get_stats_by_date_range(start_date, end_date)` - 获取日期范围统计

## 唯一键设计

唯一键：apisys + apimicro + apiobj + ability + caller
- 第一次调用：插入新记录，num=1
- 后续调用：更新计数，num += 1, dlong += elapsed_ms

## 示例

```rust
use datastate::dataaudit::AuditLogDataState;

let state = AuditLogDataState::new();
state.init_table()?;

// 记录审计日志
state.log_audit("local", "datastate", "testtb", "getone", "Inventory", 150)?;

// 获取日志
let logs = state.get_audit_logs(Some("testtb"), 7);
```

## 测试方案

### 主要逻辑测试

#### 测试1：创建实例
```
输入：AuditLogDataState::new()
步骤：检查 base.name 和 datasync.table_name
预期：都为 "data_audit_log"
```

#### 测试2：SQL 有效性
```
输入：AUDIT_LOG_CREATE_SQL
步骤：验证 SQL 语法
预期：包含 CREATE TABLE、data_audit_log、ability、caller、num、dlong、UNIQUE
```

#### 测试3：记录审计日志
```
输入：apisys="local", apimicro="datastate", apiobj="testtb", ability="msave", caller="inventory", elapsed_ms=150
步骤：log_audit() -> get_audit_logs()
预期：日志记录存在，num=1, dlong=150
```

#### 测试4：计数更新
```
输入：相同唯一键，第二次调用
步骤：两次 log_audit() -> 验证计数
预期：num=2, dlong=150+200=350
```

### 其它测试（边界、异常等）

#### 测试5：空表名查询
```
输入：apiobj=None
步骤：get_audit_logs(None, 10)
预期：返回所有 apiobj 的日志
```

#### 测试6：日期范围查询
```
输入：start_date, end_date
步骤：get_stats_by_date_range()
预期：返回指定范围内的统计
```

#### 测试7：设置上下文
```
输入：cid="company001", upby="admin"
步骤：set_context() -> 验证字段
预期：cid 和 upby 已更新
```
