use crate::level2::OrderBook;

pub fn display_books(books: &[&OrderBook]) {
    // Перемещаем курсор в верхний левый угол и очищаем экран
    print!("\x1b[H\x1b[2J");
    println!("{:<12} {:>12} {:>12}", "Exchange", "Best Bid", "Best Ask");
    println!("{}", "-".repeat(38));

    for book in books {
        let bid = if book.bids().is_empty() {
            "-".to_string()
        } else {
            format!("{}", book.bids().best_price())
        };

        let ask = if book.asks().is_empty() {
            "-".to_string()
        } else {
            format!("{}", book.asks().best_price())
        };

        println!("{:<12} {:>12} {:>12}", book.exchange(), bid, ask);
    }

    std::io::Write::flush(&mut std::io::stdout()).unwrap();
}
