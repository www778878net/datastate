# SysSqlData - SQL 统计数据结构

## 概述

SysSqlData 是 SQL 统计数据结构，SQLite 和 MySQL 共用，字段名一致。

## 第一性目的

定义 sys_sql 表的数据结构：
- SQL 执行统计
- 性能分析
- 慢查询追踪

## 核心类型

### SysSqlData

SQL 统计数据：
- `id` - 业务主键
- `cid` - 公司ID
- `apisys` - 系统名
- `apimicro` - 微服务名
- `apiobj` - 对象名（表名）
- `cmdtext` - SQL 语句
- `uname` - 用户名
- `num` - 执行次数
- `dlong` - 总耗时(ms)
- `downlen` - 下载数据量
- `upby` - 上传者
- `cmdtextmd5` - SQL MD5
- `uptime` - 更新时间

## 示例

```rust
use datastate::shared::SysSqlData;

let data = SysSqlData {
    id: "sql_001".to_string(),
    cmdtext: "SELECT * FROM users".to_string(),
    cmdtextmd5: "abc123".to_string(),
    num: 10,
    dlong: 1500,
    ..Default::default()
};
```

## 测试方案

### 主要逻辑测试

#### 测试1：默认值
```
输入：SysSqlData::default()
步骤：检查各字段默认值
预期：所有字符串为空，数字为 0
```

#### 测试2：序列化
```
输入：SysSqlData 实例
步骤：serde_json::to_string()
预期：返回有效 JSON 字符串
```

#### 测试3：反序列化
```
输入：JSON 字符串
步骤：serde_json::from_str()
预期：返回有效的 SysSqlData 实例
```

### 其它测试（边界、异常等）

#### 测试4：Clone
```
输入：SysSqlData 实例
步骤：clone()
预期：返回独立副本
```

#### 测试5：Debug 输出
```
输入：SysSqlData 实例
步骤：format!("{:?}", data)
预期：输出包含所有字段
```
