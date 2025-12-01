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
        let err = OtherError(format!(
            "Symbol {} is not in {} available symbols",
            symbol, exchange
        ));
        Err(err)
    }
}
