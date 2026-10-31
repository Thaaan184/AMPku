//! Lock-free atomic 32-bit floating-point primitive using AtomicU32.

use std::sync::atomic::{AtomicU32, Ordering};

/// Thread-safe lock-free atomic `f32` value backed by `AtomicU32`.
///
/// Converts between `f32` and `u32` raw IEEE 754 bit representations.
#[derive(Debug, Default)]
pub struct AtomicF32 {
    storage: AtomicU32,
}

impl AtomicF32 {
    /// Create a new atomic float initialized to the given value.
    #[inline]
    pub const fn new(val: f32) -> Self {
        Self {
            storage: AtomicU32::new(val.to_bits()),
        }
    }

    /// Load the current float value with relaxed memory ordering.
    #[inline]
    pub fn get(&self) -> f32 {
        f32::from_bits(self.storage.load(Ordering::Relaxed))
    }

    /// Store a new float value with relaxed memory ordering.
    #[inline]
    pub fn set(&self, val: f32) {
        self.storage.store(val.to_bits(), Ordering::Relaxed);
    }

    /// Load the current float value with the specified memory ordering.
    #[inline]
    pub fn load(&self, order: Ordering) -> f32 {
        f32::from_bits(self.storage.load(order))
    }

    /// Store a new float value with the specified memory ordering.
    #[inline]
    pub fn store(&self, val: f32, order: Ordering) {
        self.storage.store(val.to_bits(), order);
    }

    /// Atomically swap the float value and return the previous value.
    #[inline]
    pub fn swap(&self, val: f32, order: Ordering) -> f32 {
        f32::from_bits(self.storage.swap(val.to_bits(), order))
    }

    /// Atomically update value to `max(current, val)` via compare-and-swap loop.
    #[inline]
    pub fn update_max(&self, val: f32) {
        let mut current = self.get();
        while val > current {
            match self.storage.compare_exchange_weak(
                current.to_bits(),
                val.to_bits(),
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => break,
                Err(actual) => current = f32::from_bits(actual),
            }
        }
    }
}

impl From<f32> for AtomicF32 {
    #[inline]
    fn from(val: f32) -> Self {
        Self::new(val)
    }
}

impl std::fmt::Display for AtomicF32 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.get())
    }
}
