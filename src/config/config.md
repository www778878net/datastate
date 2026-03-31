# Config - 配置管理模块

## 第一性目的

提供全局配置管理能力，支持从JSON文件加载配置，并提供类型安全的配置访问接口。

## 主要类型

### ConfigError

配置错误枚举类型：
- `FileNotFound(String)` - 配置文件不存在
- `ParseError(String)` - 配置解析失败
- `NotInitialized` - 配置未初始化
- `KeyNotFound(String)` - 配置键不存在

### Config

配置结构体，包含：
- `config_object` - 配置对象（HashMap<String, Value>）
- `tables` - 表配置管理器
- `project_root` - 项目根目录

## 核心方法

### 初始化方法
- `init(config_file: Option<&str>)` - 初始化配置，支持环境变量 `CONFIG_FILE` 和 `APP_ENV`
- `load_config_file(path: &Path)` - 从文件加载配置

### 配置访问方法
- `get(key: &str) -> Option<&Value>` - 获取配置项
- `get_string(key: &str) -> Option<String>` - 获取字符串配置
- `get_int(key: &str) -> Option<i64>` - 获取整数配置
- `get_bool(key: &str) -> Option<bool>` - 获取布尔配置
- `has(key: &str) -> bool` - 检查配置项是否存在
- `set(key: &str, value: Value)` - 设置配置项

### 表配置方法
- `get_table(table_name: &str) -> Option<&TableSet>` - 获取表配置
- `table_names() -> Vec<&String>` - 获取所有表名

## 全局实例

使用单例模式，通过 `get_instance()` 获取全局配置实例。

## 示例

```rust
use datastate::config::Config;

// 初始化配置
let config = Config::get_instance();
config.lock().unwrap().init(Some("config/development.json")).ok();

// 读取配置
let db_host = config.lock().unwrap().get_string("db_host");
let timeout = config.lock().unwrap().get_int("timeout");
```

## 环境变量

- `CONFIG_FILE` - 配置文件路径
- `APP_ENV` - 运行环境（development/production），默认 development

## 测试方案

### 主要逻辑测试

#### 测试1：创建新实例
```
输入：Config::new_instance()
步骤：创建新的配置实例
预期：get("notexist") 返回 None
```

#### 测试2：设置和获取配置
```
输入：key1="value1", key2=123, key3=true
步骤：config.set() 设置后 config.get_xxx() 获取
预期：get_string("key1")="value1", get_int("key2")=123, get_bool("key3")=true
```

#### 测试3：配置项存在检查
```
输入：key1
步骤：设置前 has("key1")=false, 设置后 has("key1")=true
预期：has() 正确反映配置项是否存在
```

#### 测试4：单例模式
```
输入：Config::get_instance() 两次
步骤：获取两个实例并比较指针
预期：两个实例是同一个 Arc（ptr_eq = true）
```

### 其它测试（边界、异常等）

#### 测试5：获取不存在的表配置
```
输入：get_table("notexist")
步骤：查询不存在的表
预期：返回 None
```

#### 测试6：空表名列表
```
输入：新创建的配置实例
步骤：table_names()
预期：返回空数组
```

#### 测试7：错误类型显示
```
输入：各类型 ConfigError
步骤：to_string() 获取错误信息
预期：FileNotFound 包含文件名，ParseError 包含错误信息，NotInitialized 包含"未初始化"
