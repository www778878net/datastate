# TableSet - 表配置定义

## 第一性目的

定义数据库表的元信息，包括表名、列配置、数据隔离类型等，用于统一管理表结构配置。

## 核心功能

- 表配置定义（表名、列名、数据隔离类型）
- 数据隔离类型支持（公司级 Cid / 用户级 Uid）
- 表配置管理器（注册、获取、JSON 加载）

## 使用方式

```rust
use crate::table_config::{TableSet, TableConfigManager, UidCid};

// 创建表配置
let ts = TableSet::new(
    "user_table",
    vec!["name".to_string(), "email".to_string()],
    UidCid::Uid,
);

// 使用配置管理器
let mut manager = TableConfigManager::new();
manager.register(ts);

let config = manager.get("user_table");
```

## 完成标准

- 表配置正确创建
- 数据隔离类型正确识别
- 配置管理器正常工作

## 前置依赖

- serde 库

## 测试方案

### 主要逻辑测试

#### 测试1：UidCid 枚举
```
输入：UidCid::Cid, UidCid::Uid
步骤：field_name(), from_str()
预期：Cid.field_name() = "cid"，Uid.field_name() = "uid"
```

#### 测试2：创建 TableSet
```
输入：tbname="test_table", cols_imp=["field1", "field2"], uidcid=Cid
步骤：TableSet::new()
预期：tbname="test_table"，cols_imp 包含 field1/field2，isolation_field()="cid"
```

#### 测试3：INSERT 列和占位符
```
输入：columns = ["name", "email"]
步骤：insert_columns(), insert_placeholders(3)
预期：insert_columns = "`name`,`email`"，insert_placeholders = "?,?,?"
```

#### 测试4：配置管理器注册和获取
```
输入：注册 TableSet
步骤：manager.register(), manager.get()
预期：get("test") 返回 Some，get("TEST") 返回 Some（忽略大小写）
```

### 其它测试（边界、异常等）

#### 测试5：从 JSON 加载配置
```
输入：JSON 配置字符串
步骤：manager.load_from_json(json)
预期：配置正确加载，可通过 get() 获取
```

#### 测试6：系统列
```
输入：TableSet::system_cols()
步骤：检查返回值
预期：包含 ["id", "idpk", "upby", "uptime"]
```

#### 测试7：不存在的表配置
```
输入：manager.get("notexist")
步骤：获取不存在的配置
预期：返回 None
```

## 知识库

### 数据结构

```rust
pub enum UidCid {
    Cid,  // 公司级隔离
    Uid,  // 用户级隔离
}

pub struct TableSet {
    pub tbname: String,      // 表名（小写）
    pub cols: Vec<String>,   // 所有列
    pub cols_imp: Vec<String>, // 重要业务列
    pub uidcid: UidCid,      // 数据隔离类型
}
```

### 方法列表

| 方法 | 说明 |
|------|------|
| TableSet::new(tbname, cols_imp, uidcid) | 创建新配置 |
| isolation_field() | 获取隔离字段名 ("cid" 或 "uid") |
| has_col(col) | 检查列是否存在 |
| system_cols() | 获取系统列列表 |
| insert_columns(colp) | 获取 INSERT 列部分 |
| insert_placeholders(count) | 获取占位符字符串 |

## 好坏示例

### 好示例
```rust
// 使用 TableSet 管理表配置
let ts = TableSet::new("user", cols, UidCid::Uid);
let field = ts.isolation_field(); // "uid"
```

### 坏示例
```rust
// 错误：硬编码表名和列名
let sql = format!("INSERT INTO {} ({}) VALUES", "user", "name,email");
```
