use crate::level2::LevelUpdated;
use crate::shared::{Price, Quantity, Side, TimestampMS};
use crate::trade::TradeEvent;

pub trait OrderFlowMetrics {
    // ======================
    //  Handlers
    // ======================
    fn handle_level_update(&mut self, events: &[LevelUpdated]);
    fn handle_trade(&mut self, trades: &[TradeEvent]);

    // ======================
    //  Snapshots
    // ======================
    fn current_quantity(&self, price: Price, side: Side) -> Quantity;
    fn level_position(&self, price: Price, side: Side) -> Option<usize>;
    fn level_lifetime(&self, price: Price, side: Side) -> Option<TimestampMS>;

    /// Bid/Ask  imbalance throw depth levels
    fn book_pressure(&self, side: Side, depth: usize) -> f32;

    // ======================
    //  Stat Metrics
    // ======================
    fn avg_quantity(&self, price: Price, side: Side, period: TimestampMS) -> Quantity;
    fn total_added(&self, price: Price, side: Side, period: TimestampMS) -> Quantity;
    fn total_cancelled(&self, price: Price, side: Side, period: TimestampMS) -> Quantity;
    fn add_rate(&self, price: Price, side: Side, period: TimestampMS) -> f32;
    fn cancel_rate(&self, price: Price, side: Side, period: TimestampMS) -> f32;
    fn is_volume_spike(
        &self,
        price: Price,
        side: Side,
        period: TimestampMS,
        threshold: f32,
    ) -> bool;

    // ======================
    //  Trades
    // ======================
    fn executed_volume(&self, price: Price, side: Side, period: TimestampMS) -> Quantity;
    fn execution_ratio(&self, price: Price, side: Side, period: TimestampMS) -> f32;
}
