use crate::level2::OrderBook;
use crate::shared::utils::format_price;

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
