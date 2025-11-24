use crate::bus::Event;
use std::any::TypeId;
use std::sync::Arc;
use crossbeam::queue::SegQueue;
use dashmap::DashMap;

type Subscriber<T> = fn(&T);

pub struct Bus {
    subs: Arc<DashMap<TypeId, Vec<Arc<dyn Fn(&dyn Event) + Send + Sync>>>>,
    events: Arc<SegQueue<Arc<dyn Event + Send + Sync>>>,
}

impl Bus {
    pub fn new() -> Self {
        Self {
            subs: Arc::new(DashMap::new()),
            events: Arc::new(SegQueue::new()),
        }
    }

    pub fn publish(&self, event: Arc<dyn Event + Send + Sync>) {
        self.events.push(event);
    }

    pub fn subscribe<T: Event + 'static>(&self, subscriber: Subscriber<T>) {
        let type_id = TypeId::of::<T>();

        let wrapped_subscriber = Arc::new(move |event: &dyn Event| {
            if let Some(concrete_event) = event.as_any().downcast_ref::<T>() {
                subscriber(concrete_event);
            } else {
                panic!("Unexpected event type");
            }
        });

        self.subs
            .entry(type_id)
            .or_insert_with(Vec::new)
            .push(wrapped_subscriber);
    }

    pub fn process_events(&self) {
        while let Some(event) = self.events.pop() {
            let type_id = event.as_any().type_id();

            if let Some(subscribers) = self.subs.get(&type_id) {
                for subscriber in subscribers.iter() {
                    subscriber(&*event);
                }
            }
        }
    }

    pub async fn processing(&self) {
        // Асинхронная обработка событий
        loop {
            self.process_events();
            tokio::task::yield_now().await;
        }
    }
}

// Реализация клонирования для Bus
impl Clone for Bus {
    fn clone(&self) -> Self {
        Self {
            subs: Arc::clone(&self.subs),
            events: Arc::clone(&self.events),
        }
    }
}