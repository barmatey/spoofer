use crate::level2::OrderBook;

fn format_price(price: u64, decimals: usize) -> String {
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


pub fn display_books(books: &[&OrderBook], decimals: usize) {
    print!("\x1b[H\x1b[2J");
    println!("{:<12} {:>12} {:>12}", "Exchange", "Best Bid", "Best Ask");
    println!("{}", "-".repeat(38));

    for book in books {
        let bid = if book.bids().is_empty() {
            "-".to_string()
        } else {
            format!("{}", format_price(book.bids().best_price(), decimals))
        };

        let ask = if book.asks().is_empty() {
            "-".to_string()
        } else {
            format!("{}", format_price(book.asks().best_price(), decimals))
        };

        println!("{:<12} {:>12} {:>12}", book.exchange(), bid, ask);
    }

    std::io::Write::flush(&mut std::io::stdout()).unwrap();
}
