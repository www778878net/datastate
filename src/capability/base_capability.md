# BaseCapability - 能力基类

---

## 管理员指示

无（基础组件）

---

## 第一性目的

能力的统一基类，解决以下问题：
- 所有能力的生命周期管理
- 执行统计、经济统计的统一计算
- 日志记录和错误追踪
- 支持保存到 workflow_capability 表

---

## 完成标准

### 1. 能力创建后可执行
- 验证方法：创建能力后，调用 execute 方法
- 预期结果：返回 `res = 0`（成功）

### 2. 执行成功后状态正确
- 验证方法：执行能力后，查询 state 字段
- 预期结果：`state = 2`（已完成），`res = 0`（成功）

### 3. 执行失败后状态正确
- 验证方法：执行失败的能力，查询 state 和 lasterrinfo
- 预期结果：`state = 3`（失败），`lasterrinfo` 包含错误信息

### 4. 统计字段正确更新
- 验证方法：执行后查询 runcount、successcount、executiontime
- 预期结果：`runcount += 1`，`successcount += 1`，`executiontime > 0`

---

## 前置依赖

### 1. Sqlite78 组件可用
- 验证方法：`LocalDB::new()`
- 预期结果：返回有效连接

### 2. MyLogger 组件可用
- 验证方法：`let logger = mylogger!();`
- 预期结果：返回有效日志器

---

## 测试方案

### 主要逻辑

#### 测试1：能力创建和执行
```
输入：{"id": "test-cap-001", "myname": "测试能力"}
步骤：创建能力 → 执行 → 验证结果
预期：返回的 res = 0，state = 2
```

#### 测试2：执行成功流程
```
输入：有效 context78 上下文
步骤：execute() → 检查状态
预期：state = 2，res = 0，runcount = 1
```

#### 测试3：执行失败流程
```
输入：会触发错误的上下文
步骤：execute() → 检查状态
预期：state = 3，lasterrinfo 不为空
```

### 边界测试

#### 测试4：空ID自动生成
```
输入：不传 id 参数
预期：自动生成 UUID 格式的 id
```

#### 测试5：简化模式
```
输入：issimple = true
预期：不统计生命周期和经济数据
```

---

## 知识库

### 状态定义（INTEGER）
- 0：待领取
- 1：执行中
- 2：已完成
- 3：失败

### 核心组件
- `BaseEntity`：基础数据结构
- `LifecycleManager`：生命周期统计（runcount、successcount、errorcount）
- `EconomicManager`：经济统计（costtotal、revenuetotal、roi）

### 数据流向
```
JSON输入 → CapabilityBase → execute() → 返回结果
                ↓
         LifecycleManager 统计更新
                ↓
         EconomicManager 成本计算
```

---

## 好坏示例

### 好示例
```rust
// 创建能力并执行
let mut capability = TestCapability::new();
capability.capability_base.economic.price = 10.0;
let result = capability.execute(context).await;

// 验证结果
assert_eq!(result.get("res").and_then(|v| v.as_i64()), Some(0));
assert_eq!(capability.base().state, 2);
```

### 坏示例
```rust
// 错误：只检查返回值，不验证状态
let result = capability.execute(context).await;
assert!(result.is_ok()); // 不够，必须检查 state
```

### 坏示例
```rust
// 错误：硬编码状态值
capability.base_mut().state = "completed".to_string(); // 错误
capability.base_mut().state = 2; // 正确，状态是 INTEGER
```

### 坏示例
```rust
// 错误：测试时插入模拟数据
db.insert_mock_data("test-cap-001"); // 错误，禁止模拟数据
capability.execute(context).await; // 应该用真实执行产生的数据
```