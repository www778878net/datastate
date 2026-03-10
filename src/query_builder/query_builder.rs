//! QueryBuilder - SQL 查询构建器
//!
//! 支持链式调用构建 SQL 查询

use serde_json::Value;

/// SQL 查询构建器
#[derive(Debug, Clone)]
pub struct QueryBuilder {
    /// SELECT 字段
    select_fields: Vec<String>,
    /// FROM 表名
    from_table: Option<String>,
    /// WHERE 条件
    where_conditions: Vec<String>,
    /// 参数值
    values: Vec<Value>,
    /// GROUP BY 字段
    group_by_fields: Vec<String>,
    /// ORDER BY 子句
    order_by: Option<String>,
    /// LIMIT
    limit: Option<usize>,
    /// OFFSET
    offset: Option<usize>,
}

impl QueryBuilder {
    /// 创建新的查询构建器
    pub fn new() -> Self {
        Self::default()
    }

    /// 指定 SELECT 字段
    pub fn select(&mut self, fields: &[&str]) -> &mut Self {
        self.select_fields = fields.iter().map(|f| f.to_string()).collect();
        self
    }

    /// 添加原始 SELECT 字段
    pub fn add_raw_select(&mut self, raw_fields: &[&str]) -> &mut Self {
        for f in raw_fields {
            self.select_fields.push(f.to_string());
        }
        self
    }

    /// SELECT *
    pub fn select_all(&mut self) -> &mut Self {
        self.select_fields = vec!["*".to_string()];
        self
    }

    /// 指定 FROM 表名
    pub fn from(&mut self, table: &str) -> &mut Self {
        self.from_table = Some(table.to_string());
        self
    }

    /// 添加 WHERE 条件
    pub fn where_clause(&mut self, field: &str, operator: &str, value: Value) -> &mut Self {
        self.where_conditions.push(format!("`{}` {} ?", field, operator));
        self.values.push(value);
        self
    }

    /// 添加 AND WHERE 条件
    pub fn and_where(&mut self, field: &str, operator: &str, value: Value) -> &mut Self {
        self.where_clause(field, operator, value)
    }

    /// WHERE IN 条件
    pub fn where_in(&mut self, field: &str, values: &[Value]) -> &mut Self {
        let placeholders: Vec<String> = values.iter().map(|_| "?".to_string()).collect();
        self.where_conditions.push(format!("`{}` IN ({})", field, placeholders.join(",")));
        self.values.extend(values.iter().cloned());
        self
    }

    /// WHERE LIKE 条件
    pub fn where_like(&mut self, field: &str, pattern: &str) -> &mut Self {
        self.where_conditions.push(format!("`{}` LIKE ?", field));
        self.values.push(Value::String(pattern.to_string()));
        self
    }

    /// WHERE IS NULL
    pub fn where_null(&mut self, field: &str) -> &mut Self {
        self.where_conditions.push(format!("`{}` IS NULL", field));
        self
    }

    /// WHERE IS NOT NULL
    pub fn where_not_null(&mut self, field: &str) -> &mut Self {
        self.where_conditions.push(format!("`{}` IS NOT NULL", field));
        self
    }

    /// GROUP BY
    pub fn group_by(&mut self, fields: &[&str]) -> &mut Self {
        self.group_by_fields = fields.iter().map(|f| f.to_string()).collect();
        self
    }

    /// ORDER BY
    pub fn order_by(&mut self, field: &str, direction: &str) -> &mut Self {
        self.order_by = Some(format!("`{}` {}", field, direction));
        self
    }

    /// ORDER BY DESC
    pub fn order_by_desc(&mut self, field: &str) -> &mut Self {
        self.order_by(field, "DESC")
    }

    /// ORDER BY ASC
    pub fn order_by_asc(&mut self, field: &str) -> &mut Self {
        self.order_by(field, "ASC")
    }

    /// LIMIT
    pub fn limit(&mut self, count: usize) -> &mut Self {
        self.limit = Some(count);
        self
    }

    /// OFFSET
    pub fn offset(&mut self, offset: usize) -> &mut Self {
        self.offset = Some(offset);
        self
    }

    /// 分页 (LIMIT offset, count)
    pub fn page(&mut self, offset: usize, count: usize) -> &mut Self {
        self.offset = Some(offset);
        self.limit = Some(count);
        self
    }

    /// 构建 SQL 字符串
    pub fn build_sql(&self) -> String {
        let mut parts = Vec::new();

        // SELECT
        if self.select_fields.is_empty() {
            parts.push("SELECT *".to_string());
        } else {
            let fields: Vec<String> = self.select_fields.iter()
                .map(|f| {
                    if f == "*" || f.starts_with("`") {
                        f.clone()
                    } else {
                        format!("`{}`", f)
                    }
                })
                .collect();
            parts.push(format!("SELECT {}", fields.join(", ")));
        }

        // FROM
        if let Some(ref table) = self.from_table {
            parts.push(format!("FROM `{}`", table));
        }

        // WHERE
        if !self.where_conditions.is_empty() {
            parts.push(format!("WHERE {}", self.where_conditions.join(" AND ")));
        }

        // GROUP BY
        if !self.group_by_fields.is_empty() {
            let fields: Vec<String> = self.group_by_fields.iter()
                .map(|f| format!("`{}`", f))
                .collect();
            parts.push(format!("GROUP BY {}", fields.join(", ")));
        }

        // ORDER BY
        if let Some(ref order) = self.order_by {
            parts.push(format!("ORDER BY {}", order));
        }

        // LIMIT / OFFSET
        if let Some(limit) = self.limit {
            if let Some(offset) = self.offset {
                parts.push(format!("LIMIT {}, {}", offset, limit));
            } else {
                parts.push(format!("LIMIT {}", limit));
            }
        }

        parts.join(" ")
    }

