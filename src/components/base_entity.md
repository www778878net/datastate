# BaseEntity - 基础实体组件

---

## 管理员指示

无（基础组件）

---

## 第一性目的

BaseEntity 是所有实体类的基类组件，负责：
- 身份标识（id、idpk、cid）
- 能力接口定义（inputjson、outputjson）
- 分类索引（apisys、apimicro、apiobj）
- 会话信息（sid、uname、uid）

---

## 完成标准

### 1. 实例创建成功
- 验证方法：`BaseEntity::default()`
- 预期结果：返回默认实例，state=1，apisys="apitmp"

### 2. 字典加载正确
- 验证方法：`entity.load_from_dict(&data)`
- 预期结果：字段从字典正确加载

### 3. 转换字典正确
- 验证方法：`entity.to_dict()`
- 预期结果：返回包含所有字段的HashMap

---

## 前置依赖

无

---

## 测试方案

### 主要逻辑

#### 测试1：默认创建
```
输入：无
步骤：BaseEntity::default()
预期：id="", state=1, apisys="apitmp"
```

#### 测试2：字典加载
```
输入：{"id": "test_001", "state": 2, "myname": "测试"}
步骤：entity.load_from_dict(&data)
预期：entity.id = "test_001", entity.state = 2
```

---

## 知识库

### 字段说明

#### 身份标识
- `id: String` - 全局唯一ID
- `idpk: Option<i64>` - 自增主键
- `cid: String` - 公司标识
- `state: i32` - 状态（0=待执行, 1=运行中, 2=已完成, 3=失败, 4=已取消, 5=暂停, 6=警告）
- `priority: i32` - 优先级
- `myname: String` - 实体名称
- `idworkflowinstance: String` - 工作流实例ID

#### 能力接口
- `inputjson: Value` - 输入接口定义
- `outputjson: Value` - 输出接口定义
- `description: Value` - 能力描述
- `configjson: Value` - 配置信息
- `resourcereq: Value` - 资源需求
- `preinputjson: Value` - 透传的原始输入

#### 分类索引
- `apisys: String` - 系统标识（默认"apitmp"）
- `apimicro: String` - 微服务标识
- `apiobj: String` - 对象/类标识
- `idagent: String` - 代理标识符

#### 会话信息
- `sid: String` - 会话ID
- `uname: String` - 用户名
- `uid: String` - 用户ID
- `money78: String` - 余额信息
- `consume: String` - 消费信息
- `coname: String` - 公司名称

### 方法说明
- `default()` - 创建默认实例
- `load_from_dict(&mut self, data: &HashMap<String, Value>)` - 从字典加载数据
- `to_dict(&self) -> HashMap<String, Value>` - 转换为字典

---

## 好坏示例

### 好示例
```rust
// 创建默认实例
let mut entity = BaseEntity::default();
entity.id = "test_001".to_string();
entity.state = 2;

// 从字典加载
let data = HashMap::from([
    ("id".to_string(), Value::String("test_002".to_string())),
]);
entity.load_from_dict(&data);

// 转换为字典
let dict = entity.to_dict();
```

### 坏示例
```rust
// 错误：state 使用字符串
entity.state = "completed".to_string(); // 错误，state 是 i32
entity.state = 2; // 正确
```
