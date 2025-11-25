use crate::bus::Event;
use arc_swap::ArcSwap;
use crossbeam::queue::SegQueue;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Arc;

pub struct Bus {
    /// Вектор всех событий по типам
    snapshots: ArcSwap<HashMap<TypeId, Arc<Vec<Arc<dyn Any + Send + Sync>>>>>,
    /// Очередь для входящих событий
    queue: SegQueue<Arc<dyn Event + Send + Sync>>,
    /// Offset каждого подписчика
    offsets: ArcSwap<HashMap<u16, HashMap<TypeId, usize>>>,
    /// Генератор ID для подписчиков
    next_sub_id: AtomicU16,
}

impl Bus {
    pub fn new() -> Self {
        Self {
            snapshots: ArcSwap::new(Arc::new(HashMap::new())),
            queue: SegQueue::new(),
            offsets: ArcSwap::new(Arc::new(HashMap::new())),
            next_sub_id: AtomicU16::new(1),
        }
    }

    /// Публикация события
    pub fn publish<T: Event + Send + Sync>(&self, event: T) {
        self.queue.push(Arc::new(event));
    }

    /// Добавление подписчика, возвращает уникальный sub_id
    pub fn subscribe<T: Event + 'static>(&self) -> u16 {
        let sub_id = self.next_sub_id.fetch_add(1, Ordering::Relaxed);

        loop {
            let current = self.offsets.load();
            let mut new_map = (**current).clone();
            new_map.insert(sub_id, HashMap::new()); // новый подписчик с пустыми offset

            match self.offsets.compare_and_swap(&current, Arc::new(new_map)) {
                old if Arc::ptr_eq(&old, &current) => break,
                _ => continue,
            }
        }

        sub_id
    }

    /// Pull событий для подписчика начиная с его offset
    pub fn pull<T: Event + 'static>(&self, sub_id: u16) -> Result<Vec<Arc<T>>, ()> {
        let wanted = TypeId::of::<T>();

        // Переносим все события из очереди в snapshots
        while let Some(event) = self.queue.pop() {
            let eid = (*event).type_id();

            loop {
                let current_snap = self.snapshots.load();
                let mut new_snap = (**current_snap).clone();

                let entry = new_snap.entry(eid).or_insert_with(|| Arc::new(Vec::new()));
                let mut vec_copy = entry.as_ref().clone();
                vec_copy.push(event.clone());
                new_snap.insert(eid, Arc::new(vec_copy));

                if Arc::ptr_eq(
                    &self
                        .snapshots
                        .compare_and_swap(&current_snap, Arc::new(new_snap)),
                    &current_snap,
                ) {
                    break;
                }
            }
        }

        // Получаем snapshot и offset подписчика
        let snapshots = self.snapshots.load();
        let offsets = self.offsets.load();
        let sub_offset_map = offsets.get(&sub_id).ok_or(())?;
        let mut new_offset_map = sub_offset_map.clone();

        let mut result: Vec<Arc<T>> = Vec::new();
        if let Some(events) = snapshots.get(&wanted) {
            let offset = sub_offset_map.get(&wanted).copied().unwrap_or(0);
            if offset < events.len() {
                for ev in &events[offset..] {
                    // Downcast Arc<dyn Event> -> Arc<T>
                    let e = Arc::clone(ev).downcast::<T>().expect("Unexpected type");
                    result.push(e);
                }
                new_offset_map.insert(wanted, events.len());
            }
        }

        // Обновляем offset подписчика
        loop {
            let current = self.offsets.load();
            let mut new_map = (**current).clone();
            new_map.insert(sub_id, new_offset_map.clone());

            if Arc::ptr_eq(
                &self.offsets.compare_and_swap(&current, Arc::new(new_map)),
                &current,
            ) {
                break;
            }
        }
        
        Ok(result)
    }
}
