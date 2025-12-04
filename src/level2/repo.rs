use clickhouse::insert::Insert;
use crate::level2::{Level2Error, LevelUpdated};
use clickhouse::Client;

pub struct LevelUpdatedRepo<'a> {
    client:&'a Client,
    buffer_size: usize,
    buffer: Vec<LevelUpdated>,
}

impl<'a> LevelUpdatedRepo<'a> {
    pub fn new(client:&'a Client, buffer_size: usize) -> Self {
        Self {
            client,
            buffer_size,
            buffer: Vec::with_capacity(buffer_size + 1),
        }
    }

    pub fn add_one(&mut self, event: LevelUpdated) {
        self.buffer.push(event);
    }
    
    pub async fn save_if_full(&mut self)-> Result<(), Level2Error>{
        if self.buffer.len() >= self.buffer_size {
            self.save().await?
        }
        Ok(())
    }

    pub async fn save(&mut self) -> Result<(), Level2Error> {
        if self.buffer.is_empty() {
            return Ok(());
        }
        let mut insert: Insert<(String, String, u8, u64, u64, u64)> =
            self.client.insert("level_updates").await?;

        for event in self.buffer.iter() {
            insert.write(&(
                event.exchange.to_string(),
                event.ticker.to_string(),
                event.side as u8,
                event.price,
                event.quantity,
                event.timestamp,
            )).await?;
        }

        insert.end().await?;
        self.buffer.clear();
    Ok(())
    }
}
