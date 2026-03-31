# SysWarnData - 警告日志数据结构

## 概述

SysWarnData 是警告日志数据结构，SQLite 和 MySQL 共用，字段名一致。

## 第一性目的

定义 sys_warn 表的数据结构：
- 调试日志记录
- 错误追踪
- 运行时监控

## 核心类型

### SysWarnData

警告日志数据：
- `id` - 业务主键
- `uid` - 用户ID
- `kind` - 日志类型（debug_xxx, err_xxx）
- `apimicro` - 微服务名
- `apiobj` - 对象名
- `content` - 内容
- `upid` - 上传者ID
- `upby` - 上传者
- `uptime` - 更新时间
- `remark` ~ `remark6` - 备用字段

## 核心方法

### 初始化方法
- `new()` - 创建新实例
- `new_id()` - 生成新ID

## 示例

```rust
use datastate::shared::SysWarnData;

let data = SysWarnData {
    id: SysWarnData::new_id(),
    kind: "debug_test".to_string(),
    content: "测试日志内容".to_string(),
    ..Default::default()
};
```

## 测试方案

### 主要逻辑测试

#### 测试1：默认值
```
输入：SysWarnData::default()
步骤：检查各字段默认值
预期：所有字符串为空
```

#### 测试2：创建新实例
```
输入：SysWarnData::new()
步骤：检查实例
预期：返回有效实例，所有字段为默认值
```

#### 测试3：生成新ID
```
输入：SysWarnData::new_id()
步骤：连续生成两个 ID
预期：两个 ID 不同
```

### 其它测试（边界、异常等）

#### 测试4：序列化
```
输入：SysWarnData 实例
步骤：serde_json::to_string()
预期：返回有效 JSON 字符串
```

#### 测试5：反序列化
```
输入：JSON 字符串
步骤：serde_json::from_str()
预期：返回有效的 SysWarnData 实例
```

#### 测试6：Clone
```
输入：SysWarnData 实例
步骤：clone()
预期：返回独立副本
```
