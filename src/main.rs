use std::collections::HashMap;
use rust_validator::orderbook::{Order, OrderBook, OrderType, BookUpdate, Action};
use rust_validator::messaging::NatsClient;
use rust_validator::utils::now_nanos;
use prost::Message;

mod proto {
    include!(concat!(env!("OUT_DIR"), "/order.rs"));
}

const INITIAL_CAPACITY: usize = 100;

fn is_important_update(new: &BookUpdate, last: &BookUpdate) -> bool {
    if new.bids.is_empty() || new.asks.is_empty() || last.bids.is_empty() || last.asks.is_empty() {
        return true;
    }
    
    let bid_diff = (new.bids[0].price - last.bids[0].price).abs();
    let ask_diff = (new.asks[0].price - last.asks[0].price).abs();
    
    bid_diff > 0.01 || ask_diff > 0.01
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut order_books: HashMap<String, OrderBook> = HashMap::with_capacity(INITIAL_CAPACITY);
    let mut last_updates: HashMap<String, BookUpdate> = HashMap::with_capacity(INITIAL_CAPACITY);
    let client = NatsClient::new("localhost:4222")?;
    let subscription = client.subscribe("market_data")?;

    println!("Validator started. Waiting for market data...");

    for message in subscription.messages() {
        let start = now_nanos();
        
        // Try to decode the message data
        let proto_order = match proto::Order::decode(message.data.as_ref()) {
            Ok(order) => order,
            Err(e) => {
                eprintln!("Failed to decode message: {}", e);
                eprintln!("Raw message data: {:?}", message.data);
                continue;
            }
        };

        let order = Order {
            id: proto_order.id as u128,
            symbol: proto_order.symbol,
            price: proto_order.price,
            amount: proto_order.amount,
            action: match proto_order.action {
                0 => Action::Buy,
                1 => Action::Sell,
                _ => {
                    eprintln!("Invalid action value: {}", proto_order.action);
                    continue;
                }
            },
            order_type: match proto_order.order_type {
                0 => OrderType::Market,
                1 => OrderType::Limit,
                2 => OrderType::Cancel,
                _ => {
                    eprintln!("Invalid order type value: {}", proto_order.order_type);
                    continue;
                }
            },
            timestamp: proto_order.timestamp as u128,
        };

        let book = order_books.entry(order.symbol.clone()).or_insert_with(|| OrderBook::new(order.symbol.clone()));
        book.add_order(&order);

        let update = book.get_book_update();
        let should_publish = last_updates
            .get(&order.symbol)
            .map(|last| is_important_update(&update, last))
            .unwrap_or(true);

        if should_publish {
            if let Err(e) = client.publish_book_update("book_updates", &update) {
                eprintln!("Failed to publish book update: {}", e);
            } else {
                last_updates.insert(order.symbol.clone(), update);
            }
        }

        let latency = now_nanos() - start;
        println!(
            "Processed order: {} {:?} {} @ {} (id: {}) in {:.3} ms",
            order.symbol,
            order.action, // Use {:?} for Debug formatting
            order.amount,
            order.price,
            order.id,
            latency as f64 / 1_000_000.0
        );
    }

    Ok(())
}
