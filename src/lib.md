# datastate - 数据状态库

## 概述

datastate 是数据状态管理库，提供本地优先的数据存储和同步能力。

## 第一性目的

提供统一的数据状态管理：
- DataState - 数据状态管理
- DataSync - 数据同步
- DataAudit - 数据审计
- LocalDB - 本地数据库
- Mysql78 - MySQL 数据库

## 主要模块

| 模块 | 说明 |
|------|------|
| datastate | 数据状态管理 |
| data_sync | 数据同步组件 |
| dataaudit | 数据审计组件 |
| localdb | 本地数据库封装 |
| mysql78 | MySQL 数据库封装 |
| sqlite78 | SQLite 数据库封装 |
| workflow | 工作流存储层 |
| config | 配置管理 |
| snowflake | 雪花ID生成器 |

## 测试方案

### 主要逻辑测试

#### 测试1：库导出
```
输入：use datastate::*;
步骤：验证各公共类型可访问
预期：DataState、LocalDB、Mysql78 等类型可用
```

### 其它测试（边界、异常等）

#### 测试2：模块功能测试
```
输入：各模块测试用例
步骤：参见各模块独立测试方案
预期：各模块测试通过
```

## 使用方式

```rust
use datastate::{DataState, LocalDB, Mysql78};

// 使用本地数据库
let db = LocalDB::default_instance()?;

// 创建数据状态
let state = DataState::with_db("my_table", db);
```
