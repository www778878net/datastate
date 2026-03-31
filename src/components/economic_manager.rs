//! 经济管理组件 (EconomicManager)
//! 管理能力的定价、成本和收入统计

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// 经济管理组件：管理定价、成本和收入
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicManager {
    // 定价信息
    /// 基础价格
    #[serde(default = "default_price")]
    pub pricebase: f64,
    /// 当前价格
    #[serde(default = "default_price")]
    pub price: f64,
    /// 单位成本
    #[serde(default)]
    pub costunit: f64,
    /// 价格描述
    #[serde(default)]
    pub pricedescription: Value,
    /// 成本描述
    #[serde(default)]
    pub costdescription: Value,

    // 经济统计
    /// 总收入
    #[serde(default)]
    pub revenuetotal: f64,
    /// 总成本
    #[serde(default)]
    pub costtotal: f64,
    /// 总利润
    #[serde(default)]
    pub profittotal: f64,
    /// 目标利润率
    #[serde(default = "default_profittarget")]
    pub profittarget: f64,
    /// 投资回报率
    #[serde(default)]
    pub roi: f64,
}

fn default_price() -> f64 { 1.0 }
fn default_profittarget() -> f64 { 0.2 }

impl Default for EconomicManager {
    fn default() -> Self {
        Self {
            pricebase: 1.0,
            price: 1.0,
            costunit: 0.0,
            pricedescription: Value::Object(serde_json::Map::new()),
            costdescription: Value::Object(serde_json::Map::new()),
            revenuetotal: 0.0,
            costtotal: 0.0,
            profittotal: 0.0,
            profittarget: 0.2,
            roi: 0.0,
        }
    }
}

impl EconomicManager {
    /// 增加成本
    pub fn add_cost(&mut self, cost: f64) {
        self.costtotal += cost;
        self.update_profit_and_roi();
    }

    /// 增加收入
    pub fn add_revenue(&mut self, revenue: f64) {
        self.revenuetotal += revenue;
        self.update_profit_and_roi();
    }

    /// 更新利润和ROI
    fn update_profit_and_roi(&mut self) {
        self.profittotal = self.revenuetotal - self.costtotal;
        if self.costtotal > 0.0 {
            self.roi = (self.profittotal / self.costtotal) * 100.0;
        } else {
            self.roi = 0.0;
        }
    }

    /// 从字典加载数据
    pub fn load_from_dict(&mut self, data: &HashMap<String, Value>) {
        if let Some(v) = data.get("pricebase").and_then(|v| v.as_f64()) {
            self.pricebase = v;
        }
        if let Some(v) = data.get("price").and_then(|v| v.as_f64()) {
            self.price = v;
        }
        if let Some(v) = data.get("costunit").and_then(|v| v.as_f64()) {
            self.costunit = v;
        }
        if let Some(v) = data.get("pricedescription").cloned() {
            self.pricedescription = v;
        }
        if let Some(v) = data.get("costdescription").cloned() {
            self.costdescription = v;
        }
        if let Some(v) = data.get("revenuetotal").and_then(|v| v.as_f64()) {
            self.revenuetotal = v;
        }
        if let Some(v) = data.get("costtotal").and_then(|v| v.as_f64()) {
            self.costtotal = v;
        }
        if let Some(v) = data.get("profittotal").and_then(|v| v.as_f64()) {
            self.profittotal = v;
        }
        if let Some(v) = data.get("profittarget").and_then(|v| v.as_f64()) {
            self.profittarget = v;
        }
        if let Some(v) = data.get("roi").and_then(|v| v.as_f64()) {
            self.roi = v;
        }
    }

    /// 转换为字典
    pub fn to_dict(&self) -> HashMap<String, Value> {
        let mut map = HashMap::new();
        map.insert("pricebase".to_string(), serde_json::json!(self.pricebase));
        map.insert("price".to_string(), serde_json::json!(self.price));
        map.insert("costunit".to_string(), serde_json::json!(self.costunit));
        map.insert("pricedescription".to_string(), self.pricedescription.clone());
        map.insert("costdescription".to_string(), self.costdescription.clone());
        map.insert("revenuetotal".to_string(), serde_json::json!(self.revenuetotal));
        map.insert("costtotal".to_string(), serde_json::json!(self.costtotal));
        map.insert("profittotal".to_string(), serde_json::json!(self.profittotal));
        map.insert("profittarget".to_string(), serde_json::json!(self.profittarget));
        map.insert("roi".to_string(), serde_json::json!(self.roi));
        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试1：默认创建
    /// 验证：pricebase=1.0, revenuetotal=0.0, roi=0.0
    #[test]
    fn test_economic_manager_default() {
        let manager = EconomicManager::default();

        assert_eq!(manager.pricebase, 1.0);
        assert_eq!(manager.price, 1.0);
        assert_eq!(manager.profittarget, 0.2);
        assert_eq!(manager.revenuetotal, 0.0);
        assert_eq!(manager.costtotal, 0.0);
        assert_eq!(manager.profittotal, 0.0);
        assert_eq!(manager.roi, 0.0);
    }

    /// 测试2：成本收入统计
    /// 验证：profittotal=5.0, roi=50.0
    #[test]
    fn test_economic_manager_cost_revenue() {
        let mut manager = EconomicManager::default();

        manager.add_cost(10.0);
        manager.add_revenue(15.0);

        assert_eq!(manager.costtotal, 10.0);
        assert_eq!(manager.revenuetotal, 15.0);
        assert_eq!(manager.profittotal, 5.0);
        assert_eq!(manager.roi, 50.0); // (5/10) * 100
    }

    /// 测试3：多次添加成本和收入
    #[test]
    fn test_economic_manager_multiple_operations() {
        let mut manager = EconomicManager::default();

        manager.add_cost(5.0);
        manager.add_cost(5.0);
        manager.add_revenue(20.0);

        assert_eq!(manager.costtotal, 10.0);
        assert_eq!(manager.revenuetotal, 20.0);
        assert_eq!(manager.profittotal, 10.0);
    }

    /// 测试4：零成本时的ROI处理
    #[test]
    fn test_economic_manager_zero_cost_roi() {
        let mut manager = EconomicManager::default();

        // 成本为0时，ROI应为0
        manager.add_revenue(10.0);

        assert_eq!(manager.costtotal, 0.0);
        assert_eq!(manager.revenuetotal, 10.0);
        assert_eq!(manager.roi, 0.0);
    }

    /// 测试5：字典加载和转换
    #[test]
    fn test_economic_manager_dict_operations() {
        let mut manager = EconomicManager::default();
        manager.price = 100.0;
        manager.costunit = 0.5;
        manager.revenuetotal = 500.0;
        manager.costtotal = 200.0;

        let dict = manager.to_dict();

        assert_eq!(dict.get("price").and_then(|v| v.as_f64()), Some(100.0));
        assert_eq!(dict.get("costunit").and_then(|v| v.as_f64()), Some(0.5));

        let mut loaded = EconomicManager::default();
        loaded.load_from_dict(&dict);

        assert_eq!(loaded.price, 100.0);
        assert_eq!(loaded.costunit, 0.5);
    }

    /// 测试6：负利润场景
    #[test]
    fn test_economic_manager_negative_profit() {
        let mut manager = EconomicManager::default();

        manager.add_cost(20.0);
        manager.add_revenue(10.0);

        assert_eq!(manager.profittotal, -10.0);
        assert_eq!(manager.roi, -50.0); // (-10/20) * 100
    }
}