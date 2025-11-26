use crate::shared::{Period, Price, Quantity, Side};
use crate::trade::errors::TradeError;
use crate::trade::TradeEvent;

pub trait TradeStore {
    fn handle_trade(&mut self, trade: TradeEvent)  -> Result<(), TradeError>;

    fn level_executed(&self, price: Price, period: Period) -> Quantity;
    fn level_executed_bid(&self, price: Price, period: Period) -> Quantity;
    fn level_executed_ask(&self, price: Price, period: Period) -> Quantity;
    fn level_executed_side(&self, side: Side, price: Price, period: Period) -> Quantity;
}
