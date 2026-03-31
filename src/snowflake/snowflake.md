# snowflake - 雪花算法 ID 生成器

## 第一性目的

生成分布式唯一ID，64位整数，字符串形式存储。
结构：时间戳(41位) + 机器ID(10位) + 序列号(12位)。

## 核心功能

- `next_id()` - 生成下一个雪花ID（i64）
- `next_id_string()` - 生成下一个雪花ID（String）
- `init_worker_id(worker_id)` - 手动设置机器ID（范围 0-1023）
- `get_worker_id()` - 获取当前 worker_id

## 使用方式

```rust
use crate::snowflake::{next_id, next_id_string, init_worker_id, get_worker_id};

// 生成整数 ID
let id = next_id();

// 生成字符串 ID
let id_str = next_id_string();

// 手动设置 worker_id（可选）
init_worker_id(100);

// 获取当前 worker_id
let wid = get_worker_id();
```

## 完成标准

- 生成的 ID 唯一且递增
- worker_id 自动生成或手动设置
- 支持高并发场景

## 前置依赖

- uuid 库（用于自动生成 worker_id）

## 测试方案

### 主要逻辑测试

#### 测试1：生成递增ID
```
输入：连续调用 next_id() 两次
步骤：
  let id1 = next_id();
  let id2 = next_id();
预期：id2 > id1
```

#### 测试2：生成字符串ID
```
输入：调用 next_id_string()
步骤：
  let id = next_id_string();
预期：!id.is_empty() && id.len() >= 18
```

#### 测试3：自动生成 worker_id
```
输入：首次调用 next_id()
步骤：
  let id = next_id();
  let worker_id = get_worker_id();
预期：worker_id <= 1023，id > 0
```

### 其它测试（边界、异常等）

#### 测试4：worker_id 范围验证
```
输入：手动设置 worker_id 为 0 和 1023
步骤：
  init_worker_id(0);
  assert_eq!(get_worker_id(), 0);
  init_worker_id(1023);
  assert_eq!(get_worker_id(), 1023);
预期：边界值正常设置
```

#### 测试5：worker_id 超范围 panic
```
输入：init_worker_id(1024)
步骤：
  init_worker_id(1024);
预期：应该 panic（#[should_panic]）
```

#### 测试6：连续生成 ID 唯一性
```
输入：连续生成 1000 个 ID
步骤：
  let ids: Vec<i64> = (0..1000).map(|_| next_id()).collect();
  let unique_count = ids.iter().collect::<std::collections::HashSet<_>>().len();
预期：unique_count == 1000（所有 ID 唯一）
```

## 知识库

### ID 结构
- 时间戳（41位）：从 2024-01-01 00:00:00 UTC 开始计算
- 机器ID（10位）：支持 1024 个节点
- 序列号（12位）：同一毫秒内最多 4096 个 ID

### 常量定义
```rust
const EPOCH: i64 = 1704067200000; // 2024-01-01 00:00:00 UTC
const MAX_WORKER_ID: i64 = 1023;  // 最大机器ID
const MAX_SEQUENCE: i64 = 4095;   // 最大序列号
```

## 好坏示例

### 好示例
```rust
// 生成 ID 并存储为字符串
let id = next_id_string();
record.set("id", id);

// 分布式环境下设置不同的 worker_id
init_worker_id(server_id % 1024);
```

### 坏示例
```rust
// 错误：假设 ID 是递增的且可以预测
let id = next_id();
let next_id = id + 1; // 错误：下一个 ID 不一定是 +1

// 错误：worker_id 超范围
init_worker_id(2000); // 会 panic
```
