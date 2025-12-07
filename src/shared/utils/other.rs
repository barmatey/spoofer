use crate::shared::TimestampMS;

pub fn now_timestamp() -> TimestampMS {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as TimestampMS
}

pub fn now_timestamp_ns() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64
}


pub fn format_price(price: u64, decimals: usize) -> String {
    let factor = 10u64.pow(decimals as u32);
    let integer = price / factor;
    let fraction = price % factor;

    // Разделяем integer разрядов с запятой
    let integer_str = format!("{}", integer)
        .chars()
        .rev()
        .collect::<Vec<_>>()
        .chunks(3)
        .map(|chunk| chunk.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join(",");

    let integer_str = integer_str.chars().rev().collect::<String>();
    let frac_str = format!("{:0>decimals$}", fraction, decimals = decimals);
    let frac_str = &frac_str[..4.min(decimals)]; // показываем максимум 4 знака
    format!("{}.{}", integer_str, frac_str)
}