use crate::db::errors::Error;
use crate::level2::{LevelUpdated, LevelUpdatedRepo};
use crate::trade::{TradeEvent, TradeEventRepo};
use clickhouse::Client;

pub struct SaverService<'a> {
    level_updated_buffer: Vec<LevelUpdated>,
    level_updated_repo: LevelUpdatedRepo<'a>,
    trade_buffer: Vec<TradeEvent>,
    trade_repo: TradeEventRepo<'a>,
    buffer_size: usize,
}

impl<'a> SaverService<'a> {
    pub fn new(client: &'a &Client, buffer_size: usize) -> Self {
        Self {
            level_updated_repo: LevelUpdatedRepo::new(client),
            level_updated_buffer: vec![],
            trade_repo: TradeEventRepo::new(client),
            trade_buffer: vec![],
            buffer_size,
        }
    }

    async fn save_level_updated(&mut self, event: LevelUpdated) -> Result<(), Error> {
        self.level_updated_buffer.push(event);
        if self.level_updated_buffer.len() >= self.buffer_size {
            self.level_updated_repo
                .save(&self.level_updated_buffer)
                .await?;
            self.level_updated_buffer.clear();
        }
        Ok(())
    }

    async fn save_trade_event(&mut self, event: TradeEvent) -> Result<(), Error> {
        self.trade_buffer.push(event);
        if self.trade_buffer.len() >= self.buffer_size {
            self.trade_repo.save(&self.trade_buffer).await?;
            self.trade_buffer.clear();
        }
        Ok(())
    }
}
