# Arbitrage System — Quick Reference

Short, practical guide to understand and use the Rust-based streaming + saver + arbitrage pipeline. Focused on user workflows and concrete code snippets so you can jump straight into the system.

---

## Subscribe data stream

**Purpose / user story:** get live trade and L2 (order book level) updates from configured exchanges and tickers and publish them onto an in-process broadcast channel so background workers can consume them.

**Event models** (what you will receive from the connector):

```rust
pub struct TradeEvent {
    pub exchange: Exchange,
    pub ticker: Arc<String>,
    pub price: Price,
    pub quantity: Quantity,
    pub timestamp: TimestampMS,
    pub received: TimestampNS,
    pub market_maker: Side,
}

pub struct LevelUpdated {
    pub exchange: Exchange,
    pub ticker: Arc<String>,
    pub side: Side,
    pub price: Price,
    pub quantity: Quantity,
    pub timestamp: TimestampMS,
    pub received: TimestampNS,
}
```

The connector emits a simple `Event` enum (consumed by the app):

```rust
// pattern seen in code (actual enum lives in connector module)
match event {
    Event::Trade(v) => {/* TradeEvent */},
    Event::LevelUpdate(v) => {/* LevelUpdated */},
}
```

**Subscription example** — how the stream is built and pushed into a `broadcast` channel:

```rust
static TICKERS: [(&'static str, u32, u32); 4] = [
    ("btc/usdt", 100, 1_000_000),
    ("eth/usdt", 100, 10_000),
    ("sol/usdt", 1000, 10_000),
    ("bnb/usdt", 1000, 10_000),
];

async fn stream(tx_events: broadcast::Sender<Event>) {
    let mut stream = StreamConnector::new()
        .exchanges(&[Exchange::Binance, Exchange::Kraken])
        .tickers(&TICKERS)
        .subscribe_depth(10)
        .subscribe_trades()
        .log_level_info()
        .connect()
        .await
        .unwrap();

    loop {
        let event = stream.next().await.unwrap();
        tx_events.send(event).unwrap();
    }
}
```

**Explanation of numbers next to tickers:**
- The first number (e.g., 100 for btc/usdt) is a `price multiplier`: prices are scaled by this factor to store as integers instead of floats for precision.
- The second number (e.g., 1_000_000) is a `quantity multiplier`: trade quantities are scaled similarly to avoid floating-point rounding errors.
- This allows the system to handle fractional prices and volumes precisely without floating-point inaccuracies.

**Notes / best practices:**
- Keep the `broadcast` buffer large enough for peak events (example uses `50_000`).
- Prefer lightweight event structs (Arc\<String\> for ticker avoids clones).
- `subscribe_depth(10)` configures L2 depth to maintain for each book.
- Write tickers in lowercase using / as a delimiter. They will later be automatically converted to each exchange's specific format.

---

## Save events

**Purpose / user story:** persist incoming events in batches to ClickHouse using `BufferService` and repository objects.

**Relevant snippet** — saver worker:

```rust
async fn get_client() -> Client {
    let client = DatabaseClient::default()
        .with_url("http://127.0.0.1:8123")
        .with_user("default")
        .with_password("")
        .with_database("spoofer")
        .build()
        .await
        .unwrap();
    client
}

async fn saver(mut rx_events: broadcast::Receiver<Event>) {
    let client = get_client().await;
    let trade_saver = BufferService::new(TradeEventRepo::new(&client), 10_000);
    let level2saver = BufferService::new(LevelUpdatedRepo::new(&client), 50_000);
    loop {
        let event = rx_events.recv().await.unwrap();
        match event {
            Event::Trade(v) => trade_saver.push(v).await.unwrap(),
            Event::LevelUpdate(v) => level2saver.push(v).await.unwrap(),
        };
    }
}
```

**Notes:**
- `BufferService` batches and flushes to the repo for throughput. Tune batch sizes to trade volume and ClickHouse write throughput.
- Repos (`TradeEventRepo`, `LevelUpdatedRepo`) encapsulate schema and insert logic — keep them small and stable.
- On errors, prefer to log + backoff rather than panic in production; the example uses `.unwrap()` for clarity.

---

## Check arbitrage opportunities

**Purpose / user story:** maintain an in-memory order-book per exchange and ticker, compare book tops to detect cross-exchange spreads that exceed a configured threshold, emit `ArbitrageSignal` events and persist them.

**How it works (flow):**
1. Two `OrderBook` instances are created per ticker (one per exchange).
2. On each `LevelUpdated` event, update both books via `update_or_miss`.
3. Run `ArbitrageMonitor::new(&book_a, &book_b, threshold).execute()`.
4. If a `Signal` is returned, handle it.

**Processor snippet:**

```rust
async fn processor(mut rx_events: broadcast::Receiver<Event>) {
    let mut books = vec![];
    for (ticker, _, _) in TICKERS.iter() {
        let ob1 = OrderBook::new(Exchange::Binance, ticker, 10);
        let ob2 = OrderBook::new(Exchange::Kraken, ticker, 10);
        books.push((ob1, ob2));
    }

    loop {
        let event = rx_events.recv().await.unwrap();
        match event {
            Event::Trade(_v) => {}
            Event::LevelUpdate(v) => {
                for pair in books.iter_mut() {
                    pair.0.update_or_miss(&v);
                    pair.1.update_or_miss(&v);
                    let signal = ArbitrageMonitor::new(&pair.0, &pair.1, 0.0002).execute();
                    if let Some(s) = signal {
                        println!("{:?}", s);
                    }
                }
            }
        }
    }
}
```

