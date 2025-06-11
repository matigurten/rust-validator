use async_nats;
use prost::Message;
use crate::orderbook::{Order, BookUpdate};

pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/order.rs"));
}

pub struct NatsClient {
    client: async_nats::Client,
}

impl NatsClient {
    pub async fn new(url: &str) -> Result<Self, async_nats::Error> {
        let client = async_nats::connect(url).await?;
        Ok(Self { client })
    }

    pub async fn subscribe(&self, subject: &str) -> Result<async_nats::Subscriber, Box<dyn std::error::Error + Send + Sync>> {
        self.client.subscribe(subject.to_string()).await.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    pub async fn publish_book_update(&self, subject: &str, update: &BookUpdate) -> Result<(), async_nats::Error> {
        let proto_update = proto::BookUpdate {
            symbol: update.symbol.clone(),
            bids: update.bids.iter().map(|pl| proto::PriceLevel {
                price: pl.price,
                amount: pl.total_amount,
            }).collect(),
            asks: update.asks.iter().map(|pl| proto::PriceLevel {
                price: pl.price,
                amount: pl.total_amount,
            }).collect(),
            timestamp: update.last_update as u64,
        };
        let buf = proto_update.encode_to_vec();
        self.client.publish(subject.into(), buf.into()).await?;
        Ok(())
    }

    pub async fn publish_order(&self, subject: &str, order: &Order) -> Result<(), async_nats::Error> {
        let proto_order = proto::Order {
            id: order.id as u64,
            symbol: order.symbol.clone(),
            price: order.price,
            amount: order.amount,
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
        self.client.publish(subject.into(), buf.into()).await?;
        Ok(())
    }
}