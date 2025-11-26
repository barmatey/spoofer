pub type Price = u64;
pub type Quantity = u64;

pub type TimestampMS = u64;

pub type Period = (TimestampMS, TimestampMS);

#[derive(Debug, Clone, PartialEq)]
pub enum Side {
    Buy,
    Sell,
}