**Model of arbitrage signal:**
- The monitor compares best bid/ask between two books and returns a `Signal` when the spread > `threshold` (example `0.0002` = 0.02%).
- You should include fees, slippage and transfer costs in production thresholds. The example assumes zero transfer cost.

```rust
pub struct ArbitrageLeg {
    pub exchange: Exchange,
    pub ticker: Arc<String>,
    pub price: Price,
}

pub struct ArbitrageSignal {
    pub buy: ArbitrageLeg,
    pub sell: ArbitrageLeg,
    pub profit_pct: f32,
    pub profit_abs: Option<f32>,
    pub timestamp: TimestampMS,
}
```

---

## Full example: application wiring

```rust
mod connector;
mod db;
mod level2;
mod shared;
mod signal;
mod trade;

use clickhouse::Client;
use crate::connector::{Event, StreamConnector};
use crate::level2::{LevelUpdatedRepo, OrderBook};
use crate::shared::utils::buffer_service::BufferService;
use crate::shared::Exchange;
use crate::signal::arbitrage_monitor::{ArbitrageMonitor, ArbitrageSignalRepo};
use crate::trade::TradeEventRepo;
use db::DatabaseClient;
use futures_util::StreamExt;
use tokio::sync::broadcast;

static TICKERS: [(&'static str, u32, u32); 4] = [
    ("btc/usdt", 100, 1_000_000),
    ("eth/usdt", 100, 10_000),
    ("sol/usdt", 1000, 10_000),
    ("bnb/usdt", 1000, 10_000),
];

async fn get_client() -> Client{
    let client = DatabaseClient::default()
        .with_url("http://127.0.0.1:8123")
        .with_user("default")
        .with_password("")
        .with_database("spoofer")
        .build()
        .await
        .unwrap();
    client
}

async fn stream(tx_events: broadcast::Sender<Event>) {
    let mut stream = StreamConnector::new()
        .exchanges(&[Exchange::Binance, Exchange::Kraken])
        .tickers(&TICKERS)
        .subscribe_depth(10)
        .subscribe_trades()
        .log_level_info()
        .connect()
        .await
        .unwrap();
    loop {
        let event = stream.next().await.unwrap();
        tx_events.send(event).unwrap();
    }
}

async fn saver(mut rx_events: broadcast::Receiver<Event>) {
    let client = get_client().await;
    let trade_saver = BufferService::new(TradeEventRepo::new(&client), 10_000);
    let level2saver = BufferService::new(LevelUpdatedRepo::new(&client), 50_000);
    loop {
        let event = rx_events.recv().await.unwrap();
        match event {
            Event::Trade(v) => trade_saver.push(v).await.unwrap(),
            Event::LevelUpdate(v) => level2saver.push(v).await.unwrap(),
        };
    }
}

async fn processor(mut rx_events: broadcast::Receiver<Event>) {
    let mut books = vec![];
    for (ticker, _, _) in TICKERS.iter() {
        let ob1 = OrderBook::new(Exchange::Binance, ticker, 10);
        let ob2 = OrderBook::new(Exchange::Kraken, ticker, 10);
        books.push((ob1, ob2));
    }
    loop {
        let event = rx_events.recv().await.unwrap();
        match event {
            Event::Trade(_v) => {
                println!("{:?}", s);
            }
            Event::LevelUpdate(v) => {
                for pair in books.iter_mut() {
                    pair.0.update_or_miss(&v);
                    pair.1.update_or_miss(&v);
                    let signal = ArbitrageMonitor::new(&pair.0, &pair.1, 0.0002).execute();
                    if let Some(s) = signal {
                        println!("{:?}", s);
                    }
                }
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let (tx_events, _) = broadcast::channel::<Event>(50_000);

    // Stream tread
    let stream_tx = tx_events.clone();
    let handle_stream = tokio::spawn(async move {
        stream(stream_tx).await;
    });

    // Saver thread
    let saver_rx = tx_events.subscribe();
    let handle_saver = tokio::spawn(async move {
        saver(saver_rx).await;
    });

    // Arbitrage tread
    let processor_rx = tx_events.subscribe();
    let handle_processor = tokio::spawn(async move {
        processor(processor_rx).await;
    });

    tokio::select! {
        res = handle_stream => println!("handle_stream: {:?}", res),
        res = handle_saver => println!("handle_saver: {:?}", res),
        res = handle_processor => println!("handle_processor: {:?}", res),
    }
}

```

**Deployment notes:**
- Run with `RUST_LOG=info` to surface connector logs.
- Monitor consumer lag (broadcast buffer) and ClickHouse insert latencies.
- Consider splitting saver and processor into separate processes if CPU / memory becomes a bottleneck.

---

## Quick checklist before production
- Add error handling (avoid `.unwrap()` in long-running tasks).
- Account for fees/slippage in arbitrage threshold.
- Add backpressure or drop policy for the stream when DB is down.
- Add metrics (throughput, latency, signal count) and alerting.

