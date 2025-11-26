use crate::level2::{Level2Error, LevelUpdated};
use crate::shared::{Period, Price, Quantity, Side, TimestampMS};

pub trait BookSide {
    fn total_quantity(&self, depth: usize) -> Quantity;
    fn level_quantity(&self, price: Price) -> Quantity;
    fn level_lifetime(&self, price: Price, period: Period) -> Option<TimestampMS>;
    fn level_average_quantity(&self, price: Price, period: Period) -> Quantity;
    fn level_total_added(&self, price: Price, period: Period) -> Quantity;
    fn level_total_cancelled(&self, price: Price, period: Period) -> Quantity;
    fn level_add_rate(&self, price: Price, period: Period) -> f32;
    fn level_cancel_rate(&self, price: Price, period: Period) -> f32;
    fn level_volume_spike(&self, price: Price, period: Period, threshold: f32) -> bool;
}

pub trait OrderBookFlowMetrics {
    fn bids(&self) -> &dyn BookSide;
    fn asks(&self) -> &dyn BookSide;

    fn update(&mut self, event: LevelUpdated) -> Result<(), Level2Error>;

    fn bid_ask_pressure(&self, depth: usize) -> f32;
}
