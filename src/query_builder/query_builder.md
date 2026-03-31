# QueryBuilder - SQL 查询构建器

## 第一性目的

支持链式调用构建 SQL 查询，避免 SQL 拼接错误，提供类型安全的查询构建方式。

## 核心功能

- 链式调用构建 SELECT 查询
- 支持 WHERE、GROUP BY、ORDER BY、LIMIT/OFFSET
- 参数化查询，防止 SQL 注入
- 支持复杂查询组合

## 使用方式

```rust
use crate::query_builder::QueryBuilder;
use serde_json::json;

// 简单查询
let mut qb = QueryBuilder::new();
qb.select(&["id", "name"]).from("users");
let sql = qb.build_sql();
// SELECT `id`, `name` FROM `users`

// 带条件查询
let mut qb = QueryBuilder::new();
qb.select_all()
    .from("users")
    .where_clause("status", "=", json!("active"))
    .order_by_desc("created_at")
    .page(0, 10);
let (sql, values) = qb.build();
```

## 完成标准

- 生成的 SQL 语法正确
- 参数值与占位符对应
- 链式调用正常工作

## 前置依赖

- serde_json 库

## 测试方案

### 主要逻辑测试

#### 测试1：基本 SELECT 查询
```
输入：select(&["id", "name"]).from("users")
步骤：qb.build_sql()
预期："SELECT `id`, `name` FROM `users`"
```

#### 测试2：SELECT * 查询
```
输入：select_all().from("users")
步骤：qb.build_sql()
预期："SELECT * FROM `users`"
```

#### 测试3：WHERE 条件查询
```
输入：where_clause("id", "=", json!("123"))
步骤：qb.build()
预期：SQL 包含 WHERE `id` = ?，values 包含 "123"
```

#### 测试4：多条件 WHERE 查询
```
输入：where_clause("status", "=", json!("active")).and_where("age", ">=", json!(18))
步骤：qb.build()
预期：SQL 包含 WHERE ... AND ...，values 包含两个值
```

### 其它测试（边界、异常等）

#### 测试5：WHERE IN 条件
```
输入：where_in("id", &[json!(1), json!(2), json!(3)])
步骤：qb.build_sql()
预期："WHERE `id` IN (?,?,?)"
```

#### 测试6：WHERE LIKE 条件
```
输入：where_like("name", "%john%")
步骤：qb.build()
预期：SQL 包含 WHERE `name` LIKE ?，values 包含 "%john%"
```

#### 测试7：ORDER BY 排序
```
输入：order_by_desc("created_at")
步骤：qb.build_sql()
预期：SQL 包含 "ORDER BY `created_at` DESC"
```

#### 测试8：分页查询
```
输入：page(10, 20)
步骤：qb.build_sql()
预期：SQL 包含 "LIMIT 10, 20"
```

#### 测试9：GROUP BY 分组
```
输入：group_by(&["status"])
步骤：qb.build_sql()
预期：SQL 包含 "GROUP BY `status`"
```

#### 测试10：重置构建器
```
输入：reset()
步骤：构建查询后调用 reset()
预期：重置后 build_sql() 返回 "SELECT *"
```

## 知识库

### 方法列表

| 方法 | 说明 |
|------|------|
| select(&["field1", "field2"]) | 指定 SELECT 字段 |
| select_all() | SELECT * |
| from("table") | 指定表名 |
| where_clause("field", "=", value) | 添加 WHERE 条件 |
| and_where("field", ">=", value) | 添加 AND 条件 |
| where_in("field", &[values]) | WHERE IN 条件 |
| where_like("field", "pattern") | WHERE LIKE 条件 |
| where_null("field") | WHERE IS NULL |
| where_not_null("field") | WHERE IS NOT NULL |
| group_by(&["field"]) | GROUP BY |
| order_by("field", "DESC") | ORDER BY |
| order_by_desc("field") | ORDER BY DESC |
| order_by_asc("field") | ORDER BY ASC |
| limit(n) | LIMIT |
| offset(n) | OFFSET |
| page(offset, limit) | 分页 |
| build_sql() | 构建 SQL 字符串 |
| build_values() | 获取参数值 |
| build() | 构建 (SQL, values) |
| reset() | 重置构建器 |

## 好坏示例

### 好示例
```rust
// 使用参数化查询，防止 SQL 注入
let mut qb = QueryBuilder::new();
qb.where_clause("id", "=", json!(user_input));
```

### 坏示例
```rust
// 错误：直接拼接字符串，有 SQL 注入风险
let sql = format!("SELECT * FROM users WHERE id = {}", user_input);
```
