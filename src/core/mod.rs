//! Core audio primitives and data structures.

pub mod frame;
pub mod ring_buffer;

pub use frame::AudioFrame;
pub use ring_buffer::{create_audio_ring_buffer, AudioConsumer, AudioProducer, AudioRingBuffer};
