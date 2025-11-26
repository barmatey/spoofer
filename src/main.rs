use crate::connector::{BinanceConnector, BinanceConnectorConfig, Connector};
use std::time::Duration;
use crate::level2::{display_order_book, BookStats, OrderBookRealization, Snap};
use crate::shared::{Bus, Side};

mod connector;
mod shared;
mod signal;

mod trade;
mod level2;

#[tokio::main]
async fn main() {
    let symbol = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "btcusdt".to_string());

    let bus = Bus::new();

    let worker = async || {
        let mut order_book = OrderBookRealization::new();
        let mut order_stat = BookStats::new(10);

        loop {
            let events = bus.levels.pull();

            order_book.update(&events);

            for s in events
                .iter()
                .map(|x| Snap{
                    side: x.side.clone(),
                    quantity: x.quantity,
                    timestamp: x.timestamp,
                    level: order_book.get_position(&x.side, x.price).unwrap_or(0)
                })
                .filter(|x| x.level < 10)
            {
                order_stat.push(s).unwrap();
            }

            display_order_book(&order_book, 10);
            println!();
            println!("Average in bid 0: {}", order_stat.get_average_quantity(Side::Buy, 0, 30_000).unwrap());
            tokio::time::sleep(Duration::from_millis(2_000)).await;
        }
    };

    let mut connector = BinanceConnector::new(
        &bus,
        BinanceConnectorConfig {
            ticker: symbol,
            price_multiply: 100,
            quantity_multiply: 10_000_000,
        },
    );

    tokio::join!(connector.listen(), worker(),);
}
