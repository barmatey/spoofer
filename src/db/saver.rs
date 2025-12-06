use crate::connector::Event;
use crate::db::errors::Error;
use crate::level2::{LevelUpdated, LevelUpdatedRepo};
use crate::shared::logger::Logger;
use crate::trade::{TradeEvent, TradeEventRepo};
use clickhouse::Client;
use tracing::Level;

trait Repository<T> {
    async fn save(&self, events: &[T]) -> Result<(), Error>;
}

impl<'a> Repository<LevelUpdated> for LevelUpdatedRepo<'a> {
    async fn save(&self, events: &[LevelUpdated]) -> Result<(), Error> {
        LevelUpdatedRepo::save(self, events).await?;
        Ok(())
    }
}

impl<'a> Repository<TradeEvent> for TradeEventRepo<'a> {
    async fn save(&self, events: &[TradeEvent]) -> Result<(), Error> {
        TradeEventRepo::save(self, events).await?;
        Ok(())
    }
}

struct BufferedSaver<T, R: Repository<T>> {
    buffer: Vec<T>,
    repo: R,
    buffer_size: usize,
}

impl<T, R: Repository<T>> BufferedSaver<T, R> {
    pub fn new(repo: R, buffer_size: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(buffer_size),
            repo,
            buffer_size,
        }
    }

    pub async fn push(&mut self, event: T) -> Result<(), Error> {
        self.buffer.push(event);

        if self.buffer.len() >= self.buffer_size {
            Logger::new("saver", Level::DEBUG)
                .debug(&format!("Saving {:?} records", self.buffer.len()));
            self.repo.save(&self.buffer).await?;
            self.buffer.clear();
        }

        Ok(())
    }

    pub async fn flush(&mut self) -> Result<(), Error> {
        if !self.buffer.is_empty() {
            self.repo.save(&self.buffer).await?;
            self.buffer.clear();
        }
        Ok(())
    }
}

pub struct SaverService<'a> {
    level_updated: BufferedSaver<LevelUpdated, LevelUpdatedRepo<'a>>,
    trades: BufferedSaver<TradeEvent, TradeEventRepo<'a>>,
}

impl<'a> SaverService<'a> {
    pub fn new(client: &'a Client, buffer_size: usize) -> Self {
        Self {
            level_updated: BufferedSaver::new(LevelUpdatedRepo::new(client), buffer_size),
            trades: BufferedSaver::new(TradeEventRepo::new(client), buffer_size),
        }
    }

    async fn save_level_updated(&mut self, event: LevelUpdated) -> Result<(), Error> {
        self.level_updated.push(event).await
    }

    async fn save_trade_event(&mut self, event: TradeEvent) -> Result<(), Error> {
        self.trades.push(event).await
    }

    pub async fn save(&mut self, event: Event) -> Result<(), Error> {
        match event {
            Event::LevelUpdate(ev) => self.save_level_updated(ev).await,
            Event::Trade(ev) => self.save_trade_event(ev).await,
        }
    }

    pub async fn flush_all(&mut self) -> Result<(), Error> {
        self.level_updated.flush().await?;
        self.trades.flush().await?;
        Ok(())
    }
}
