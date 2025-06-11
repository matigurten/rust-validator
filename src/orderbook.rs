use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Action {
    Buy,
    Sell,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum OrderType {
    Market,
    Limit,
    Cancel,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Order {
    pub id: u128,
    pub symbol: String,
    pub price: f64,
    pub amount: i32,
    pub action: Action,
    pub order_type: OrderType,
    pub timestamp: u128,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PriceLevel {
    pub price: f64,
    pub total_amount: i32,
    pub orders: Vec<Order>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OrderBook {
    pub symbol: String,
    pub bids: BTreeMap<i64, PriceLevel>,  // Using i64 for price to avoid floating point comparisons
    pub asks: BTreeMap<i64, PriceLevel>,  // Using i64 for price to avoid floating point comparisons
    pub last_update: u128,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BookUpdate {
    pub symbol: String,
    pub bids: Vec<PriceLevel>,
    pub asks: Vec<PriceLevel>,
    pub last_update: u128,
}

impl OrderBook {
    pub fn new(symbol: String) -> Self {
        OrderBook {
            symbol,
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            last_update: 0,
        }
    }

    pub fn add_order(&mut self, order: &Order) {
        let price_levels = match order.action {
            Action::Buy => &mut self.bids,
            Action::Sell => &mut self.asks,
        };

        // Convert price to i64 for efficient comparison
        let price_key = (order.price * 1_000_000.0) as i64;
        
        // Find or create price level
        let price_level = price_levels.entry(price_key).or_insert_with(|| PriceLevel {
            price: order.price,
            total_amount: 0,
            orders: Vec::with_capacity(1), // Pre-allocate for better performance
        });

        price_level.orders.push(order.clone());
        price_level.total_amount += order.amount;
        self.last_update = order.timestamp;
    }

    pub fn cancel_order(&mut self, order: &Order) {
        let price_levels = match order.action {
            Action::Buy => &mut self.bids,
            Action::Sell => &mut self.asks,
        };

        let price_key = (order.price * 1_000_000.0) as i64;
        
        if let Some(price_level) = price_levels.get_mut(&price_key) {
            if let Some(pos) = price_level.orders.iter().position(|o| o.id == order.id) {
                let cancelled_order = price_level.orders.remove(pos);
                price_level.total_amount -= cancelled_order.amount;
                
                // Remove price level if empty
                if price_level.orders.is_empty() {
                    price_levels.remove(&price_key);
                }
            }
        }
        self.last_update = order.timestamp;
    }

    pub fn get_book_update(&self) -> BookUpdate {
        // Convert BTreeMap to Vec efficiently
        let bids: Vec<PriceLevel> = self.bids
            .iter()
            .rev() // Reverse for bids (highest first)
            .map(|(_, pl)| pl.clone())
            .collect();
            
        let asks: Vec<PriceLevel> = self.asks
            .iter()
            .map(|(_, pl)| pl.clone())
            .collect();

        BookUpdate {
            symbol: self.symbol.clone(),
            bids,
            asks,
            last_update: self.last_update,
        }
    }
}

pub fn validate(order: &Order) -> Result<(), String> {
    if order.price <= 0.0 {
        return Err("Invalid price".into());
    }
    if order.amount <= 0 {
        return Err("Invalid amount".into());
    }
    Ok(())
}