# BaseInstance - 工作流实例基类

---

## 管理员指示

无（基础组件）

---

## 第一性目的

工作流实例的统一基类，解决以下问题：
- 所有工作流实例的生命周期管理
- 执行统计、经济统计的统一计算
- 日志记录和错误追踪

---

## 完成标准

### 1. 实例创建后可查询
- 验证方法：创建实例后，查询数据库验证记录存在
- 预期结果：`SELECT * FROM workflow_instance_{date} WHERE id = ?` 返回对应记录

### 2. 执行成功后状态正确
- 验证方法：执行实例后，查询 state 字段
- 预期结果：`state = 2`（已完成），`res = 0`（成功）

### 3. 执行失败后状态正确
- 验证方法：执行失败的实例，查询 state 和 lasterrinfo
- 预期结果：`state = 3`（失败），`lasterrinfo` 包含错误信息

### 4. 统计字段正确更新
- 验证方法：执行后查询 runcount、successcount、executiontime
- 预期结果：`runcount += 1`，`successcount += 1`，`executiontime > 0`

---

## 前置依赖

### 1. 数据库表存在
- 验证方法：`SELECT 1 FROM workflow_instance_{date} LIMIT 1`
- 预期结果：查询成功

### 2. Sqlite78 组件可用
- 验证方法：`Sqlite78::with_default_path()`
- 预期结果：返回有效连接

---

## 测试方案

### 主要逻辑

#### 测试1：实例创建和查询
```
输入：{"id": "test_001", "myname": "测试实例"}
步骤：创建实例 → 保存到数据库 → 查询验证
预期：查询返回的 id = "test_001"，myname = "测试实例"
```

#### 测试2：执行成功流程
```
输入：有效 context78 上下文
步骤：execute() → 查询数据库
预期：state = 2，res = 0，runcount = 1
```

#### 测试3：执行失败流程
```
输入：会触发错误的上下文
步骤：execute() → 查询数据库
预期：state = 3，lasterrinfo 不为空
```

### 边界测试

#### 测试4：空ID自动生成
```
输入：不传 id 参数
预期：自动生成 UUID 格式的 id
```

#### 测试5：超时处理
```
输入：timeout = 1，执行耗时 2 秒的任务
预期：任务被中断，state = 3
```

---

## 知识库

### 状态定义（INTEGER）
- 0：待领取
- 1：执行中
- 2：已完成
- 3：失败

### 核心组件
- `InstanceBase`：基础数据结构
- `LifecycleManager`：生命周期统计（runcount、successcount、errorcount）
- `EconomicManager`：经济统计（costtotal、revenuetotal、roi）

### 数据流向
```
JSON输入 → InstanceBase → execute() → 数据库记录
                ↓
         LifecycleManager 统计更新
                ↓
         EconomicManager 成本计算
```

---

## 好坏示例

### 好示例
```rust
// 创建实例并执行
let mut instance = TestInstance::new();
instance.instance_base.base.id = "test-001".to_string();
let result = instance.execute(context).await;

// 验证数据库
let record = db.query("SELECT * FROM workflow_instance WHERE id = ?", &["test-001"]);
assert_eq!(record.state, 2);
```

### 坏示例
```rust
// 错误：只检查返回值，不验证数据库
let result = instance.execute(context).await;
assert!(result.is_ok()); // 不够，必须查询数据库验证
```

### 坏示例
```rust
// 错误：硬编码状态值
instance.state = "completed".to_string(); // 错误
instance.state = 2; // 正确，状态是 INTEGER
```

### 坏示例
```rust
// 错误：测试时插入模拟数据
db.insert_mock_data("test-001"); // 错误，禁止模拟数据
instance.execute(context).await; // 应该用真实执行产生的数据
```