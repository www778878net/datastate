# DataState 数据状态机文档

## 管理员指示
- 本文档由 CLAUDE 自动生成，描述 DataState 模块的设计和实现

## 第一性目的
- 单表状态机，管理数据库表的同步状态
- 控制下载/上传时机，避免重复请求和并发冲突
- 记录同步日志到 sync_queue 队列

## 完成标准
- 状态正确流转：IDLE <-> WORKING -> ERROR
- need_download() 在正确时机返回 true
- need_upload() 在有数据待同步时返回 true
- save_to_local_db() 正确插入/更新/跳过记录
- sync_queue 操作成功记录和查询待同步项
- 首次下载：支持自定义条件，默认全表idpk顺排分页下载，每次获取指定条数
- 后续上传：每5分钟上传本表全部synclog，调用服务器批量处理接口
- 后续下载：每5分钟下载非本地worker的synclog，本地执行SQL
- 任务式触发：local_count < min_pending 时触发下载

## 前置依赖
- BaseState 提供三态基础能力
- LocalDB 提供数据库操作
- TableConfig 提供配置参数

## 业务逻辑

### 状态定义
```
IDLE = 0    空闲，可接受新任务
WORKING = 1 工作中，拒绝新任务
ERROR = 2   错误，需恢复
```

### 字段说明
- `apiurl`: 服务器 API 地址
- `download_interval`: 下载间隔(秒)，默认 300
- `upload_interval`: 上传间隔(秒)，默认 300
- `init_getnumber`: 首次下载数量，默认 50
- `getnumber`: 每次下载数量，默认 50
- `min_pending`: 最小待处理数量，用于任务表触发
- `last_download`: 上次下载时间戳
- `last_upload`: 上次上传时间戳

### 下载时机判断
```rust
pub fn need_download(&self, current_time: f64, local_count: i32) -> bool {
    // 1. 必须是空闲状态
    if !self.is_idle() { return false; }

    // 2. 时间到了 或 任务数量不足
    let time_ok = current_time - last_download >= download_interval;
    let pending_ok = min_pending > 0 && local_count < min_pending;

    time_ok || pending_ok
}
```

### 上传时机判断
```rust
pub fn need_upload(&self, current_time: f64, pending_count: i32) -> bool {
    // 1. 必须是空闲状态
    if !self.is_idle() { return false; }

    // 2. 时间到了 且 有待同步数据
    let time_ok = current_time - last_upload >= upload_interval;
    time_ok && pending_count > 0
}
```

### 数据保存逻辑
1. 遍历记录，检查 `id` 字段
2. 查询本地是否存在该 `id`
3. 存在则比较 `uptime`，新则更新
4. 不存在则插入
5. 返回

### sync_queue 操作
- `add_to_sync_queue()`: 记录本地变更
- `get_pending_count()`: 获取待同步数量
- `get_pending_items()`: 获取待同步列表
- `mark_synced()`: 标记已同步

### 首次下载逻辑
1. 默认调用get方法，支持自定义条件
2. 默认获取全表，按idpk顺排
3. 分页下载，每次获取 `init_getnumber` 条
4. 获取到数据后保存到本地表
5. 获取满10条记录后结束首次下载
6. 更新 `last_download` 时间戳

### 后续上传逻辑
1. 默认每5分钟上传一次
2. 收集本表全部 `synclog` 记录
3. 调用服务器批量处理接口 `batch_process`
4. 服务器执行批量SQL
5. 清理已同步的 `synclog`
6. 更新 `last_upload` 时间戳

### 后续下载逻辑
1. 默认每5分钟下载一次
2. 请求服务器获取非本地worker的 `synclog`
3. 本地执行SQL语句
4. 清理已处理的记录
5. 更新 `last_download` 时间戳

### 任务式触发逻辑
1. 检测 `local_count < min_pending`
2. 触发下载补充任务数量
3. 与定时触发共存，任一满足即触发

## 测试方案

### 首次下载逻辑测试
- [ ] 默认get方法支持自定义条件查询
- [ ] 默认全表idpk顺排下载
- [ ] 分页下载正确（每次获取指定条数）
- [ ] 获取10条数据后正常结束
- [ ] 首次下载 last_download == 0 时触发
- [ ] init_getnumber 和 getnumber 参数生效

### 后续更新逻辑测试
- [ ] 默认5分钟上传一次
- [ ] 上传时提交本表全部synclog
- [ ] 上传后调用服务器批量处理接口
- [ ] 默认5分钟下载一次
- [ ] 下载时只获取非本地worker的synclog
- [ ] 下载后本地执行SQL正确
- [ ] 批量SQL执行成功

### 任务式触发测试
- [ ] local_count < min_pending 时触发下载
- [ ] 任务数量充足时不触发额外下载
- [ ] 任务式触发与时间触发共存时正确处理

### 主要逻辑测试
- [ ] need_download 空闲且时间到返回 true
- [ ] need_download 工作中返回 false
- [ ] need_upload 空闲且有待同步数据返回 true
- [ ] save_to_local_db 新记录插入成功
- [ ] save_to_local_db 已存在且 uptime 更新则更新
- [ ] save_to_local_db 已存在且 uptime 更旧则跳过
- [ ] add_to_sync_queue 成功写入 sync_queue

### 边界测试
- [ ] uptime 为空字符串时的比较
- [ ] sync_queue 空列表 mark_synced 成功
- [ ] 分页下载最后一页不满时正确处理
- [ ] synclog 数量为0时的上传处理

