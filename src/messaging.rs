use std::io::Error;
use nats;
use prost::Message;
use crate::orderbook::{Order, BookUpdate};

mod proto {
    include!(concat!(env!("OUT_DIR"), "/order.rs"));
}

pub struct NatsClient {
    connection: nats::Connection,
}

impl NatsClient {
    pub fn new(url: &str) -> Result<Self, Error> {
        let connection = nats::connect(url)?;
        Ok(Self { connection })
    }

    pub fn subscribe(&self, subject: &str) -> Result<nats::Subscription, Error> {
        Ok(self.connection.subscribe(subject)?)
    }

    pub fn publish_book_update(&self, subject: &str, update: &BookUpdate) -> Result<(), Error> {
        let proto_update = proto::BookUpdate {
            symbol: update.symbol.clone(),
            bids: update.bids.iter().map(|pl| proto::PriceLevel {
                price: pl.price,
                amount: pl.total_amount as i32,
            }).collect(),
            asks: update.asks.iter().map(|pl| proto::PriceLevel {
                price: pl.price,
                amount: pl.total_amount as i32,
            }).collect(),
            timestamp: update.last_update as u64,
        };
        let buf = proto_update.encode_to_vec();
        self.connection.publish(subject, &buf)?;
        Ok(())
    }

    pub fn publish_order(&self, subject: &str, order: &Order) -> Result<(), Error> {
        let proto_order = proto::Order {
            id: order.id as u64,
            symbol: order.symbol.clone(),
            price: order.price,
            amount: order.amount as i32,
            action: match order.action {
                crate::orderbook::Action::Buy => 0,
                crate::orderbook::Action::Sell => 1,
            },
            order_type: match order.order_type {
                crate::orderbook::OrderType::Market => 0,
                crate::orderbook::OrderType::Limit => 1,
                crate::orderbook::OrderType::Cancel => 2,
            },
            timestamp: order.timestamp as u64,
        };
        let buf = proto_order.encode_to_vec();
        self.connection.publish(subject, &buf)?;
        Ok(())
    }

    pub fn publish_error(&self, subject: &str, error: &str) -> Result<(), Error> {
        self.connection.publish(subject, error.as_bytes())?;
        Ok(())
    }
}