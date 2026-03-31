# TestTb - 测试表 DataState

## 概述

TestTb 是用于演示权限控制使用方式的测试表 DataState 实现。

## 第一性目的

演示权限控制的完整实现：
- 不同调用方的权限验证
- CRUD 操作的权限检查
- 同步方法（不写 sync_queue）

## 权限设计

### 调用方权限
- `testtb`: 同表调用，全部权限
- `inventory`: 库存服务，可读可写（getone, mlist, m_save）
- `trade`: 交易服务，只读查询（getone, mlist）

## 核心类型

### TestTb

测试表 DataState：
- `db: LocalDB` - 本地数据库
- `audit: DataAudit` - 审计组件
- `state: DataState` - 数据状态

### TestTbRecord

测试表记录：
- `idpk` - 自增主键
- `id` - 业务主键
- `cid` - 公司ID
- `kind` - 类型
- `item` - 项目
- `data` - 数据

## 核心方法

### 创建方法
- `new()` - 创建默认实例
- `with_db_path(db_path)` - 使用指定数据库路径创建
- `get_config()` - 获取表配置

### CRUD 方法（带权限检查）
- `getone(id, caller, summary)` - 查询单条
- `mlist(caller, limit, summary)` - 列表查询
- `m_add(record, caller, summary)` - 插入记录
- `m_save(record, caller, summary)` - 保存记录
- `m_update(id, record, caller, summary)` - 更新记录
- `m_del(id, caller, summary)` - 删除记录

### 同步方法
- `m_sync_save(record)` - 同步保存
- `m_sync_update(id, record)` - 同步更新
- `m_sync_del(id)` - 同步删除

## 测试方案

### 主要逻辑测试

#### 测试1：权限验证 - testtb 调用
```
输入：caller="testtb"
步骤：m_save(), mlist(), getone(), m_del()
预期：全部操作成功
```

#### 测试2：权限验证 - inventory 调用
```
输入：caller="inventory"
步骤：m_save(), mlist(), getone() 成功，m_del() 失败
预期：前三个成功，删除操作返回错误
```

#### 测试3：权限验证 - trade 调用
```
输入：caller="trade"
步骤：mlist(), getone() 成功，m_save(), m_del() 失败
预期：只读操作成功，写操作返回错误
```

### 其它测试（边界、异常等）

#### 测试4：权限验证 - 未知调用方
```
输入：caller="unknown"
步骤：任意操作
预期：返回权限错误
```

#### 测试5：同步保存
```
输入：record
步骤：m_sync_save()
预期：保存成功，不写 sync_queue
```
