use nats;
use rand::seq::SliceRandom;
use rand::Rng;
use rust_validator::utils::now_nanos;
use std::thread;
use std::time::Duration;
use prost::Message;

mod proto {
    include!(concat!(env!("OUT_DIR"), "/order.rs"));
}

fn main() {
    let nc = nats::connect("localhost:4222").expect("Failed to connect to NATS");
    let symbols = ["AAPL", "TSLA"];
    let actions = [proto::Action::Buy, proto::Action::Sell];
    let order_types = [proto::OrderType::Market, proto::OrderType::Limit, proto::OrderType::Cancel];
    let mut rng = rand::thread_rng();

    loop {
        let symbol = symbols.choose(&mut rng).unwrap().to_string();
        let action = actions.choose(&mut rng).unwrap().clone();
        let order_type = order_types.choose(&mut rng).unwrap().clone();
        let price = ((rng.gen_range(100.0f64..500.0f64) * 10.0).round()) / 10.0;
        let amount = rng.gen_range(1.0..100.0) as i32;
        let now_ns = now_nanos();

        let order = proto::Order {
            id: now_ns as u64,
            symbol,
            price,
            amount,
            action: action as i32,
            order_type: order_type as i32,
            timestamp: now_ns as u64,
        };

        // Pre-allocate buffer for better performance
        let mut buf = Vec::with_capacity(64);
        order.encode(&mut buf).unwrap();
        nc.publish("market_data", buf).unwrap();
        println!("Published: {:?}", order);

        thread::sleep(Duration::from_secs(2));
    }
}