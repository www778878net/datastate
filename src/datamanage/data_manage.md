# DataManage 数据库状态机管理器文档

## 管理员指示
- 本文档描述 DataManage 模块的设计和实现
- 禁止把代码写在 mod.rs, lib.rs，应该在子模块中实现

## 第一性目的
- 管理所有表的同步状态
- 只负责：验证、日志、统计
- 协调多个 DataState 实例

## 完成标准
- 状态注册成功
- 状态查询正确
- 同步检查正常
- 数据库操作正常

## 前置依赖
- DataState 状态类
- LocalDB 数据库类
- TableConfig 配置类

## 业务逻辑

### 核心功能
- `register()`: 注册表状态
- `unregister()`: 注销表状态
- `get_state()`: 获取状态
- `list_states()`: 列出所有状态
- `sync_once()`: 执行一次同步检查

### 状态管理
- 使用 RwLock 保证线程安全
- 使用 Arc 支持多线程共享
- worker 标识区分不同客户端

### sync_queue 表
- 记录本地变更
- 支持批量同步
- 支持冲突检测

## 测试方案

### 基础功能测试
- [ ] 创建管理器成功
- [ ] 注册状态成功
- [ ] 查询状态正确
- [ ] 列出状态正确

### 同步测试
- [ ] sync_once 执行正常
- [ ] 状态转换正确

## 知识库

### 创建管理器
```rust
let manager = DataManage::new()?;
let manager = DataManage::with_db_path("path/to/db")?;
```

### 注册状态
```rust
let config = TableConfig::new("my_table")
    .with_apiurl("http://example.com/api");
let state = manager.register(config)?;
```

## 好坏示例

### 好示例
- 使用 register() 注册状态
- 使用 get_state() 查询状态

### 坏示例
- 直接修改内部状态
- 不通过管理器操作数据库