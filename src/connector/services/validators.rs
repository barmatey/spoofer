use crate::connector::errors::Error;
use crate::connector::errors::Error::OtherError;
use std::collections::HashSet;

pub fn check_symbol_exist(
    exchange: &str,
    symbol: &str,
    valid_symbols: &HashSet<String>,
) -> Result<(), Error> {
    if valid_symbols.contains(symbol) {
        Ok(())
    } else {
        let mut available = valid_symbols.iter().cloned().collect::<Vec<_>>();
        available.sort();
        let available = available.join(", ");

        let err = OtherError(format!(
            "Symbol {} is not in {} available symbols. Available values: {}",
            symbol, exchange, available
        ));
        Err(err)
    }
}
