//! Modular dynamic effect chain and channel strip audio engine.

pub mod chain;
pub mod channel;
pub mod master_bus;

pub use chain::{ChainCommand, CommandConsumer, CommandProducer, EffectChain};
pub use channel::{
    calculate_pan_gains, db_to_linear, linear_to_db, ChannelMeter, ChannelStrip, PanLaw,
};
pub use master_bus::MasterBus;
