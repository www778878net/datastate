# State 状态基类模块文档

## 管理员指示
- 本文档描述 State 模块的设计和实现
- 禁止把代码写在 mod.rs, lib.rs，应该在子模块中实现

## 第一性目的
- 提供 3 种基础状态：IDLE、WORKING、ERROR
- 作为所有状态机的基类
- 简单、可靠、可扩展

## 完成标准
- 状态正确流转：IDLE <-> WORKING -> ERROR
- 状态判断方法正确
- 序列化/反序列化正确

## 前置依赖
- serde 库

## 业务逻辑

### 状态定义
```
IDLE = 0    空闲，可接受新任务
WORKING = 1 工作中，拒绝新任务
ERROR = 2   错误，需恢复
```

### 状态转换规则
- IDLE -> WORKING: 开始执行任务
- WORKING -> IDLE: 任务完成
- WORKING -> ERROR: 任务失败
- ERROR -> IDLE: 恢复后可重新开始

### BaseState 字段
- `name`: 实例名称（唯一标识）
- `status`: 状态值

## 测试方案

### 基础状态测试
- [ ] 默认状态为 IDLE
- [ ] 状态名称正确
- [ ] 状态判断方法正确

### 状态转换测试
- [ ] IDLE -> WORKING 转换正确
- [ ] WORKING -> IDLE 转换正确
- [ ] WORKING -> ERROR 转换正确

### 序列化测试
- [ ] JSON 序列化正确
- [ ] JSON 反序列化正确

## 知识库

### 创建状态实例
```rust
let state = BaseState::new("my_instance");
assert!(state.is_idle());
```

### 状态转换
```rust
let mut state = BaseState::new("my_instance");
state.set_working();
assert!(state.is_working());
state.set_error();
assert!(state.is_error());
```

## 好坏示例

### 好示例
- 使用 is_idle() 判断状态
- 使用 set_working()/set_idle() 切换状态

### 坏示例
- 直接修改 status 字段
- 不检查状态就执行操作