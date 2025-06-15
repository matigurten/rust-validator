use chrono::Local;
use memmap2::MmapOptions;
use rand::rngs::ThreadRng;
use rand::seq::SliceRandom;
use rand::Rng;
use rand_distr::Distribution;
use rust_validator::utils::now_nanos;
use std::fs::OpenOptions;
use std::{thread, time::Duration};
use prost::Message;

mod proto {
    include!(concat!(env!("OUT_DIR"), "/order.rs"));
}

const SHM_SIZE: usize = 4096; // Adjust as needed

struct InstrumentSimulator {
    mid_price: f64,
    volatility: f64,
}

impl InstrumentSimulator {
    fn new(mid_price: f64, volatility: f64) -> Self {
        Self { mid_price, volatility }
    }

    fn next_order_price(&mut self, is_buy: bool, rng: &mut ThreadRng) -> f64 {
        // Simulate price movement: drift mid price slightly
        let drift = rng.gen_range(-self.volatility..self.volatility);
        self.mid_price += drift;
        // For buys, price is below mid; for sells, price is above mid
        let normal = if is_buy {
            rand_distr::Normal::new(self.mid_price - 0.5, 1.0).unwrap()
        } else {
            rand_distr::Normal::new(self.mid_price + 0.5, 1.0).unwrap()
        };
        let mut price = normal.sample(rng);
        if price < 1.0 { price = 1.0; }
        // Round to nearest 10 cents
        price = (price * 10.0).round() / 10.0;
        price
    }
}

fn main() -> std::io::Result<()> {
    // Get current date in YYYYMMDD format
    let date_str = Local::now().format("%Y%m%d").to_string();
    let exchange = "NYSE";
    let instrument = "TSLA";
    let shm_path = format!("/tmp/{}.{}", date_str, exchange);
    // Create or open the shared memory file
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&shm_path)?;
    file.set_len(SHM_SIZE as u64)?;

    // Memory-map the file
    let mut mmap = unsafe { MmapOptions::new().map_mut(&file)? };

    let instruments = ["TSLA", "AAPL"];
    // Create a simulator for each instrument with different mean prices
    let mut simulators = std::collections::HashMap::new();
    simulators.insert("TSLA", InstrumentSimulator::new(300.0, 0.2));
    simulators.insert("AAPL", InstrumentSimulator::new(180.0, 0.15));
    let actions = [proto::Action::Buy, proto::Action::Sell];
    let mut rng = rand::thread_rng();

    loop {
        // Randomly pick an instrument for this order
        let instrument = instruments.choose(&mut rng).unwrap();
        let simulator = simulators.get_mut(instrument).unwrap();
        // Set to 50% chance for cancel orders for testing
        let order_type = if rng.gen_bool(0.5) {
            proto::OrderType::Limit
        } else {
            proto::OrderType::Cancel
        };
        let is_buy = actions.choose(&mut rng).unwrap() == &proto::Action::Buy;
        let (price, amount) = if order_type == proto::OrderType::Limit {
            (simulator.next_order_price(is_buy, &mut rng), rng.gen_range(1.0..100.0) as i32)
        } else {
            (0.0, 0) // Cancel orders have no price or amount
        };
        let now_ns = now_nanos();
        let action = if is_buy { proto::Action::Buy } else { proto::Action::Sell };
        let order = proto::Order {
            id: now_ns as u64,
            price,
            amount,
            action: action as i32,
            order_type: order_type as i32,
            timestamp: now_ns as u64,
            instrument: instrument.to_string(),
        };
        let mut buf = Vec::with_capacity(128);
        order.encode(&mut buf).unwrap();

        // Write the length and then the message to shared memory (simple protocol)
        let msg_len = buf.len() as u32;
        mmap[..4].copy_from_slice(&msg_len.to_le_bytes());
        mmap[4..4 + buf.len()].copy_from_slice(&buf);
        mmap.flush()?;

        // Print order in requested format
        let order_type_str = match order_type {
            proto::OrderType::Limit => "NEW",
            proto::OrderType::Cancel => "CNL",
            _ => "UNK",
        };
        if order_type == proto::OrderType::Limit || order_type == proto::OrderType::Cancel {
            let side = if is_buy { "Buy" } else { "Sell" };
            println!(
                "{}: {:>3} \tLMT # {}: {:<4} {:>3} @ {:.1}",
                instrument, order_type_str, order.id, side, amount, price
            );
        }

        thread::sleep(Duration::from_secs(2));
    }
}
