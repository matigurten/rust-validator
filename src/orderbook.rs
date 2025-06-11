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
    pub bids: Vec<PriceLevel>,
    pub asks: Vec<PriceLevel>,
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

        // Convert price to i64 for efficient comparison
        let price_key = (order.price * 1_000_000.0) as i64;

        // Find or create price level
        let price_level = price_levels
            .iter_mut()
            .find(|pl| (pl.price * 1_000_000.0) as i64 == price_key);
        match price_level {
            Some(pl) => {
                pl.orders.push(order.clone());
                pl.total_amount += order.amount;
            }
            None => {
                price_levels.push(PriceLevel {
                    price: order.price,
                    total_amount: order.amount,
                    orders: vec![order.clone()],
                });
            }
        }

        // After updating bids/asks, remove crossing orders
        self.bids
            .sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap());
        self.asks
            .sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap());
        // Remove crossing: bids >= best ask, asks <= best bid
        if let (Some(best_ask), Some(best_bid)) = (self.asks.first(), self.bids.first()) {
            let best_ask_price = best_ask.price;
            let best_bid_price = best_bid.price;
            self.bids.retain(|b| b.price < best_ask_price);
            self.asks.retain(|a| a.price > best_bid_price);
        }
        self.last_update = order.timestamp;
    }

    pub fn cancel_order(&mut self, order: &Order) {
        let price_levels = match order.action {
            Action::Buy => &mut self.bids,
            Action::Sell => &mut self.asks,
        };

        let price_key = (order.price * 1_000_000.0) as i64;

        // Find the index of the price level to remove the order from
        if let Some(idx) = price_levels
            .iter()
            .position(|pl| (pl.price * 1_000_000.0) as i64 == price_key)
        {
            // Remove the order from the orders vector
            let mut remove_price_level = false;
            if let Some(pos) = price_levels[idx].orders.iter().position(|o| o.id == order.id) {
                let cancelled_order = price_levels[idx].orders.remove(pos);
                price_levels[idx].total_amount -= cancelled_order.amount;

                // Mark for removal if no orders left at this price level
                if price_levels[idx].orders.is_empty() {
                    remove_price_level = true;
                }
            }
            // Remove the price level if needed (after mutable borrow ends)
            if remove_price_level {
                let price = price_levels[idx].price;
                // Drop all previous borrows before calling retain
                drop(idx);
                price_levels.retain(|pl| pl.price != price);
            }
        }
        self.last_update = order.timestamp;
    }

    pub fn get_book_update(&self) -> BookUpdate {
        // Convert BTreeMap to Vec efficiently
        let bids: Vec<PriceLevel> = self
            .bids
            .iter()
            .rev() // Reverse for bids (highest first)
            .map(|pl| pl.clone())
            .collect();

        let asks: Vec<PriceLevel> = self.asks.iter().map(|pl| pl.clone()).collect();

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
