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
