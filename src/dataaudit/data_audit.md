# data_audit.rs - DataAudit 数据层审计组件

## 管理员指示
禁止修改权限检查逻辑核心流程（权限验证→日志记录→执行），禁止绕过宏或 delegate 方法直接调用业务逻辑，caller 必须自动获取，禁止手动传入。

## 第一性目的
DataAudit 是数据层审计组件，职责包括：权限表初始化、能力权限注册、权限检查、审计日志记录。解决数据操作需要统一权限控制和审计追踪的问题。

## 完成标准
1. init_tables 成功创建 data_ability_perm、data_ability_log、data_ability_daily 三张表
2. register_ability 成功注册能力权限到数据库
3. check_permission 在审计开启时正确验证 caller 权限
4. do_action/do_action_with_input 正确执行权限检查→日志记录→业务逻辑三步骤
5. with_audit 宏自动获取 caller，无法伪造

## 前置依赖
- LocalDB 数据库连接可用
- std::any::type_name::<T>() 可用
- serde_json 用于输入参数序列化

## 测试方案
- **主要逻辑**：调用 init_tables 初始化三张表，调用 register_ability 注册能力，调用 check_permission 验证权限，调用 get_ability_logs 查询审计日志
- **边界**：审计关闭时跳过权限检查但仍记录日志，未注册能力返回错误

## 业务逻辑
DataAudit 提供数据层统一的权限审计能力。流程：1）init_tables 在应用启动时创建权限表 2）register_ability 注册微服务能力及允许的调用者 3）do_action 调用时先检查权限再记录日志最后执行闭包 4）审计日志记录每次调用 caller、ability、input_params。权限表字段：micro_name（能力类名）+ ability（函数名）+ caller（允许调用者，逗号分隔）。

## 知识库

### 表结构
```sql
-- 能力权限注册表
CREATE TABLE data_ability_perm (
    idpk INTEGER PRIMARY KEY,
    id TEXT NOT NULL,
    micro_name TEXT NOT NULL,
    ability TEXT NOT NULL,
    caller TEXT DEFAULT '',
    description TEXT DEFAULT ''
);

-- 能力调用日志
CREATE TABLE data_ability_log (
    ability_name TEXT NOT NULL,
    caller TEXT DEFAULT '',
    action TEXT DEFAULT '',
    input_params TEXT DEFAULT '',
    created_at REAL
);

-- 每日唯一调用
CREATE TABLE data_ability_daily (
    ability_name TEXT,
    caller TEXT,
    input_hash TEXT,
    stat_date TEXT
);
```

### 核心方法
```rust
// 初始化表
DataAudit::init_tables(db)?;

// 注册能力
DataAudit::register_ability(db, "testtb", "getone", "TestTb,ClassA", "获取单条记录")?;

// 权限检查
audit.check_permission(db, "getone", "TestTb")?;

// 闭包执行（需手动传 caller，不推荐）
audit.do_action(db, "getone", "ClassA", || self.getone_impl(db, id))?;

// 委托执行（自动用 micro_name 作 caller）
audit.delegate(db, "getone", || self.getone_impl(db, id))?;

// 带输入参数
audit.delegate_with(db, "save", &input, || self.save_impl(db, record))?;
```

### with_audit 宏（推荐）
```rust
// 不带输入参数
with_audit!(self, db, "getone", self.getone_impl(db, id))

// 带输入参数
with_audit!(self, db, "save", input, self.save_impl(db, record))
```

## 好坏示例

### caller 传入方式
- error - 手动传入 caller，可伪造 audit.do_action(db, "getone", "Admin", || ...)
- good - 使用宏自动获取 caller with_audit!(self, db, "getone", self.getone_impl(db, id))
- good - 使用 delegate 方法 audit.delegate(db, "getone", || self.getone_impl(db, id))

### 绕过权限检查
- error - 直接调用实现绕过检查 self.getone_impl(db, id) 无权限检查，无审计日志
- good - 始终通过宏或 delegate 调用 with_audit!(self, db, "getone", self.getone_impl(db, id))

### 审计开关
- good - DATASTATE_AUDIT=1 开启审计，严格检查权限
- good - 未开启时跳过权限检查但仍记录日志，用于审计追踪

### 权限配置
- good - caller 字段为空表示不限制，任何调用者都可使用
- good - caller 字段为 "ClassA,ClassB" 表示只允许这两个类调用