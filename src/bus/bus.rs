use crate::bus::Event;
use arc_swap::ArcSwap;
use crossbeam::queue::SegQueue;
use std::any::TypeId;
use std::collections::HashMap;
use std::sync::Arc;

type Subscriber<T> = fn(&T);

type SubWrapper = Arc<dyn Fn(&dyn Event) + Send + Sync>;

pub struct Bus {
    subs: ArcSwap<HashMap<TypeId, Arc<[SubWrapper]>>>,
    events: SegQueue<Arc<dyn Event + Send + Sync>>,
}

impl Bus {
    pub fn new() -> Self {
        Self {
            subs: ArcSwap::new(Arc::new(HashMap::new())),
            events: SegQueue::new(),
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

        // RCU-паттерн: читаем-модифицируем-заменяем
        loop {
            let current = self.subs.load();

            let mut new_map = (**current).clone();
            let entry = new_map.entry(type_id).or_insert_with(|| Arc::new([]));
            let mut new_subscribers: Vec<_> = entry.as_ref().to_vec();
            new_subscribers.push(wrapped_subscriber.clone());
            new_map.insert(type_id, Arc::from(new_subscribers));

            match self.subs.compare_and_swap(&current, Arc::new(new_map)) {
                old if Arc::ptr_eq(&old, &current) => break,
                _ => continue,
            }
        }
    }
    pub fn process_events(&self) {
        while let Some(event) = self.events.pop() {
            let type_id = event.as_any().type_id();

            // Lock-free чтение!
            let subscribers_map = self.subs.load();
            if let Some(subscribers) = subscribers_map.get(&type_id) {
                for subscriber in subscribers.iter() {
                    subscriber(&*event);
                }
            }
        }
    }

    pub async fn processing(&self) {
        loop {
            self.process_events();
            tokio::task::yield_now().await;
        }
    }
}
