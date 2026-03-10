# SyncConfig 同步配置模块文档

## 管理员指示
- 本文档描述 SyncConfig 模块的设计和实现
- 禁止把代码写在 mod.rs, lib.rs，应该在子模块中实现

## 第一性目的
- 定义表同步策略
- 配置表的下载/上传参数
- 生成建表 SQL 和索引 SQL

## 完成标准
- 同步策略正确枚举
- 表配置完整可序列化
- 建表 SQL 生成正确
- 索引 SQL 生成正确

## 前置依赖
- serde 库

## 业务逻辑

### 同步策略
- REALTIME: 实时同步（热数据）
- BATCH: 批量同步（冷数据）
- LOCAL_ONLY: 仅本地存储

### 表配置字段
- `name`: 表名
- `apiurl`: API 地址
- `download_interval`: 下载间隔(秒)，默认 300
- `upload_interval`: 上传间隔(秒)，默认 300
- `init_getnumber`: 初始化下载数量，默认 1000
- `getnumber`: 每次下载数量，默认 1000
- `columns`: 列定义
- `indexes`: 索引定义

### 建表规则
- 所有表必须和 logsvc 服务器字段名称和类型一致
- json 可换 TEXT
- 必须有默认值 NOT NULL

## 测试方案

### 配置创建测试
- [ ] 创建默认配置
- [ ] 链式配置方法正确

### SQL 生成测试
- [ ] 建表 SQL 格式正确
- [ ] 简单索引 SQL 正确
- [ ] 复合索引 SQL 正确

## 知识库

### 创建表配置
```rust
let config = TableConfig::new("my_table")
    .with_apiurl("http://example.com/api/my_table/get")
    .with_download_interval(60);
```

### 生成建表 SQL
```rust
let mut columns = HashMap::new();
columns.insert("id".to_string(), "TEXT NOT NULL PRIMARY KEY".to_string());
columns.insert("name".to_string(), "TEXT NOT NULL DEFAULT ''".to_string());

let config = TableConfig::new("my_table")
    .with_columns(columns);

let sql = config.get_create_sql();
```

## 好坏示例

### 好示例
- 使用链式方法配置
- 使用 system_columns() 获取系统字段

### 坏示例
- 直接修改字段值
- 手动拼接 SQL