### 异常测试
- [ ] 记录无 id 字段跳过
- [ ] 数据库操作失败返回错误
- [ ] 服务器批量接口失败时的处理
- [ ] 网络超时重试机制

## 知识库

### 创建 DataState
```rust
let config = TableConfig::new("my_table")
    .with_apiurl("http://example.com/api/my_table/get");
let state = DataState::from_config(&config);
```

### 判断同步时机
```rust
let current_time = DataState::current_time();
let local_count = db.count("my_table")?;

if state.need_download(current_time, local_count) {
    // 执行下载
}

let pending = state.get_pending_count(&db, "my_table");
if state.need_upload(current_time, pending) {
    // 执行上传
}
```

### 记录变更
```rust
state.add_to_sync_queue(
    &db,
    "my_table",
    "record_id",
    "update",
    &data,
    "worker_001"
)?;
```

## 好坏示例

### 好示例
- 通过 `from_config()` 创建，配置完整
- 使用 `need_download()` 判断时机，避免频繁请求
- 检查 `uptime` 避免覆盖新数据
- 工作前 `set_working()`，完成后 `set_idle()`

### 坏示例
- 直接修改 `last_download` 绕过时间检查
- 不检查状态直接执行下载，导致并发冲突
- 忽略 `uptime` 直接覆盖，丢失更新
- 错误后不设置 `set_error()`，状态混乱

## 审计功能（二级权限模型）

### 目的
- 记录能力调用日志，控制访问权限
- 审计开关开启时检查权限，关闭时跳过权限检查但仍记录日志

### 审计开关
```bash
# 开启审计（检查权限）
export DATASTATE_AUDIT=1
```

### 权限检查流程（二级权限）
```
1. 本微服务内部调用 → 自动放行（caller与micro_name匹配）
2. 微服务层权限检查 → 有权限则放行
3. 能力层权限检查 → 有权限则放行
4. 拒绝访问
```

### 权限表设计

#### data_micro_perm - 微服务层权限（一级权限）
- 粗粒度控制：一次配置覆盖微服务内所有能力
- 字段说明：
  - micro_name: 被访问的微服务名
  - allow_caller: 允许的调用微服务名

#### data_ability_perm - 能力层权限（二级权限）
- 细粒度控制：精确到某个能力的访问权限
- 字段说明：
  - micro_name: 能力类名/微服务名
  - ability: 函数名
  - caller: 允许的调用者，多个用逗号分隔，空表示不限制

#### data_ability_log - 能力调用日志
- 字段说明：
  - ability_name: 能力名称（micro_name:ability格式）
  - caller: 调用者
  - action: 操作说明
  - input_params: JSON输入参数

#### data_ability_daily - 每日统计
- 字段说明：
  - ability_name: 能力名称
  - caller: 调用者
  - stat_date: 日期（YYYY-MM-DD）

### 使用示例

#### 注册微服务层权限（一级权限）
```rust
use database::{LocalDB, register_micro_perm_simple};

// 一次配置，允许 order 微服务访问 account 微服务的所有能力
register_micro_perm_simple(
    &db,
    "account",   // 被访问的微服务名
    "order",     // 允许的调用微服务名
    "订单服务可访问帐号服务所有能力",
)?;
```

#### 注册能力层权限（二级权限）
```rust
use database::{LocalDB, register_ability_simple};

// 精确控制：只允许 order 微服务访问 account 的 getone 能力
register_ability_simple(
    &db,
    "account",   // 能力类名
    "getone",    // 函数名
    "获取单个帐号",
    "order",     // 允许的调用者，空表示不限制
)?;
```

#### 检查权限（自动走二级检查流程）
```rust
use database::{check_ability_permission, DataAudit};

// 创建DataAudit实例并开启审计
let audit = DataAudit::with_audit("account");

// 使用实例的audit_enabled属性
if audit.audit_enabled {
    // 内部调用自动放行
    check_ability_permission(&db, "account", "getone", "AccountService", true)?; // OK

    // 微服务层有权限
    check_ability_permission(&db, "account", "getone", "order", true)?; // OK（如果注册了微服务层权限）

    // 能力层有权限
    check_ability_permission(&db, "account", "getone", "payment", true)?; // OK（如果注册了能力层权限）

    // 无权限
    check_ability_permission(&db, "account", "getone", "unknown", true)?; // Err
}
```

#### 记录调用日志
```rust
use database::log_ability_call;

let input_params = serde_json::json!({
    "table": "account",
    "id": 123,
});

log_ability_call(
    &db,
    "account",       // 能力类名
    "getone",        // 函数名
    "order",         // 调用者
    "获取帐号",      // 操作说明
    &input_params,   // 输入参数
)?;
```

### 测试验证
- 测试文件：`crates/datastate/tests/test_audit_permissions.rs`
- 运行测试：`cargo test -p database test_tb_permission`
- 审计开启测试：`DATASTATE_AUDIT=1 cargo test -p database test_tb_permission`

### 权限控制行为
| 审计状态 | 内部调用 | 微服务层权限 | 能力层权限 | 结果 |
|---------|---------|------------|-----------|------|
| 关闭 | - | - | - | 允许 |
| 开启 | 是 | - | - | 允许（自动放行） |
| 开启 | 否 | 有 | - | 允许 |
| 开启 | 否 | 无 | 有 | 允许 |
| 开启 | 否 | 无 | 无 | 拒绝 |
