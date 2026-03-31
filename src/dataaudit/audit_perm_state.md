# audit_perm_state

## 概述

AuditPermDataState - 权限表数据状态机，管理哪些调用方可以访问哪些表的哪些方法。

## 第一性目的

权限管理的核心组件：
- 注册能力权限
- 验证调用权限
- 支持通配符权限（ability=*）

## 核心类型

### AuditPermDataState

权限表数据状态机：
- `base: BaseState` - 基础状态
- `datasync: DataSync` - 同步组件

### AuditPermRecord

权限记录：
- `tablename` - 表名
- `ability` - 方法名（支持 * 通配符）
- `caller` - 允许调用的微服务名
- `description` - 功能说明

## 核心方法

### 初始化方法
- `new()` - 创建实例
- `from_config(config)` - 从配置创建
- `init_table()` - 初始化表

### 权限管理方法
- `register_ability(tablename, ability, caller, description)` - 注册权限
- `check_permission(tablename, ability, caller, audit_enabled)` - 验证权限
- `get_permission(tablename, ability)` - 获取权限记录
- `list_permissions()` - 列出所有权限

## 权限检查流程

1. 审计关闭时 → 直接放行
2. 本表内部调用 → 自动放行
3. 能力层权限检查（支持 ability=*） → 有权限则放行
4. 拒绝

## 示例

```rust
use datastate::dataaudit::AuditPermDataState;

let state = AuditPermDataState::new();
state.init_table()?;

// 注册权限
state.register_ability("testtb", "getone", "Inventory", "查询详情")?;

// 验证权限
let result = state.check_permission("testtb", "getone", "Inventory", true);
assert!(result.is_ok());
```

## 测试方案

### 主要逻辑测试

#### 测试1：创建实例
```
输入：AuditPermDataState::new()
步骤：检查 base.name 和 datasync.table_name
预期：都为 "datastate_audit"
```

#### 测试2：SQL 有效性
```
输入：AUDIT_PERM_CREATE_SQL
步骤：验证 SQL 语法
预期：包含 CREATE TABLE、tablename、ability、caller、UNIQUE
```

### 其它测试（边界、异常等）

#### 测试3：权限注册
```
输入：tablename, ability, caller, description
步骤：register_ability() -> get_permission()
预期：权限记录存在且字段正确
```

#### 测试4：权限验证 - 有权限
```
输入：已注册的权限
步骤：check_permission()
预期：返回 Ok(true)
```

#### 测试5：权限验证 - 无权限
```
输入：未注册的 caller
步骤：check_permission()
预期：返回错误
```

#### 测试6：通配符权限
```
输入：ability="*" 注册
步骤：check_permission() 任意 ability
预期：返回 Ok(true)
```

#### 测试7：本表内部调用
```
输入：caller == tablename
步骤：check_permission()
预期：自动放行（不检查权限表）
```
