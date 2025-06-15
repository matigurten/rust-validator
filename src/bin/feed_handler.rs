use async_nats;
use rand::seq::SliceRandom;
use rand::Rng;
use rust_validator::utils::now_nanos;
use tokio::time::{sleep, Duration};
use prost::Message;

mod proto {
    include!(concat!(env!("OUT_DIR"), "/order.rs"));
}

#[tokio::main]
async fn main() {
    let nc = async_nats::connect("localhost:4222").await.expect("Failed to connect to NATS");
    let actions = [proto::Action::Buy, proto::Action::Sell];
    let order_types = [proto::OrderType::Market, proto::OrderType::Limit, proto::OrderType::Cancel];
    let mut rng = rand::thread_rng();
    let symbol = "TSLA";

    loop {
        let action = actions.choose(&mut rng).unwrap().clone();
        let order_type = order_types.choose(&mut rng).unwrap().clone();
        let price = ((rng.gen_range(100.0f64..500.0f64) * 10.0).round()) / 10.0;
        let amount = rng.gen_range(1.0..100.0) as i32;

        let mut buf = Vec::with_capacity(64);
        let now_ns = now_nanos();
        let order = proto::Order {
            id: now_ns as u64,
            price,
            amount,
            action: action as i32,
            order_type: order_type as i32,
            timestamp: now_ns as u64,
            instrument: symbol.to_string(),
        };
        order.encode(&mut buf).unwrap();
        nc.publish("market_data".into(), buf.into()).await.unwrap();
        println!("Published: {:?}", order);

        sleep(Duration::from_secs(2)).await;
    }
}