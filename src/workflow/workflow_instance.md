# WorkflowInstance - 工作流实例表（分表）

---

## 管理员指示

无（基础组件）

---

## 第一性目的

工作流实例的数据库管理类，解决以下问题：
- 按天分表存储工作流实例数据
- 自动创建和清理过期分表（保留7天）
- 提供完整的 CRUD 操作
- 支持工作流状态管理

---

## 完成标准

### 1. 分表实例创建成功
- 验证方法：调用 `WorkflowInstance::with_default_path()`
- 预期结果：返回有效实例，表名格式为 `workflow_instance_YYYYMMDD`

### 2. 插入工作流实例
- 验证方法：调用 `insert()` 方法
- 预期结果：返回生成的实例ID

### 3. 状态更新正确
- 验证方法：调用 `mark_completed()` 或 `mark_failed()`
- 预期结果：数据库中 state 字段正确更新

---

## 前置依赖

### 1. Sqlite78 组件可用
- 验证方法：`Sqlite78::with_default_path()`
- 预期结果：返回有效连接

### 2. ShardingManager 组件可用
- 验证方法：创建分表实例
- 预期结果：分表创建成功

---

## 测试方案

### 主要逻辑

#### 测试1：分表实例创建
```
输入：无参数
步骤：WorkflowInstance::with_default_path()
预期：实例创建成功，表名格式正确
```

#### 测试2：工作流实例CRUD
```
输入：工作流实例数据
步骤：insert() -> get_by_id() -> update() -> delete()
预期：所有操作成功，数据一致性正确
```

#### 测试3：状态管理
```
输入：工作流ID和状态
步骤：mark_completed() 或 mark_failed()
预期：状态和时间戳正确更新
```

### 边界测试

#### 测试4：分页查询
```
输入：条件、偏移量、限制
步骤：query_page()
预期：返回正确分页数据和总数
```

#### 测试5：分表维护
```
输入：无
步骤：perform_maintenance()
预期：过期表被清理，当前表存在
```

---

## 知识库

### 状态定义（INTEGER）
- 0：待领取
- 1：执行中
- 2：已完成
- 3：失败
- 6：警告（完成但有警告）

### 核心方法

| 方法 | 说明 |
|------|------|
| `new(db)` | 创建不分表实例 |
| `with_sharding(db)` | 创建分表实例 |
| `with_default_path()` | 使用默认路径创建分表实例 |
| `with_path(path)` | 使用指定路径创建分表实例 |
| `get_table_name()` | 获取当前表名 |
| `insert(data, up)` | 插入工作流实例 |
| `get(id, up)` | 根据ID查询 |
| `get_by_id(id, up)` | 根据ID查询 |
| `update(data, up)` | 更新实例 |
| `update_state(id, state, up)` | 更新状态 |
| `mark_completed(id, info, up)` | 标记完成 |
| `mark_failed(id, errinfo, up)` | 标记失败 |
| `get_running(up)` | 查询运行中的实例 |
| `get_by_workflow(idworkflowdefinition, up)` | 按工作流定义查询 |
| `query_list(condition, params, up)` | 条件查询 |
| `query_page(condition, params, offset, limit, up)` | 分页查询 |
| `delete(id, up)` | 删除实例 |
| `perform_maintenance()` | 执行分表维护 |
| `create_today_table()` | 创建今天的分表 |
| `get_db()` | 获取数据库引用 |

### 数据流向
```
JSON数据 -> WorkflowInstance -> 分表存储
                ↓
         状态管理方法
                ↓
         实例状态更新
```

---

## 好坏示例

### 好示例
```rust
// 创建分表实例
let instance = WorkflowInstance::with_default_path()?;

// 插入工作流
let id = instance.insert(&data, &up)?;

// 标记完成
instance.mark_completed(&id, "执行成功", &up)?;
```

### 坏示例
```rust
// 错误：使用 new() 创建不分表实例，无法享受分表功能
let instance = WorkflowInstance::new(db); // 不推荐
```

### 坏示例
```rust
// 错误：不处理分表创建失败
let instance = WorkflowInstance::with_default_path().unwrap(); // 应该处理错误
```
