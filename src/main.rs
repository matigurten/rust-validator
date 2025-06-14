use std::collections::HashMap;
use rust_validator::orderbook::{Order, OrderBook, OrderType, Action};
use rust_validator::utils::now_nanos;
use prost::Message;
use futures_util::stream::StreamExt;
use std::task::{Context, Poll};
use futures_util::task::noop_waker;
use std::env;

mod proto {
    include!(concat!(env!("OUT_DIR"), "/order.rs"));
}

const INITIAL_CAPACITY: usize = 100;

fn is_important_update(new: &OrderBook, last: &OrderBook) -> bool {
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
    let mut last_updates: HashMap<String, OrderBook> = HashMap::with_capacity(INITIAL_CAPACITY);
    let client = async_nats::connect("localhost:4222").await?;
    let mut subscription = client.subscribe("market_data".to_string()).await?;

    println!("Validator started. Waiting for market data...");

    // Read BUSY_MODE environment variable to toggle busy/sleep mode
    let busy_mode = env::var("BUSY_MODE").unwrap_or_else(|_| "1".to_string()) == "1";
    println!("Busy mode: {}", busy_mode);

    // BUSY-POLLING LOOP: This loop will continuously poll for new messages without yielding or sleeping,
    // maximizing CPU usage for lowest possible message latency.
    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);
    loop {
        if busy_mode {
            // BUSY-POLLING: maximize CPU usage for lowest latency
            match StreamExt::poll_next_unpin(&mut subscription, &mut cx) {
                Poll::Ready(Some(message)) => {
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
                        .map(|last| is_important_update(update, last))
                        .unwrap_or(true);
                    if should_publish {
                        let best_bid = update.bids.first().map(|b| format!("{:.2} x {}", b.price, b.total_amount)).unwrap_or("None".to_string());
                        let best_ask = update.asks.first().map(|a| format!("{:.2} x {}", a.price, a.total_amount)).unwrap_or("None".to_string());
                        println!("TSLA BOOK TOP | Bid: {} | Ask: {}", best_bid, best_ask);
                    }
                    if should_publish {
                        last_updates.insert(order.symbol.clone(), update.clone());
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
                Poll::Ready(None) | Poll::Pending => {
                    std::hint::spin_loop();
                }
            }
        } else {
            // SLEEPING: poll for messages, but sleep briefly if none are available
            match StreamExt::poll_next_unpin(&mut subscription, &mut cx) {
                Poll::Ready(Some(message)) => {
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
                        .map(|last| is_important_update(update, last))
                        .unwrap_or(true);
                    if should_publish {
                        let best_bid = update.bids.first().map(|b| format!("{:.2} x {}", b.price, b.total_amount)).unwrap_or("None".to_string());
                        let best_ask = update.asks.first().map(|a| format!("{:.2} x {}", a.price, a.total_amount)).unwrap_or("None".to_string());
                        println!("TSLA BOOK TOP | Bid: {} | Ask: {}", best_bid, best_ask);
                    }
                    if should_publish {
                        last_updates.insert(order.symbol.clone(), update.clone());
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
                Poll::Ready(None) | Poll::Pending => {
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
            }
        }
    }
}
