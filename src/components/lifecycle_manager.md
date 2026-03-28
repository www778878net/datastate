# LifecycleManager - 生命周期管理组件

---

## 管理员指示

无（基础组件）

---

## 第一性目的

LifecycleManager 管理能力的生命周期和执行统计：
- 时间管理（创建时间、开始时间、结束时间）
- 执行统计（运行次数、成功次数、错误次数、成功率）
- 最后记录（最后运行时间、最后成功/错误信息）

---

## 完成标准

### 1. 实例创建成功
- 验证方法：`LifecycleManager::default()`
- 预期结果：runcount=0, successcount=0, errorcount=0

### 2. 执行统计正确
- 验证方法：调用 mark_started() → record_success()
- 预期结果：runcount=1, successcount=1, successrate=100.0

---

## 前置依赖

无

---

## 测试方案

### 主要逻辑

#### 测试1：默认创建
```
输入：无
步骤：LifecycleManager::default()
预期：runcount=0, successcount=0, errorcount=0, successrate=0.0
```

#### 测试2：执行流程
```
输入：无
步骤：mark_started() → record_success(1.5)
预期：runcount=1, successcount=1, executiontime=1.5, successrate=100.0
```

---

## 知识库

### 字段说明

#### 时间管理
- `createtime: Option<DateTime<Local>>` - 创建时间
- `starttime: Option<DateTime<Local>>` - 开始时间
- `endtime: Option<DateTime<Local>>` - 结束时间

#### 执行统计
- `runcount: i32` - 运行次数
- `successcount: i32` - 成功次数
- `errorcount: i32` - 错误次数
- `successrate: f64` - 成功率（百分比）
- `executiontime: f64` - 执行时间(秒)

#### 最后记录
- `lastruntime: Option<DateTime<Local>>` - 最后运行时间
- `lastoktime: Option<DateTime<Local>>` - 最后成功时间
- `lasterrortime: Option<DateTime<Local>>` - 最后错误时间
- `lastokinfo: Value` - 最后成功信息
- `lasterrinfo: Value` - 最后错误信息

### 方法说明
- `default()` - 创建默认实例
- `mark_started(&mut self)` - 标记开始执行
- `record_success(&mut self, execution_time: f64)` - 记录执行成功
- `record_error(&mut self)` - 记录执行错误
- `load_from_dict(&mut self, data: &HashMap)` - 从字典加载
- `to_dict(&self) -> HashMap` - 转换为字典

---

## 好坏示例

### 好示例
```rust
let mut manager = LifecycleManager::default();
manager.mark_started();
// 执行业务逻辑...
manager.record_success(1.5);
// runcount=1, successcount=1, successrate=100.0
```

### 坏示例
```rust
// 错误：手动设置计算字段
manager.successrate = 100.0; // 应该通过 record_success 自动计算
```
