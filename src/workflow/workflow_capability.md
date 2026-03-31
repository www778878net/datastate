# workflow_capability

## 概述

WorkflowCapability - 工作流能力定义表管理。

## 第一性目的

管理工作流能力定义表（workflow_capability）的 CRUD 操作：
- 能力定义的存储和查询
- 能力状态管理
- 能力价格和成本配置

## 完成标准

- 表初始化成功
- CRUD 操作正常
- 状态更新正确

## 核心类型

### WorkflowCapability

管理工作流能力定义的结构体：
- `db: Sqlite78` - SQLite 数据库实例

## 核心方法

### 初始化方法
- `new(db: Sqlite78)` - 使用指定数据库创建实例
- `with_default_path()` - 使用默认路径创建实例
- `with_path(path: &str)` - 使用指定路径创建实例
- `init_table()` - 初始化表结构

### CRUD 方法
- `insert(data, up)` - 插入或更新能力定义
- `get(id, up)` - 根据 ID 查询能力定义
- `update_state(id, state, up)` - 更新能力状态
- `get_enabled(up)` - 查询所有启用的能力

## 示例

```rust
use datastate::workflow::WorkflowCapability;
use std::collections::HashMap;
use serde_json::Value;

// 创建实例
let cap = WorkflowCapability::with_default_path()?;
cap.init_table()?;

// 插入能力定义
let mut data = HashMap::new();
data.insert("id".to_string(), Value::String("cap_001".to_string()));
data.insert("capability".to_string(), Value::String("test_capability".to_string()));
cap.insert(&data, &UpInfo::new())?;

// 查询
let found = cap.get("cap_001", &UpInfo::new())?;
```

## 测试方案

### 主要逻辑测试

#### 测试1：初始化表
```
输入：临时数据库路径
步骤：WorkflowCapability::with_path() -> init_table()
预期：初始化成功，无错误
```

#### 测试2：插入和查询
```
输入：能力定义数据 {id: "cap_001", capability: "test_capability"}
步骤：insert() -> get()
预期：查询结果与插入数据一致
```

### 其它测试（边界、异常等）

#### 测试3：查询不存在的能力
```
输入：不存在的 ID
步骤：get("notexist")
预期：返回 None
```

#### 测试4：更新状态
```
输入：id="cap_001", state=2
步骤：update_state() -> get()
预期：状态已更新为 2
```

#### 测试5：获取启用能力列表
```
输入：多个能力定义，部分 state=1，部分 state=0
步骤：get_enabled()
预期：只返回 state=1 的能力
```
