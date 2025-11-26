use crossbeam::queue::SegQueue;
use crate::level2::LevelUpdated;
use crate::trade::TradeEvent;

pub struct Topic<T: Send + Sync> {
    events: SegQueue<T>,
}

impl<T: Send + Sync> Topic<T> {
    pub fn new() -> Self {
        Self {
            events: SegQueue::new(),
        }
    }

    pub fn publish(&self, event: T) {
        self.events.push(event);
    }

    pub fn pull(&self) -> Vec<T> {
        let mut result = Vec::with_capacity(self.events.len());
        while let Some(event) = self.events.pop() {
            result.push(event);
        }
        result
    }
}

pub struct Bus {
    pub levels: Topic<LevelUpdated>,
    pub trades: Topic<TradeEvent>,
}

impl Bus {
    pub fn new() -> Self {
        Self {
            levels: Topic::new(),
            trades: Topic::new(),
        }
    }
}
