# Workflow - 工作流存储层

## 第一性目的

管理工作流相关的数据存储，包括能力定义、任务实例和工作流实例的持久化。支持分表管理，适用于大规模工作流场景。

## 主要功能

| 功能 | 说明 |
|------|------|
| 能力定义 | workflow_capability 表，存储能力模板、配置、价格 |
| 任务实例 | workflow_task 表，每次执行产生的任务实例 |
| 工作流实例 | workflow_instance 表，工作流执行实例 |
| 分表管理 | 自动创建和管理分表 |

## 表关系

```
workflow_capability (定义表)
       ↓
workflow_task (实例表) ← workflow_instance (实例表)
```

## 导出模块

| 模块 | 说明 |
|------|------|
| ShardingConfig | 分表配置 |
| ShardType | 分表类型 |
| ShardingManager | 分表管理器 |
| WorkflowCapability | 能力定义 |
| WorkflowInstance | 工作流实例 |
| WorkflowTask | 任务实例 |

## 测试方案

### 主要逻辑测试

#### 测试1：模块导出
```
输入：use datastate::workflow::*;
步骤：验证各组件可访问
预期：WorkflowCapability、WorkflowInstance、WorkflowTask 可用
```

#### 测试2：初始化工作流表
```
输入：数据库连接、UpInfo
步骤：init_workflow_tables()
预期：所有工作流表创建成功
```

### 其它测试（边界、异常等）

#### 测试3：组件功能测试
```
输入：各组件测试用例
步骤：参见各组件独立测试方案
预期：各组件测试通过
```

## 使用方式

```rust
use datastate::workflow::{WorkflowCapability, WorkflowInstance, WorkflowTask};

// 初始化工作流表
init_workflow_tables(&mut db, &up)?;
```

## 相关文件

| 文件 | 说明 |
|------|------|
| mod.rs | 模块入口 |
| sharding.rs | 分表管理 |
| workflow_capability.rs | 能力定义表操作 |
| workflow_instance.rs | 工作流实例表操作 |
| workflow_task.rs | 任务实例表操作 |
