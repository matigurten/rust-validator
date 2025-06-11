# Rust Order Book Project

A high-performance order book implementation in Rust with NATS messaging integration.

## Features

- Real-time order book management
- Support for Market, Limit, and Cancel orders
- NATS messaging integration for order distribution
- Low-latency order processing
- Price level aggregation

## Prerequisites

- Rust (latest stable version)
- NATS server running locally (default: localhost:4222)

## Building

```bash
cargo build
```

## Running

```bash
cargo run --bin feed_handler
```

## Project Structure

- `src/orderbook.rs` - Order book implementation
- `src/messaging.rs` - NATS messaging integration
- `src/utils.rs` - Utility functions
- `src/main.rs` - Main application entry point

## License

MIT