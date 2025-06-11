use serde::{Deserialize, Serialize};

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
    pub amount: f64,
    pub action: Action,
    pub order_type: OrderType,
    pub timestamp: u128,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PriceLevel {
    pub price: f64,
    pub total_amount: f64,
    pub orders: Vec<Order>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OrderBook {
    pub symbol: String,
    pub bids: Vec<PriceLevel>,  // Sorted by price (highest first)
    pub asks: Vec<PriceLevel>,  // Sorted by price (lowest first)
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
            bids: Vec::new(),
            asks: Vec::new(),
            last_update: 0,
        }
    }

    pub fn add_order(&mut self, order: &Order) {
        let price_levels = match order.action {
            Action::Buy => &mut self.bids,
            Action::Sell => &mut self.asks,
        };

        // Find or create price level
        let price_level = match price_levels.iter_mut().find(|pl| pl.price == order.price) {
            Some(pl) => pl,
            None => {
                let new_pl = PriceLevel {
                    price: order.price,
                    total_amount: 0.0,
                    orders: Vec::new(),
                };
                price_levels.push(new_pl);
                // Sort price levels
                match order.action {
                    Action::Buy => price_levels.sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap()),
                    Action::Sell => price_levels.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap()),
                }
                price_levels.last_mut().unwrap()
            }
        };

        price_level.orders.push(order.clone());
        price_level.total_amount += order.amount;
        self.last_update = order.timestamp;
    }

    pub fn cancel_order(&mut self, order: &Order) {
        let price_levels = match order.action {
            Action::Buy => &mut self.bids,
            Action::Sell => &mut self.asks,
        };

        if let Some(price_level) = price_levels.iter_mut().find(|pl| pl.price == order.price) {
            if let Some(pos) = price_level.orders.iter().position(|o| o.id == order.id) {
                let cancelled_order = price_level.orders.remove(pos);
                price_level.total_amount -= cancelled_order.amount;
                
                // Remove price level if empty
                if price_level.orders.is_empty() {
                    if let Some(pos) = price_levels.iter().position(|pl| pl.price == order.price) {
                        price_levels.remove(pos);
                    }
                }
            }
        }
        self.last_update = order.timestamp;
    }

    pub fn get_book_update(&self) -> BookUpdate {
        BookUpdate {
            symbol: self.symbol.clone(),
            bids: self.bids.clone(),
            asks: self.asks.clone(),
            last_update: self.last_update,
        }
    }
}

pub fn validate(order: &Order) -> Result<(), String> {
    if order.price <= 0.0 {
        return Err("Invalid price".into());
    }
    if order.amount <= 0.0 {
        return Err("Invalid amount".into());
    }
    Ok(())
} 