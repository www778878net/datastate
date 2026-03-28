# WorkflowTask 工作流任务管理

## 管理员指示
- 本文档描述 WorkflowTask 模块的设计和实现
- WorkflowTask 是任务实例表管理类，按天分表

## 第一性目的
- 管理工作流任务实例（每次能力执行产生的任务实例记录）
- 按天分表，保留7天数据
- 提供任务的增删改查和状态管理

## 完成标准
- 任务插入成功
- 任务查询正确
- 状态更新正常
- 分表维护正常

## 前置依赖
- Sqlite78 数据库类
- ShardingManager 分表管理器
- UpInfo 更新信息类

## 业务逻辑

### 表关系说明
- **workflow_capability**: 能力【定义表】- 存储能力模板、配置、价格等静态定义
- **workflow_task**: 任务【实例表】- 每次能力执行产生的任务实例记录

### 任务状态（state字段）
| 值 | 说明 |
|----|------|
| 0 | 待领取 |
| 1 | 执行中 |
| 2 | 已完成 |
| 3 | 失败 |
| 6 | 警告（完成但有警告） |

### 主要方法

| 方法 | 说明 |
|------|------|
| new(db) | 创建新实例（不分表） |
| with_sharding(db) | 创建分表实例 |
| with_default_path() | 使用默认数据库路径创建分表实例 |
| with_path(path) | 使用指定路径创建分表实例 |
| get_table_name() | 获取当前表名 |
| perform_maintenance() | 执行分表维护 |
| create_today_table() | 创建今天的分表 |
| insert(data, up) | 插入任务记录 |
| get(id, up) | 根据 ID 查询任务 |
| update_state(id, state, up) | 更新任务状态 |
| mark_completed(id, info, up) | 标记任务完成 |
| mark_failed(id, errinfo, up) | 标记任务失败 |
| get_by_instance(idworkflowinstance, up) | 查询工作流实例的所有任务 |
| get_by_state(state, up) | 查询指定状态的任务 |
| get_db() | 获取底层数据库引用 |
| get_by_id(id) | 根据 ID 查询记录 |
| update(data) | 更新记录 |

## 测试方案

### 基础功能测试
- [ ] 创建 WorkflowTask 实例成功
- [ ] 创建今天的分表成功
- [ ] 插入任务记录成功
- [ ] 查询任务正确

### 分表测试
- [ ] 分表创建正确
- [ ] 分表维护正常
- [ ] 索引创建成功

### 状态管理测试
- [ ] mark_completed 状态更新正确
- [ ] mark_failed 状态更新正确
- [ ] get_by_state 查询正确

## 知识库

### 创建实例
```rust
// 使用默认路径创建分表实例
let task = WorkflowTask::with_default_path()?;

// 使用指定路径创建
let task = WorkflowTask::with_path("path/to/db")?;

// 不分表模式
let db = Sqlite78::with_default_path();
let task = WorkflowTask::new(db);
```

### 插入任务
```rust
let mut data = HashMap::new();
data.insert("id".to_string(), json!("task_001"));
data.insert("myname".to_string(), json!("测试任务"));
data.insert("idcapability".to_string(), json!("cap_001"));
data.insert("idworkflowinstance".to_string(), json!("wf_001"));

let up = UpInfo::new();
let id = task.insert(&data, &up)?;
```

### 状态更新
```rust
// 标记完成
task.mark_completed("task_001", "执行成功", &up)?;

// 标记失败
task.mark_failed("task_001", "执行失败: xxx", &up)?;

// 更新状态
task.update_state("task_001", 1, &up)?;
```

## 好坏示例

### 好示例
- 使用 with_default_path 创建分表实例
- 使用 mark_completed/mark_failed 更新状态
- 通过 get_by_instance 查询工作流实例的任务

### 坏示例
- 直接操作分表 SQL
- 不使用分表管理器
- 忽略任务状态转换规则
