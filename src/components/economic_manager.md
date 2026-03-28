# EconomicManager - 经济管理组件

---

## 管理员指示

无（基础组件）

---

## 第一性目的

EconomicManager 管理能力的定价、成本和收入统计：
- 定价管理（基础价格、当前价格）
- 成本统计（总成本、单位成本）
- 收入统计（总收入、总利润、ROI）

---

## 完成标准

### 1. 实例创建成功
- 验证方法：`EconomicManager::default()`
- 预期结果：pricebase=1.0, price=1.0, profittarget=0.2

### 2. 成本/收入统计正确
- 验证方法：调用 add_cost() 和 add_revenue()
- 预期结果：自动更新 profittotal 和 roi

---

## 前置依赖

无

---

## 测试方案

### 主要逻辑

#### 测试1：默认创建
```
输入：无
步骤：EconomicManager::default()
预期：pricebase=1.0, revenuetotal=0.0, roi=0.0
```

#### 测试2：成本收入统计
```
输入：add_cost(10.0), add_revenue(15.0)
步骤：添加成本和收入
预期：profittotal=5.0, roi=50.0
```

---

## 知识库

### 字段说明

#### 定价信息
- `pricebase: f64` - 基础价格（默认1.0）
- `price: f64` - 当前价格（默认1.0）
- `costunit: f64` - 单位成本
- `pricedescription: Value` - 价格描述
- `costdescription: Value` - 成本描述

#### 经济统计
- `revenuetotal: f64` - 总收入
- `costtotal: f64` - 总成本
- `profittotal: f64` - 总利润
- `profittarget: f64` - 目标利润率（默认0.2）
- `roi: f64` - 投资回报率

### 方法说明
- `default()` - 创建默认实例
- `add_cost(&mut self, cost: f64)` - 增加成本
- `add_revenue(&mut self, revenue: f64)` - 增加收入
- `load_from_dict(&mut self, data: &HashMap)` - 从字典加载
- `to_dict(&self) -> HashMap` - 转换为字典

---

## 好坏示例

### 好示例
```rust
let mut manager = EconomicManager::default();
manager.add_cost(10.0);
manager.add_revenue(15.0);
// 自动计算: profittotal=5.0, roi=50.0
```

### 坏示例
```rust
// 错误：手动设置计算字段
manager.profittotal = 5.0; // 应该通过 add_cost/add_revenue 自动计算
manager.roi = 50.0; // 应该自动计算
```
