use std::collections::HashMap;
use rust_validator::orderbook::{Order, OrderBook, OrderType, BookUpdate, Action};
use rust_validator::utils::now_nanos;
use prost::Message;
use futures_util::StreamExt;

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
    let client = async_nats::connect("localhost:4222").await?;
    let mut subscription = client.subscribe("market_data".to_string()).await?;

    println!("Validator started. Waiting for market data...");

    while let Some(message) = subscription.next().await {
        let start = now_nanos();
        // Try to decode the message data
        let proto_order = match proto::Order::decode(message.payload.as_ref()) {
            Ok(order) => order,
            Err(e) => {
                eprintln!("Failed to decode message: {}", e);
                eprintln!("Raw message data: {:?}", message.payload);
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
            // Print the current top of the book
            let best_bid = update.bids.first().map(|b| format!("{:.2} x {}", b.price, b.total_amount)).unwrap_or("None".to_string());
            let best_ask = update.asks.first().map(|a| format!("{:.2} x {}", a.price, a.total_amount)).unwrap_or("None".to_string());
            println!("TSLA BOOK TOP | Bid: {} | Ask: {}", best_bid, best_ask);
        }

        if should_publish {
            // Encode the prost-generated proto::BookUpdate, not the domain BookUpdate
            let proto_update = rust_validator::messaging::proto::BookUpdate {
                symbol: update.symbol.clone(),
                bids: update.bids.iter().map(|pl| rust_validator::messaging::proto::PriceLevel {
                    price: pl.price,
                    amount: pl.total_amount,
                }).collect(),
                asks: update.asks.iter().map(|pl| rust_validator::messaging::proto::PriceLevel {
                    price: pl.price,
                    amount: pl.total_amount,
                }).collect(),
                timestamp: update.last_update as u64,
            };
            let mut buf = Vec::with_capacity(128);
            prost::Message::encode(&proto_update, &mut buf).unwrap();
            if let Err(e) = client.publish("book_updates".into(), buf.into()).await {
                eprintln!("Failed to publish book update: {}", e);
            } else {
                last_updates.insert(order.symbol.clone(), update);
            }
        }

        let inter_service_latency_us = (start - order.timestamp) as i32 / 1000;
        let end = now_nanos();
        let processing_time_us = (end - start) as i32 / 1000;
        
        println!(
            "Order id {}: {:<4} {:<4} {:>4} @ {:.2} | inter-service latency: {} us | processing: {} us",
            &order.id,
            &order.symbol.chars().take(4).collect::<String>(),
            format!("{:<4}", format!("{:?}", order.action)).chars().take(4).collect::<String>(),
            format!("{:>4}", order.amount),
            order.price,
            inter_service_latency_us,
            processing_time_us
        );
    }

    Ok(())
}
