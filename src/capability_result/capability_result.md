# CapabilityResult 能力执行结果文档

## 管理员指示
- 本文档描述 CapabilityResult 模块的设计和实现
- 禁止把代码写在 mod.rs, lib.rs，应该在子模块中实现

## 第一性目的
- 定义能力执行的标准返回结构
- 提供类型安全的结果处理
- 支持成功/失败快速创建

## 完成标准
- 结果结构正确
- 序列化/反序列化正确
- 成功/失败工厂方法正确

## 前置依赖
- serde 库
- serde_json 库

## 业务逻辑

### 字段说明
- `res`: 执行结果，0=成功，其他=失败
- `errmsg`: 错误信息
- `result`: 业务结果（可选）
- `operation`: 操作描述（可选）

### 工厂方法
- `success()`: 创建成功结果
- `failure()`: 创建失败结果
- `is_success()`: 判断是否成功

## 测试方案

### 基础功能测试
- [ ] 默认值正确
- [ ] 成功结果创建正确
- [ ] 失败结果创建正确
- [ ] 序列化正确

## 知识库

### 创建成功结果
```rust
let result = CapabilityResult::success(
    Some(json!({"key": "value"})),
    Some("操作成功".to_string())
);
assert!(result.is_success());
```

### 创建失败结果
```rust
let result = CapabilityResult::failure("操作失败", -1, None);
assert!(!result.is_success());
assert_eq!(result.errmsg, "操作失败");
```

## 好坏示例

### 好示例
- 使用 success()/failure() 工厂方法
- 使用 is_success() 判断结果

### 坏示例
- 手动构造 res/errmsg 字段
- 忽略错误结果