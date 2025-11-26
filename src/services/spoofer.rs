use crate::domain::events::LevelUpdated;

pub struct Spoofer{
    order_book:
}

impl Spoofer {
    pub fn new() -> Self {
        Self {}
    }
    
    pub fn handle_level_updated(&mut self, event: LevelUpdated) -> Result<(), ()>{
        
    }
}
