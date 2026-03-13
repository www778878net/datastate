# DataManage 数据库状态机管理器文档

## 管理员指示
- 本文档描述 DataManage 模块的设计和实现
- 禁止把代码写在 mod.rs, lib.rs，应该在子模块中实现

## 第一性目的
- 管理所有表的同步状态
- 只负责：验证、日志、统计
- 协调多个 DataState 实例
- **提供 cid 隔离验证，确保用户只能修改自己 cid/uid 的数据**

## 完成标准
- 状态注册成功
- 状态查询正确
- 同步检查正常
- 数据库操作正常
- **cid 验证正常：正常数据同步成功，错误 cid 数据被拒绝**

## 前置依赖
- DataState 状态类
- LocalDB 数据库类
- TableConfig 配置类
- **服务器端 synclog.mAddMany() 支持 cid 验证**

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

### cid 验证测试
- [ ] 正常 cid 数据同步成功
- [ ] 错误 cid 数据被拒绝，synced=-1
- [ ] lasterrinfo 记录错误信息
- 测试用例：`test_full_sync_workflow`, `test_cid_validation_failed`
- 测试文件：`crates/database/tests/test_tb_sync.rs`

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
- 通过 DataManage 访问 LocalDB

### 坏示例
- 直接修改内部状态
- 不通过管理器操作数据库
- 直接使用 LocalDB 绕过审计

## cid 验证流程

### 客户端验证
```
m_add() / m_update()
  → 读取 TableConfig.uidcid 配置
  → uidcid="cid": cid 字段写入公司ID
  → uidcid="uid": cid 字段写入用户ID
```

### 服务器端验证
```
upload_once()
  → 发送 synclog 到服务器 mAddMany()
  → 服务器验证 cid 是否匹配 SID
  → 验证失败：返回 { errors: [{idrow, error}] }
  → 客户端更新 synced=-1, lasterrinfo=错误信息
```

### synclog 表字段
| 字段 | 说明 |
|------|------|
| synced | 0=待同步, 1=已同步, -1=验证失败 |
| lasterrinfo | 错误信息（验证失败时记录） |