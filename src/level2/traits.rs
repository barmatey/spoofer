use crate::level2::{Level2Error, LevelUpdated};
use crate::shared::{Period, Price, Quantity, Side, TimestampMS};

pub trait OrderBookFlowMetrics {
    fn handle_level_updated(&mut self, event: LevelUpdated) -> Result<(), Level2Error>;

    // ======================
    //  Snapshots
    // ======================
    fn current_quantity(&self, price: Price, side: Side) -> Quantity;
    fn book_pressure(&self, side: Side, depth: usize) -> f32;

    // ======================
    //  Stat Metrics
    // ======================
    fn level_lifetime(&self, price: Price, side: Side, period: Period) -> Option<TimestampMS>;
    fn avg_quantity(&self, price: Price, side: Side, period: Period) -> Quantity;
    fn total_added(&self, price: Price, side: Side, period: Period) -> Quantity;
    fn total_cancelled(&self, price: Price, side: Side, period: Period) -> Quantity;
    fn add_rate(&self, price: Price, side: Side, period: Period) -> f32;
    fn cancel_rate(&self, price: Price, side: Side, period: Period) -> f32;
    fn is_volume_spike(&self, price: Price, side: Side, period: Period, threshold: f32) -> bool;
}
