use std::future::Future;

pub struct Buffer<T, F, Fut>
where
    F: FnMut(&[T]) -> Fut,
    Fut: Future<Output = ()>,
{
    vec: Vec<T>,
    capacity: usize,
    save_fn: F,
}

impl<T, F, Fut> Buffer<T, F, Fut>
where
    F: FnMut(&[T]) -> Fut,
    Fut: Future<Output = ()>,
{
    pub fn new(capacity: usize, save_fn: F) -> Self {
        Self {
            vec: Vec::with_capacity(capacity),
            capacity,
            save_fn,
        }
    }

    pub async fn push(&mut self, item: T) {
        self.vec.push(item);
        if self.vec.len() >= self.capacity {
            (self.save_fn)(&self.vec).await;
            self.vec.clear();
        }
    }

    pub async fn flush(&mut self) {
        if !self.vec.is_empty() {
            (self.save_fn)(&self.vec).await;
            self.vec.clear();
        }
    }
}
