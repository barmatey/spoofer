use tracing_subscriber;

pub fn init_logging() {
    // compact() — красивый компактный формат
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .compact()
        .init();
}

pub struct Logger {
    name: &'static str,
}

impl Logger {
    pub fn new(name: &'static str) -> Self {
        Self { name }
    }

    pub fn info(&self, msg: &str) {
        tracing::info!(exchange = self.name, "{msg}");
    }

    pub fn error(&self, err: &str) {
        tracing::error!(exchange = self.name, "{err}");
    }
}
