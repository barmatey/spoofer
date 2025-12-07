#[derive(PartialEq, Clone, Debug)]
pub enum Exchange {
    Binance = 0,
    Kraken = 1,
}

impl Exchange {
    pub fn to_str(&self) -> &'static str {
        match self {
            Exchange::Binance => "binance",
            Exchange::Kraken => "kraken",
        }
    }
}
