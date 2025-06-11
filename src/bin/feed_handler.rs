use nats;
use rand::seq::SliceRandom;
use rand::Rng;
use serde::{Deserialize, Serialize};
use my_rust_project::utils::now_nanos;
use std::thread;
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize, Clone)]
enum Action {
    Buy,
    Sell,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
enum OrderType {
    Market,
    Limit,
    Cancel,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Order {
    id: u128,
    symbol: String,
    price: f64,
    amount: f64,
    action: Action,
    order_type: OrderType,
    timestamp: u128,
}

fn main() {
    let nc = nats::connect("localhost:4222").expect("Failed to connect to NATS");
    let symbols = ["AAPL", "TSLA"];
    let actions = [Action::Buy, Action::Sell];
    let order_types = [OrderType::Market, OrderType::Limit, OrderType::Cancel];
    let mut rng = rand::thread_rng();

    loop {
        let symbol = symbols.choose(&mut rng).unwrap().to_string();
        let action = actions.choose(&mut rng).unwrap().clone();
        let order_type = order_types.choose(&mut rng).unwrap().clone();
        let price = rng.gen_range(100.0..500.0);
        let amount = rng.gen_range(1.0..100.0);
        let now_ns = now_nanos();

        let order = Order {
            id: now_ns,
            symbol,
            price,
            amount,
            action,
            order_type,
            timestamp: now_ns,
        };

        let payload = serde_json::to_vec(&order).unwrap();
        nc.publish("market_data", payload).unwrap();
        println!("Published: {:?}", order);

        thread::sleep(Duration::from_secs(2));
    }
}