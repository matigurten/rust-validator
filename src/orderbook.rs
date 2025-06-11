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
        Self {
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

        let price_key = (order.price * 1_000_000.0) as i64;

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

        self.bids
            .sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap());
        self.asks
            .sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap());

        let mut book_changed = false;
        let (crossed, trades) = match order.action {
            Action::Buy => {
                let mut remaining = order.amount;
                let mut trades = Vec::new();
                let mut to_remove = Vec::new();

                for ask in &mut self.asks {
                    if ask.price <= order.price && remaining > 0 {
                        let mut matched_orders = Vec::new();
                        let mut ask_remaining = ask.total_amount;
                        let mut i = 0;
                        while i < ask.orders.len() && remaining > 0 {
                            let ask_order = &mut ask.orders[i];
                            let fill = remaining.min(ask_order.amount);
                            trades.push(format!(
                                "TRADE: Buy {} {} @ {:.2} matched with Sell order {} for {}",
                                fill, order.symbol, ask.price, ask_order.id, fill
                            ));
                            ask_order.amount -= fill;
                            ask_remaining -= fill;
                            remaining -= fill;
                            if ask_order.amount == 0 {
                                matched_orders.push(i);
                            } else {
                                i += 1;
                            }
                        }
                        for &idx in matched_orders.iter().rev() {
                            ask.orders.remove(idx);
                        }
                        ask.total_amount = ask_remaining;
                        if ask.orders.is_empty() {
                            to_remove.push(ask.price);
                        }
                        if remaining == 0 {
                            break;
                        }
                    }
                }
                self.asks.retain(|a| !to_remove.contains(&a.price));
                if !trades.is_empty() {
                    book_changed = true;
                }
                (remaining < order.amount, trades)
            }
            Action::Sell => {
                let mut remaining = order.amount;
                let mut trades = Vec::new();
                let mut to_remove = Vec::new();

                for bid in &mut self.bids {
                    if bid.price >= order.price && remaining > 0 {
                        let mut matched_orders = Vec::new();
                        let mut bid_remaining = bid.total_amount;
                        let mut i = 0;
                        while i < bid.orders.len() && remaining > 0 {
                            let bid_order = &mut bid.orders[i];
                            let fill = remaining.min(bid_order.amount);
                            trades.push(format!(
                                "TRADE: Sell {} {} @ {:.2} matched with Buy order {} for {}",
                                fill, order.symbol, bid.price, bid_order.id, fill
                            ));
                            bid_order.amount -= fill;
                            bid_remaining -= fill;
                            remaining -= fill;
                            if bid_order.amount == 0 {
                                matched_orders.push(i);
                            } else {
                                i += 1;
                            }
                        }
                        for &idx in matched_orders.iter().rev() {
                            bid.orders.remove(idx);
                        }
                        bid.total_amount = bid_remaining;
                        if bid.orders.is_empty() {
                            to_remove.push(bid.price);
                        }
                        if remaining == 0 {
                            break;
                        }
                    }
                }
                self.bids.retain(|b| !to_remove.contains(&b.price));
                if !trades.is_empty() {
                    book_changed = true;
                }
                (remaining < order.amount, trades)
            }
        };

        if crossed {
            for trade in trades {
                println!("{}", trade);
            }
        }

        self.last_update = order.timestamp;

        // Print top of book for any symbol if the book changed
        if book_changed {
            let (bid_amt, bid_price) = self.bids.first()
                .map(|b| (b.total_amount, b.price))
                .unwrap_or((0, 0.0));
            let (ask_price, ask_amt) = self.asks.first()
                .map(|a| (a.price, a.total_amount))
                .unwrap_or((0.0, 0));
            println!(
                "{} BOOK TOP | {:>5} {:>8.2} | {:<8.2} {:<5}",
                self.symbol, bid_amt, bid_price, ask_price, ask_amt
            );
        }
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
