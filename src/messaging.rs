use nats::Connection;
use serde_json;
use crate::orderbook::{Order, BookUpdate};

pub struct NatsClient {
    client: Connection,
}

impl NatsClient {
    pub fn new(url: &str) -> Result<Self, std::io::Error> {
        let client = nats::connect(url)?;
        Ok(NatsClient { client })
    }

    pub fn subscribe(&self, subject: &str) -> Result<nats::Subscription, std::io::Error> {
        Ok(self.client.subscribe(subject)?)
    }

    pub fn publish_order(&self, subject: &str, order: &Order) -> Result<(), std::io::Error> {
        let data = serde_json::to_vec(order)?;
        self.client.publish(subject, &data)?;
        Ok(())
    }

    pub fn publish_book_update(&self, subject: &str, update: &BookUpdate) -> Result<(), std::io::Error> {
        let data = serde_json::to_vec(update)?;
        self.client.publish(subject, &data)?;
        Ok(())
    }

    pub fn publish_error(&self, subject: &str, error: &str) -> Result<(), std::io::Error> {
        self.client.publish(subject, error.as_bytes())?;
        Ok(())
    }
} 