use serde::Deserialize;
use spoofing::level2::{LevelUpdated, OrderBook};
use spoofing::shared::{Price, Quantity, Side, TimestampMS};
use spoofing::trade::{TradeEvent, TradeStore};
use std::fs;
use std::path::Path;

#[derive(Deserialize, Debug)]
struct FakeTradeJson {
    market_maker: String,
    price: Price,
    quantity: Quantity,
    timestamp: TimestampMS,
}

#[derive(Deserialize, Debug)]
struct FakeLevelUpdatedJSON {
    price: Price,
    quantity: Quantity,
    timestamp: TimestampMS,
    side: String,
}

fn get_order_book() -> OrderBook {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/signal/spoofer/bids.json");
    let data = fs::read_to_string(path).expect("Failed to read ./bids.json");

    let jsons: Vec<FakeLevelUpdatedJSON> =
        serde_json::from_str(&data).expect("Failed to parse JSON");

    let mut ob = OrderBook::new();

    for fake in jsons {
        let event = LevelUpdated {
            side: Side::Buy,
            price: fake.price,
            timestamp: fake.timestamp,
            quantity: fake.quantity,
        };
        ob.update(event).unwrap();
    }
    ob
}

fn get_trade_store() -> TradeStore {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/signal/spoofer/trades.json");
    let data = fs::read_to_string(path).expect("Failed to read ./trades.json");
    let jsons: Vec<FakeTradeJson> = serde_json::from_str(&data).expect("Failed to parse JSON");

    let mut store = TradeStore::new();
    for fake in jsons {
        let event = TradeEvent {
            price: fake.price,
            quantity: fake.quantity,
            market_maker: Side::Buy,
            timestamp: fake.timestamp,
        };
        store.handle_trade(event).unwrap();
    }
    store
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foo() {

    }
}
