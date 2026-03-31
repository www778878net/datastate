# Sharding - 分表功能模块

## 概述

Sharding 模块提供分表功能，支持按天（daily）和按月（monthly）分表。

## 第一性目的

实现自动化的分表管理：
- 按时间自动创建分表
- 自动清理过期分表
- 支持按天和按月两种分表策略

## 核心类型

### ShardType

分表类型枚举：
- `Daily` - 按天分表，后缀格式 YYYYMMDD
- `Monthly` - 按月分表，后缀格式 YYYYMM
- `None` - 不分表

### ShardingConfig

分表配置：
- `shard_type` - 分表类型
- `table_sql` - 建表 SQL 模板
- `retention_days` - 保留天数
- `base_table` - 基础表名

### ShardingManager

分表管理器：
- `create_sharding_table()` - 创建分表
- `drop_old_table()` - 删除旧表
- `perform_maintenance()` - 执行分表维护
- `get_all_shard_tables()` - 获取所有分表名称

## 核心方法

### ShardingConfig 方法
- `new(shard_type, base_table)` - 创建配置
- `with_table_sql(sql)` - 设置建表 SQL
- `with_retention(days)` - 设置保留天数
- `get_table_name(date)` - 获取指定日期的表名
- `get_current_table_name()` - 获取当前表名
- `get_date_suffixes(days_back)` - 获取日期后缀列表

## 示例

```rust
use datastate::workflow::sharding::{ShardingConfig, ShardType};

// 按天分表
let config = ShardingConfig::new(ShardType::Daily, "workflow_instance")
    .with_retention(5);

let table_name = config.get_current_table_name();
// 结果: workflow_instance_20260301

// 按月分表
let config = ShardingConfig::new(ShardType::Monthly, "workflow_task");
let table_name = config.get_table_name(Some(NaiveDate::from_ymd_opt(2026, 3, 15).unwrap()));
// 结果: workflow_task_202603
```

## 测试方案

### 主要逻辑测试

#### 测试1：按天分表配置
```
输入：ShardType::Daily, base_table="workflow_instance"
步骤：get_current_table_name()
预期：表名格式为 workflow_instance_YYYYMMDD
```

#### 测试2：指定日期表名
```
输入：date=2026-03-01, ShardType::Daily
步骤：get_table_name(Some(date))
预期：返回 "workflow_instance_20260301"
```

#### 测试3：按月分表配置
```
输入：ShardType::Monthly, base_table="workflow_task"
步骤：get_table_name(Some(2026-03-15))
预期：返回 "workflow_task_202603"
```

### 其它测试（边界、异常等）

#### 测试4：不分表配置
```
输入：ShardType::None
步骤：get_current_table_name()
预期：返回基础表名，无后缀
```

#### 测试5：日期后缀列表
```
输入：days_back=3, ShardType::Daily
步骤：get_date_suffixes(3)
预期：返回 4 个表名（今天 + 过去3天）
```

#### 测试6：保留天数配置
```
输入：retention_days=5
步骤：with_retention(5)
预期：config.retention_days = 5
```
