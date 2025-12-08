use std::sync::Arc;
use crate::shared::{Exchange, Price, TimestampMS};

#[derive(Debug, Clone)]
pub struct ArbitrageLeg {
    pub exchange: Exchange,
    pub ticker: Arc<String>,
    pub price: Price,
}

#[derive(Debug, Clone)]
pub struct ArbitrageSignal {
    pub buy: ArbitrageLeg,
    pub sell: ArbitrageLeg,
    pub profit_pct: f32,
    pub profit_abs: f32,
    pub timestamp: TimestampMS,
}