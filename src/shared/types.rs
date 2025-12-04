pub type Price = u64;
pub type Bid = Price;
pub type Ask = Price;
pub type Spread = Price;
pub type Quantity = u64;

pub type TimestampMS = u64;

pub type Period = (TimestampMS, TimestampMS);

pub type Profit = i64;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Side {
    Buy,
    Sell,
}
