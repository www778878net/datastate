# DataState - 通用数据状态类

---

## 管理员指示

无（基础组件）

---

## 第一性目的

DataState 是一个组合类，统一管理数据同步和审计功能：
- 组合 BaseState、DataSync、DataAudit 三个组件
- 提供 CRUD 代理方法，自动进行权限检查和审计日志
- 支持本地操作和服务器同步

---

## 完成标准

### 1. 创建实例成功
- 验证方法：`DataState::from_config(&config)` 或 `DataState::with_db(table_name, db)`
- 预期结果：返回包含 base、datasync、audit 三个组件的实例

### 2. CRUD 方法带权限检查
- 验证方法：调用 m_add/m_update/m_del 等方法
- 预期结果：先检查权限，再执行操作

### 3. 同步方法不写 sync_queue
- 验证方法：调用 m_sync_save/m_sync_update/m_sync_del
- 预期结果：不产生待同步记录

---

## 前置依赖

### 1. BaseState 组件可用
- 提供三态基础能力（IDLE/WORKING/ERROR）

### 2. DataSync 组件可用
- 提供同步队列管理

### 3. DataAudit 组件可用
- 提供权限检查和审计日志

---

## 测试方案

### 主要逻辑测试

#### 测试1：从配置创建实例
```
输入：TableConfig { name: "test_table", apiurl: "http://api/test_table/get", ... }
步骤：
  let config = TableConfig::new("test_table");
  let state = DataState::from_config(&config);
预期：返回有效实例，datasync.table_name = "test_table"，base.name = "test_table"
```

#### 测试2：使用指定数据库创建实例
```
输入：table_name = "my_table", db = LocalDB::default_instance()
步骤：
  let db = LocalDB::default_instance().unwrap();
  let state = DataState::with_db("my_table", db);
预期：返回有效实例，datasync.table_name = "my_table"
```

#### 测试3：CRUD 操作 - 插入记录
```
输入：record = {"id": "test_001", "name": "测试"}
步骤：
  state.m_add(&record, "test_caller", "测试插入")
预期：权限检查通过，返回新记录 ID
```

#### 测试4：CRUD 操作 - 查询记录
```
输入：id = "test_001"
步骤：
  state.get_one("test_001", "test_caller", "测试查询")
预期：返回记录 {"id": "test_001", "name": "测试"}
```

#### 测试5：CRUD 操作 - 更新记录
```
输入：id = "test_001", record = {"name": "更新后"}
步骤：
  state.m_update("test_001", &record, "test_caller", "测试更新")
预期：返回 true（更新成功）
```

#### 测试6：同步操作 - 同步保存
```
输入：record = {"id": "sync_001", "name": "同步数据"}
步骤：
  let pending_before = state.datasync.get_pending_count();
  state.m_sync_save(&record);
  let pending_after = state.datasync.get_pending_count();
预期：记录插入成功，pending_after == pending_before（不产生待同步记录）
```

### 其它测试（边界、异常等）

#### 测试7：默认实例创建
```
输入：DataState::default()
步骤：创建默认实例
预期：返回包含 base、datasync、audit 三个组件的实例，base.name = ""
```

#### 测试8：空表名实例创建
```
输入：table_name = ""
步骤：DataState::new("")
预期：实例创建成功，table_name = ""
```

#### 测试9：雪花ID生成
```
输入：调用 next_id_string()
步骤：验证生成的 ID 格式
预期：ID 不为空，长度 >= 18
```

#### 测试10：删除记录
```
输入：id = "test_001"
步骤：
  state.m_del("test_001", "test_caller", "测试删除")
预期：返回 true（删除成功）
```

#### 测试11：统计记录数
```
输入：无
步骤：
  state.count("test_caller", "测试统计")
预期：返回记录数量
```

---

## 知识库

### 结构体定义
```rust
pub struct DataState {
    /// 基础状态
    pub base: BaseState,
    /// 同步组件（包含数据库实例）
    pub datasync: DataSync,
    /// 审计组件（权限检查和日志记录）
    pub audit: DataAudit,
}
```

### 创建方法
- `from_config(config: &TableConfig)` - 从配置创建
- `with_db(table_name: &str, db: LocalDB)` - 使用指定数据库创建
- `default()` - 默认创建

### CRUD 方法（带权限检查和审计日志）
- `m_add(record, caller, summary)` - 插入记录
- `m_update(id, record, caller, summary)` - 更新记录
- `m_save(record, caller, summary)` - 保存记录（存在更新，不存在插入）
- `m_del(id, caller, summary)` - 删除记录
- `get(where_clause, params, caller, summary)` - 查询记录
- `get_one(id, caller, summary)` - 查询单条记录
- `count(caller, summary)` - 统计记录数
- `do_get(sql, params, caller, summary)` - 执行任意SQL查询
- `do_m(sql, params, caller, summary)` - 执行任意SQL更新

### 同步方法（不写 sync_queue）
- `m_sync_save(record)` - 同步保存记录
- `m_sync_update(id, record)` - 同步更新记录
- `m_sync_del(id)` - 同步删除记录

### 数据流向
```
调用方 → DataState.m_xxx()
              ↓
         audit.check_permission() → 权限检查
              ↓
         datasync.m_xxx() → 执行操作
              ↓
         自动写 sync_queue（同步方法除外）
```

---

## 好坏示例

### 好示例
```rust
// 从配置创建
let config = TableConfig::new("my_table");
let state = DataState::from_config(&config);

// 带权限检查的CRUD
state.m_add(&record, "order_service", "添加订单记录")?;

// 同步方法（不写sync_queue）
state.m_sync_save(&remote_record)?;
```

### 坏示例
```rust
// 错误：直接操作 datasync 跳过权限检查
state.datasync.m_add(&record)?; // 跳过了 audit 检查

// 错误：同步方法用于本地操作
state.m_sync_save(&local_record)?; // 本地操作应该用 m_add/m_save
```
