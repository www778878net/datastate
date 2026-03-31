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

### 主要逻辑测试

#### 测试1：默认状态为 IDLE
```
输入：BaseState::new("test")
步骤：创建新实例，检查状态
预期：is_idle() = true，is_working() = false，is_error() = false
```

#### 测试2：状态名称正确
```
输入：StateStatus 枚举值
步骤：调用 name() 方法
预期：IDLE.name() = "IDLE"，WORKING.name() = "WORKING"，ERROR.name() = "ERROR"
```

#### 测试3：状态判断方法正确
```
输入：不同状态的 BaseState
步骤：调用 is_idle()、is_working()、is_error()
预期：对应方法返回 true
```

### 状态转换测试

#### 测试4：IDLE -> WORKING 转换
```
输入：set_working()
步骤：state.set_working()
预期：is_working() = true，is_idle() = false
```

#### 测试5：WORKING -> IDLE 转换
```
输入：set_idle()
步骤：state.set_working() -> state.set_idle()
预期：is_idle() = true，is_working() = false
```

#### 测试6：WORKING -> ERROR 转换
```
输入：set_error()
步骤：state.set_working() -> state.set_error()
预期：is_error() = true，is_idle() = false
```

### 其它测试（边界、异常等）

#### 测试7：状态从整数转换
```
输入：StateStatus::from_i32()
步骤：from_i32(0), from_i32(1), from_i32(2), from_i32(99)
预期：0->IDLE, 1->WORKING, 2->ERROR, 99->IDLE（默认）
```

#### 测试8：JSON 序列化正确
```
输入：BaseState 实例
步骤：serde_json::to_string(&state)
预期：JSON 包含 name 和 status 字段
```

#### 测试9：JSON 反序列化正确
```
输入：JSON 字符串
步骤：serde_json::from_str(json)
预期：正确解析 name 和 status 字段
```

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