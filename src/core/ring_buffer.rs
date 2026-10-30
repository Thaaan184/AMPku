//! Lock-free Single-Producer Single-Consumer (SPSC) ring buffer wrapper.

pub use rtrb::{
    Consumer as AudioConsumer, PopError, Producer as AudioProducer, PushError, RingBuffer,
};

/// Helper struct for type-level reference to an audio ring buffer pair.
#[derive(Debug)]
pub struct AudioRingBuffer<T> {
    _marker: std::marker::PhantomData<T>,
}

impl<T> AudioRingBuffer<T> {
    /// Create a lock-free SPSC ring buffer pair with the requested capacity.
    #[allow(clippy::new_ret_no_self)]
    #[inline]
    pub fn new(capacity: usize) -> (AudioProducer<T>, AudioConsumer<T>) {
        create_audio_ring_buffer(capacity)
    }

    /// Create a lock-free SPSC ring buffer pair with the requested capacity.
    #[inline]
    pub fn pair(capacity: usize) -> (AudioProducer<T>, AudioConsumer<T>) {
        create_audio_ring_buffer(capacity)
    }
}

/// Create a lock-free Single-Producer Single-Consumer (SPSC) ring buffer.
///
/// Provides bounded FIFO queue with zero allocations during push/pop.
#[inline]
pub fn create_audio_ring_buffer<T>(capacity: usize) -> (AudioProducer<T>, AudioConsumer<T>) {
    RingBuffer::new(capacity)
}
