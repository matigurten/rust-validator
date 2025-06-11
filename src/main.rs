use std::collections::HashMap;
use std::time::Instant;
use serde_json;

mod utils;
mod orderbook;
mod messaging;

use crate::orderbook::{Order, OrderBook, OrderType, BookUpdate, validate};
use crate::messaging::NatsClient;
use crate::utils::now_nanos;

fn is_important_update(last_update: &BookUpdate, new_update: &BookUpdate) -> bool {
    if last_update.bids.is_empty() && last_update.asks.is_empty() {
        return true;
    }
    let last_best_bid = last_update.bids.first().map(|pl| pl.price).unwrap_or(0.0);
    let last_best_ask = last_update.asks.first().map(|pl| pl.price).unwrap_or(f64::MAX);
    let new_best_bid = new_update.bids.first().map(|pl| pl.price).unwrap_or(0.0);
    let new_best_ask = new_update.asks.first().map(|pl| pl.price).unwrap_or(f64::MAX);
    (new_best_bid - last_best_bid).abs() / last_best_bid > 0.001 ||
    (new_best_ask - last_best_ask).abs() / last_best_ask > 0.001
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let nats_client = NatsClient::new("localhost:4222")?;
    let subscription = nats_client.subscribe("market_data")?;
    let mut order_books: HashMap<String, OrderBook> = HashMap::new();
    let mut last_updates: HashMap<String, BookUpdate> = HashMap::new();
    println!("Validator started. Listening for orders...");
    for message in subscription.messages() {
        let start = now_nanos();
        let order: Order = serde_json::from_slice(&message.data)?;
        if !order_books.contains_key(&order.symbol) {
            order_books.insert(order.symbol.clone(), OrderBook::new(order.symbol.clone()));
        }
        let book = order_books.get_mut(&order.symbol).unwrap();
        if let Err(e) = validate(&order) {
            println!("Invalid order: {}", e);
            continue;
        }
        match order.order_type {
            OrderType::Cancel => book.cancel_order(&order),
            _ => book.add_order(&order),
        }
        let book_update = book.get_book_update();
        let should_publish = last_updates
            .get(&order.symbol)
            .map(|last| is_important_update(last, &book_update))
            .unwrap_or(true);
        if should_publish {
            nats_client.publish_book_update("book_updates", &book_update)?;
            last_updates.insert(order.symbol.clone(), book_update);
        }
        let latency = now_nanos() - start;
        println!("Latency: {:.3} ms", latency as f64 / 1_000_000.0);
    }
    Ok(())
}