    /// 获取参数值
    pub fn build_values(&self) -> &[Value] {
        &self.values
    }

    /// 构建 SQL 和参数
    pub fn build(&self) -> (String, Vec<Value>) {
        (self.build_sql(), self.values.clone())
    }

    /// 重置构建器
    pub fn reset(&mut self) -> &mut Self {
        self.select_fields.clear();
        self.from_table = None;
        self.where_conditions.clear();
        self.values.clear();
        self.group_by_fields.clear();
        self.order_by = None;
        self.limit = None;
        self.offset = None;
        self
    }
}

impl Default for QueryBuilder {
    fn default() -> Self {
        Self {
            select_fields: Vec::new(),
            from_table: None,
            where_conditions: Vec::new(),
            values: Vec::new(),
            group_by_fields: Vec::new(),
            order_by: None,
            limit: None,
            offset: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_select() {
        let mut qb = QueryBuilder::new();
        qb.select(&["id", "name"]).from("users");
        let sql = qb.build_sql();
        assert_eq!(sql, "SELECT `id`, `name` FROM `users`");
    }

    #[test]
    fn test_select_all() {
        let mut qb = QueryBuilder::new();
        qb.select_all().from("users");
        let sql = qb.build_sql();
        assert_eq!(sql, "SELECT * FROM `users`");
    }

    #[test]
    fn test_where() {
        let mut qb = QueryBuilder::new();
        qb.select(&["*"])
            .from("users")
            .where_clause("id", "=", json!("123"));

        let (sql, values) = qb.build();
        assert_eq!(sql, "SELECT * FROM `users` WHERE `id` = ?");
        assert_eq!(values.len(), 1);
        assert_eq!(values[0], json!("123"));
    }

    #[test]
    fn test_multiple_where() {
        let mut qb = QueryBuilder::new();
        qb.select(&["*"])
            .from("users")
            .where_clause("status", "=", json!("active"))
            .and_where("age", ">=", json!(18));

        let (sql, values) = qb.build();
        assert_eq!(sql, "SELECT * FROM `users` WHERE `status` = ? AND `age` >= ?");
        assert_eq!(values.len(), 2);
    }

    #[test]
    fn test_where_in() {
        let mut qb = QueryBuilder::new();
        qb.select(&["*"])
            .from("users")
            .where_in("id", &[json!(1), json!(2), json!(3)]);

        let sql = qb.build_sql();
        assert_eq!(sql, "SELECT * FROM `users` WHERE `id` IN (?,?,?)");
    }

    #[test]
    fn test_where_like() {
        let mut qb = QueryBuilder::new();
        qb.select(&["*"])
            .from("users")
            .where_like("name", "%john%");

        let (sql, values) = qb.build();
        assert_eq!(sql, "SELECT * FROM `users` WHERE `name` LIKE ?");
        assert_eq!(values[0], json!("%john%"));
    }

    #[test]
    fn test_order_by() {
        let mut qb = QueryBuilder::new();
        qb.select(&["*"])
            .from("users")
            .order_by_desc("created_at");

        let sql = qb.build_sql();
        assert_eq!(sql, "SELECT * FROM `users` ORDER BY `created_at` DESC");
    }

    #[test]
    fn test_limit_offset() {
        let mut qb = QueryBuilder::new();
        qb.select(&["*"])
            .from("users")
            .page(10, 20);

        let sql = qb.build_sql();
        assert_eq!(sql, "SELECT * FROM `users` LIMIT 10, 20");
    }

    #[test]
    fn test_group_by() {
        let mut qb = QueryBuilder::new();
        qb.select(&["status", "COUNT(*)"])
            .from("users")
            .group_by(&["status"]);

        let sql = qb.build_sql();
        assert_eq!(sql, "SELECT `status`, `COUNT(*)` FROM `users` GROUP BY `status`");
    }

    #[test]
    fn test_complex_query() {
        let mut qb = QueryBuilder::new();
        qb.select(&["id", "name", "email"])
            .from("users")
            .where_clause("status", "=", json!("active"))
            .and_where("age", ">=", json!(18))
            .order_by_desc("created_at")
            .page(0, 10);

        let (sql, values) = qb.build();
        assert_eq!(
            sql,
            "SELECT `id`, `name`, `email` FROM `users` WHERE `status` = ? AND `age` >= ? ORDER BY `created_at` DESC LIMIT 0, 10"
        );
        assert_eq!(values.len(), 2);
    }

    #[test]
    fn test_reset() {
        let mut qb = QueryBuilder::new();
        qb.select(&["*"]).from("users").where_clause("id", "=", json!(1));

        qb.reset();

        let sql = qb.build_sql();
        assert_eq!(sql, "SELECT *");
    }
}
