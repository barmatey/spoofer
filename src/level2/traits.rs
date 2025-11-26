use crate::level2::{Level2Error, LevelUpdated};
use crate::shared::{Period, Price, Quantity, Side, TimestampMS};

pub trait OrderBookFlowMetrics {
    fn handle_level_updated(&mut self, event: LevelUpdated) -> Result<(), Level2Error>;

    // ======================
    //  Snapshots
    // ======================
    fn quantity(&self, price: Price, side: Side) -> Quantity;
    fn book_pressure(&self, side: Side, depth: usize) -> f32;

    // ======================
    //  Stat Metrics
    // ======================
    fn level_lifetime(&self, price: Price, side: Side, period: Period) -> Option<TimestampMS>;
    fn level_avg_quantity(&self, price: Price, side: Side, period: Period) -> Quantity;
    fn level_total_added(&self, price: Price, side: Side, period: Period) -> Quantity;
    fn level_total_cancelled(&self, price: Price, side: Side, period: Period) -> Quantity;
    fn level_add_rate(&self, price: Price, side: Side, period: Period) -> f32;
    fn level_cancel_rate(&self, price: Price, side: Side, period: Period) -> f32;
    fn level_volume_spike(&self, price: Price, side: Side, period: Period, threshold: f32) -> bool;
}
