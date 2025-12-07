use crate::shared::errors::{check_exchange, check_ticker, check_timestamp};
use crate::shared::{Exchange, TimestampMS};
use crate::trade::{TradeError, TradeEvent};
use std::collections::VecDeque;
use std::sync::Arc;

pub struct TradeStore {
    exchange: Exchange,
    ticker: Arc<String>,
    trades: VecDeque<TradeEvent>,
    last_ts: TimestampMS,
    max_buffer: usize,
}

impl TradeStore {
    pub fn new(exchange: Exchange, ticker: Arc<String>, max_buffer: usize) -> Self {
        Self {
            exchange,
            ticker,
            last_ts: 0,
            trades: VecDeque::with_capacity(max_buffer),
            max_buffer,
        }
    }
    pub fn update(&mut self, trade: TradeEvent) -> Result<(), TradeError> {
        check_timestamp(self.last_ts, trade.timestamp)?;
        check_exchange(&trade.exchange, &self.exchange)?;
        check_ticker(&trade.ticker, &self.ticker)?;

        self.last_ts = trade.timestamp;
        self.trades.push_back(trade);

        if self.trades.len() > self.max_buffer {
            self.trades.pop_front();
        }

        Ok(())
    }
    pub fn update_if_instrument_matches(&mut self, trade: TradeEvent) -> Result<(), TradeError> {
        if self.ticker == trade.ticker && self.exchange == trade.exchange {
            self.update(trade)?;
        }
        Ok(())
    }

    pub fn update_or_miss(&mut self, trade: TradeEvent) {
        let _ = self.update_if_instrument_matches(trade);
    }

    pub fn trades(&self) -> &VecDeque<TradeEvent> {
        &self.trades
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::{Exchange, Side};

    fn sample_trade(ts: TimestampMS, exchange: Exchange, ticker: &str) -> TradeEvent {
        TradeEvent {
            exchange,
            ticker: Arc::new(ticker.to_string()),
            timestamp: ts,
            price: 100,
            quantity: 10,
            market_maker: Side::Buy,
        }
    }

    fn trade_store() -> TradeStore {
        TradeStore::new(
            Exchange::Binance,
            Arc::new("btc/usdt".to_string()),
            100,
        )
    }

    #[test]
    fn test_update_adds_trade_in_order() {
        let mut store = trade_store();
        let t1 = sample_trade(1, Exchange::Binance, "btc/usdt");
        let t2 = sample_trade(2, Exchange::Binance, "btc/usdt");

        store.update(t1).unwrap();
        store.update(t2).unwrap();

        assert_eq!(store.trades.len(), 2);
        assert!(store.trades[0].timestamp < store.trades[1].timestamp);
    }

    #[test]
    fn test_update_rejects_wrong_exchange() {
        let mut store = trade_store();
        let trade = sample_trade(1, Exchange::Kraken, "btc/usdt");

        assert!(store.update(trade).is_err());
        assert!(store.trades.is_empty());
    }

    #[test]
    fn test_update_rejects_wrong_ticker() {
        let mut store = trade_store();
        let trade = sample_trade(1, Exchange::Binance, "eth/usdt");

        assert!(store.update(trade).is_err());
        assert!(store.trades.is_empty());
    }

    #[test]
    fn test_update_rejects_non_monotonic_timestamp() {
        let mut store = trade_store();
        let t1 = sample_trade(2, Exchange::Binance, "btc/usdt");
        let t2 = sample_trade(1, Exchange::Binance, "btc/usdt");

        store.update(t1).unwrap();
        assert!(store.update(t2).is_err());
        assert_eq!(store.trades.len(), 1);
    }

    #[test]
    fn test_update_if_instrument_matches_adds_only_matching() {
        let mut store = trade_store();
        let matching = sample_trade(1, Exchange::Binance, "btc/usdt");
        let non_matching = sample_trade(2, Exchange::Kraken, "btc/usdt");

        store.update_if_instrument_matches(matching).unwrap();
        store.update_if_instrument_matches(non_matching).unwrap();

        assert_eq!(store.trades.len(), 1);
        assert_eq!(store.trades[0].exchange, Exchange::Binance);
    }

    #[test]
    fn test_update_or_miss_ignores_non_matching() {
        let mut store = trade_store();
        let matching = sample_trade(1, Exchange::Binance, "btc/usdt");
        let non_matching = sample_trade(2, Exchange::Kraken, "btc/usdt");

        store.update_or_miss(matching);
        store.update_or_miss(non_matching);

        assert_eq!(store.trades.len(), 1);
        assert_eq!(store.trades[0].exchange, Exchange::Binance);
    }
}
