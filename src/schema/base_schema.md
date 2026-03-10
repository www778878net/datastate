# BaseSchema - 基础表结构定义

## 第一性目的

**解决什么问题**：定义所有数据表的公共字段，支持数据隔离模式。

**核心价值**：
- 统一字段：所有业务表包含相同的系统字段
- 数据隔离：支持公司级(Cid)和用户级(Uid)隔离
- 自动生成：自动生成 UUID 和时间戳

---

## 完成标准（可验证）

### ✅ 创建 BaseSchema
```rust
let schema = BaseSchema::new();
assert!(!schema.id.is_empty());
assert_eq!(schema.idpk, 0);
```

### ✅ 创建 CidSchema（公司级隔离）
```rust
let schema = CidSchema::new("company-123");
assert_eq!(schema.cid, "company-123");
```

### ✅ 创建 UidSchema（用户级隔离）
```rust
let schema = UidSchema::new("user-456");
assert_eq!(schema.uid, "user-456");
```

---

## 前置依赖

| 依赖 | 说明 |
|------|------|
| uuid | UUID 生成 |
| chrono | 时间处理 |
| serde | 序列化 |

---

## 测试方案

### 主要逻辑测试

| 测试 | 输入 | 预期输出 | 验证方法 |
|------|------|----------|----------|
| BaseSchema 创建 | new() | 有 UUID | assert!(!schema.id.is_empty()) |
| CidSchema 创建 | new("cid") | 有 cid 字段 | assert_eq!(schema.cid, "cid") |
| UidSchema 创建 | new("uid") | 有 uid 字段 | assert_eq!(schema.uid, "uid") |
| 系统字段 | system_fields() | 包含 id, idpk | assert!(fields.contains(&"id")) |

---

## 知识库

### 核心概念

| 概念 | 说明 |
|------|------|
| BaseSchema | 基础表结构 |
| CidSchema | 公司级隔离表结构 |
| UidSchema | 用户级隔离表结构 |
| SchemaType | Schema 类型枚举 |

### 字段说明

| 字段 | 类型 | 说明 |
|------|------|------|
| id | String | 业务主键 (UUID) |
| idpk | i64 | 自增主键 |
| upby | String | 操作人 |
| uptime | String | 操作时间 |

### 方法签名

```rust
// BaseSchema
pub fn new() -> Self
pub fn new_id() -> String
pub fn system_fields() -> &'static [&'static str]

// CidSchema
pub fn new(cid: &str) -> Self
pub fn isolation_field() -> &'static str

// UidSchema
pub fn new(uid: &str) -> Self
pub fn isolation_field() -> &'static str

// SchemaType
pub fn isolation_field(&self) -> &'static str
```

---

## 好坏示例

### ✅ 好示例：为多租户系统使用 CidSchema

```rust
// 公司级数据隔离
let order = CidSchema::new(&company_id);
// 自动包含 cid 字段，查询时自动过滤
```

### ✅ 好示例：为用户数据使用 UidSchema

```rust
// 用户级数据隔离
let user_data = UidSchema::new(&user_id);
// 自动包含 uid 字段
```

### ❌ 坏示例：手动创建 UUID

```rust
// 错误：应该使用 new() 自动生成
let schema = BaseSchema {
    id: "manual-uuid".to_string(),
    ..Default::default()
};
// 正确做法：
let schema = BaseSchema::new();
```

---

## 文件位置

- 类实现: `crates/database/src/schema/base_schema.rs`
- 技术文档: `crates/database/src/schema/base_schema.md`
- 模块入口: `crates/database/src/schema/mod.rs`
