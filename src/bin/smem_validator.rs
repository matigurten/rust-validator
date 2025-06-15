use chrono::Local;
use memmap2::MmapOptions;
use prost::Message;
use std::fs::OpenOptions;
use std::thread;
use std::time::Duration;
use rust_validator::utils::now_nanos;
use rust_validator::orderbook::{Order, Action, OrderType};
use glob;

mod proto {
    include!(concat!(env!("OUT_DIR"), "/order.rs"));
}

const SHM_SIZE: usize = 4096;

fn main() -> std::io::Result<()> {
    let busy_mode = std::env::var("BUSY_MODE").unwrap_or_else(|_| "0".to_string()) == "1";
    // Get current date in YYYYMMDD format
    let date_str = Local::now().format("%Y%m%d").to_string();
    // Find all shared memory files for this date
    let pattern = format!("/tmp/{}.*", date_str);
    let files: Vec<_> = glob::glob(&pattern).unwrap().filter_map(Result::ok).collect();
    println!("[VALIDATOR] Scanning for shared memory files with pattern: {}", pattern);
    println!("[VALIDATOR] Files found: {:?}", files);
    if files.is_empty() {
        println!("No shared memory files found for date {}", date_str);
        return Ok(());
    }
    println!("Found shared memory files: {:?}", files);
    // For each file, extract the symbol from the filename and process orders
    for file_path in files {
        let exchange = file_path.file_name().unwrap().to_str().unwrap().split('.').nth(1).unwrap_or("");
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&file_path)?;
        let mut mmap = unsafe { MmapOptions::new().map_mut(&file)? };
        let mut last_id = 0u64;
        // Collect command-line arguments for instrument filtering
        let args: Vec<String> = std::env::args().skip(1).collect();
        let filter_instruments: Option<std::collections::HashSet<String>> = if args.is_empty() {
            None
        } else {
            Some(args.into_iter().collect())
        };
        loop {
            // Read the length of the message (first 4 bytes)
            let len = u32::from_le_bytes([mmap[0], mmap[1], mmap[2], mmap[3]]) as usize;
            if len > 0 && len < SHM_SIZE - 4 {
                // Read the message bytes
                let msg_bytes = &mmap[4..4 + len];
                if let Ok(proto_order) = proto::Order::decode(msg_bytes) {
                    // Only print if it's a new order
                    if proto_order.id != last_id {
                        // No instrument filter: print all orders for now
                        let order = Order {
                            id: proto_order.id as u128,
                            price: proto_order.price,
                            amount: proto_order.amount,
                            action: match proto_order.action {
                                0 => Action::Buy,
                                1 => Action::Sell,
                                _ => continue,
                            },
                            order_type: match proto_order.order_type {
                                0 => OrderType::Market,
                                1 => OrderType::Limit,
                                2 => OrderType::Cancel,
                                _ => continue,
                            },
                            timestamp: proto_order.timestamp as u128,
                            instrument: proto_order.instrument.clone(),
                        };
                        // Filter by instrument if a filter is set
                        if let Some(ref filter) = filter_instruments {
                            if !filter.contains(&order.instrument) {
                                last_id = proto_order.id;
                                let len_ptr = &mut mmap[0..4];
                                len_ptr.copy_from_slice(&0u32.to_le_bytes());
                                continue;
                            }
                        }
                        let now = now_nanos();
                        let latency_us = (now as i64 - order.timestamp as i64) / 1000;
                        // Change the log output to match the feed_handler format
                        let order_type_str = match order.order_type {
                            OrderType::Limit => "NEW",
                            OrderType::Cancel => "CNL",
                            OrderType::Market => "MKT",
                        };
                        let side = match order.action {
                            Action::Buy => "Buy",
                            Action::Sell => "Sell",
                        };
                        let lmt_str = match order.order_type {
                            OrderType::Limit => "LMT",
                            OrderType::Cancel => "LMT",
                            OrderType::Market => "MKT",
                        };
                        if order.order_type == OrderType::Limit || order.order_type == OrderType::Cancel {
                            println!(
                                "{} {}: {:>3} \t{} # {}: {:<4} {:>3} @ {:.1} | Latency: {} us",
                                exchange, order.instrument, order_type_str, lmt_str, order.id, side, order.amount, order.price, latency_us
                            );
                        } else {
                            // For Market or other types, print with Incoming and timestamp
                            println!(
                                "Incoming {} {} {}: {:>3} \t{} # {}: {:<4} {:>3} @ {:.1} | Latency: {} us",
                                order.timestamp, exchange, order.instrument, order_type_str, lmt_str, order.id, side, order.amount, order.price, latency_us
                            );
                        }
                        last_id = proto_order.id;
                        // After processing, clear the slot by setting length to zero
                        let len_ptr = &mut mmap[0..4];
                        len_ptr.copy_from_slice(&0u32.to_le_bytes());
                    }
                }
            }
            if !busy_mode {
                // Sleep a tiny bit to avoid 100% CPU (adjust as needed)
                thread::sleep(Duration::from_micros(50));
            } else {
                // Busy mode: spin for lowest possible latency
                std::hint::spin_loop();
            }
        }
    }
    Ok(())
}
