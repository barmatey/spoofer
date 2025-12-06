use clickhouse::Client;
use crate::db::errors::Error;
use crate::level2::{LevelUpdated, LevelUpdatedRepo};

pub struct SaverService<'a>{
    level_updated_buffer: Vec<LevelUpdated>,
    level_updated_repo: LevelUpdatedRepo<'a>,
    buffer_size:  usize,
}

impl<'a> SaverService<'a>{
    pub fn new(client: &'a &Client, buffer_size: usize) -> Self{
        Self{
            level_updated_repo: LevelUpdatedRepo::new(client),
            level_updated_buffer: vec![],
            buffer_size,
        }
    }

    async fn save_level_updated(&mut self, event: LevelUpdated) -> Result<(), Error> {
        self.level_updated_buffer.push(event);
        if self.level_updated_buffer.len() >= self.buffer_size{
            self.level_updated_repo.save(&self.level_updated_buffer).await?;
            self.level_updated_buffer.clear();
        }
        Ok(())

    }
}