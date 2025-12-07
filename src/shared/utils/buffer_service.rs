use crossbeam::queue::SegQueue;
use std::error::Error;
use std::marker::PhantomData;

pub trait Callback<T, E>: Send + Sync
where
    E: Error + Send,
{
    async fn on_buffer_flush(&self, data: &[T]) -> Result<(), E>;
}

pub struct BufferService<T, C, E>
where
    T: Send + Sync,
    E: Error + Send,
    C: Callback<T, E>,
{
    data: SegQueue<T>,
    buffer_size: usize,
    callback: C,
    _marker: PhantomData<E>,
}

impl<T, C, E> BufferService<T, C, E>
where
    T: Send + Sync,
    C: Callback<T, E> + Send + Sync,
    E: Error + Send,
{
    pub fn new(callback: C, buffer_size: usize) -> Self {
        Self {
            data: SegQueue::new(),
            buffer_size,
            callback,
            _marker: PhantomData,
        }
    }

    pub async fn push(&self, item: T) -> Result<(), E> {
        self.data.push(item);

        if self.data.len() >= self.buffer_size {
            self.flush().await?;
        }

        Ok(())
    }

    pub async fn flush(&self) -> Result<(), E> {
        if !self.data.is_empty() {
            let mut data = Vec::with_capacity(self.buffer_size);
            while let Some(item) = self.data.pop() {
                data.push(item);
            }
            self.callback.on_buffer_flush(&data).await?;
        }
        Ok(())
    }
}
