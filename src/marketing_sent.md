# marketing_sent - 已发送记录表

## 概述

MarketingSent 用于记录已发送的营销内容，按标题+平台去重。

## 第一性目的

营销内容发送记录管理：
- 记录已发送的营销内容
- 按标题+平台去重
- 支持同步功能

## 核心类型

### SentRecord

已发送记录：
- `id` - 业务主键
- `cid` - 公司ID
- `platform` - 平台
- `title` - 标题
- `url` - 链接
- `content` - 内容
- `keyword` - 关键词
- `senttime` - 发送时间
- `uptime` - 更新时间
- `upby` - 更新人

### MarketingSent

已发送记录表 DataService：
- `db: LocalDB` - 本地数据库
- `audit: DataAudit` - 审计组件
- `state: DataState` - 数据状态

## 建表 SQL

```sql
CREATE TABLE IF NOT EXISTS marketing_sent (
    id TEXT NOT NULL PRIMARY KEY,
    cid TEXT NOT NULL DEFAULT '',
    platform TEXT NOT NULL DEFAULT '',
    title TEXT NOT NULL DEFAULT '',
    url TEXT NOT NULL DEFAULT '',
    content TEXT NOT NULL DEFAULT '',
    keyword TEXT NOT NULL DEFAULT '',
    senttime TEXT NOT NULL DEFAULT '',
    uptime TEXT NOT NULL DEFAULT '',
    upby TEXT NOT NULL DEFAULT ''
)
```

## 测试方案

### 主要逻辑测试

#### 测试1：创建实例
```
输入：LocalDB 实例
步骤：MarketingSent::new()
预期：返回有效实例
```

#### 测试2：插入发送记录
```
输入：SentRecord { platform, title, url, ... }
步骤：m_add() 或 m_save()
预期：记录插入成功
```

#### 测试3：去重检查
```
输入：相同 title + platform
步骤：两次插入相同记录
预期：第二次更新已有记录
```

### 其它测试（边界、异常等）

#### 测试4：查询已发送记录
```
输入：title, platform
步骤：get_sent()
预期：返回匹配的记录
```

#### 测试5：同步保存
```
输入：record
步骤：m_sync_save()
预期：保存成功，不写 sync_queue
```

## 注意事项

- 此模块为开发中的功能
- 需要 MySQL 数据库环境进行完整测试
