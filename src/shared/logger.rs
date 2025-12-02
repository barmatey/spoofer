use tracing::Level;

/// Цвета ANSI
pub const RED: &str = "\x1b[31m";
pub const GREEN: &str = "\x1b[32m";
pub const YELLOW: &str = "\x1b[33m";
pub const BLUE: &str = "\x1b[34m";
pub const RESET: &str = "\x1b[0m";

/// Функция для получения цвета по уровню
pub fn color_for_level(level: Level) -> &'static str {
    match level {
        Level::ERROR => RED,
        Level::WARN => YELLOW,
        Level::INFO => GREEN,
        Level::DEBUG => BLUE,
        Level::TRACE => BLUE,
    }
}

pub struct Logger {
    pub name: &'static str,
    level: Level, // минимальный уровень логирования
}

impl Logger {
    pub fn new(name: &'static str, level: Level) -> Self {
        Self {
            name,
            level,
        }
    }

    fn log(&self, level: Level, msg: &str) {
        if level <= self.level {
            let colored_level = format!("{}{}{}", color_for_level(level), level, RESET);
            println!("[{}]: {} {}", self.name, colored_level, msg);
        }
    }

    pub fn info(&self, msg: &str) {
        self.log(Level::INFO, msg);
    }

    pub fn warn(&self, msg: &str) {
        self.log(Level::WARN, msg);
    }

    pub fn error(&self, msg: &str) {
        self.log(Level::ERROR, msg);
    }

    pub fn debug(&self, msg: &str) {
        self.log(Level::DEBUG, msg);
    }
}
