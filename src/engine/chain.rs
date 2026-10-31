//! Modular dynamic audio effect chain with lock-free real-time command processing.

use crate::core::frame::AudioFrame;
use crate::dsp::traits::AudioEffect;

/// Lock-free real-time commands for dynamically reconfiguring the effect chain from the UI thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainCommand {
    /// Enable or bypass effect at index.
    SetEnabled { index: usize, enabled: bool },
    /// Swap positions of two effects.
    Swap { index_a: usize, index_b: usize },
    /// Move effect from index to another index.
    Move { from: usize, to: usize },
    /// Reset all effect buffers in the chain.
    Reset,
    /// Disable all effects in chain without deallocating on the real-time audio thread.
    Clear,
}

/// Producer handle for sending real-time chain commands from the GUI thread.
pub type CommandProducer = rtrb::Producer<ChainCommand>;
/// Consumer handle for receiving real-time chain commands on the audio thread.
pub type CommandConsumer = rtrb::Consumer<ChainCommand>;

/// Serial audio effect chain supporting dynamic addition, removal, swapping, and bypassing.
pub struct EffectChain {
    effects: Vec<Box<dyn AudioEffect>>,
    enabled: bool,
    command_consumer: Option<CommandConsumer>,
}

impl EffectChain {
    /// Create an empty effect chain.
    pub fn new() -> Self {
        Self {
            effects: Vec::new(),
            enabled: true,
            command_consumer: None,
        }
    }

    /// Create an empty effect chain with pre-allocated node capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            effects: Vec::with_capacity(capacity),
            enabled: true,
            command_consumer: None,
        }
    }

    /// Create an effect chain paired with an lock-free command producer.
    pub fn with_command_queue(capacity: usize) -> (Self, CommandProducer) {
        let (producer, consumer) = rtrb::RingBuffer::new(capacity);
        let chain = Self {
            effects: Vec::new(),
            enabled: true,
            command_consumer: Some(consumer),
        };
        (chain, producer)
    }

    /// Create a lock-free SPSC command channel for external connection.
    pub fn create_command_channel(capacity: usize) -> (CommandProducer, CommandConsumer) {
        rtrb::RingBuffer::new(capacity)
    }

    /// Attach an existing command consumer to this chain.
    pub fn attach_command_consumer(&mut self, consumer: CommandConsumer) {
        self.command_consumer = Some(consumer);
    }

    /// Process all pending commands from the attached command queue without blocking or allocating.
    pub fn process_commands(&mut self) {
        if let Some(mut consumer) = self.command_consumer.take() {
            while let Ok(cmd) = consumer.pop() {
                self.execute_command(cmd);
            }
            self.command_consumer = Some(consumer);
        }
    }

    /// Execute a single chain command in-place.
    pub fn execute_command(&mut self, cmd: ChainCommand) {
        match cmd {
            ChainCommand::SetEnabled { index, enabled } => {
                self.set_effect_enabled(index, enabled);
            }
            ChainCommand::Swap { index_a, index_b } => {
                self.swap_effects(index_a, index_b);
            }
            ChainCommand::Move { from, to } => {
                self.move_effect(from, to);
            }
            ChainCommand::Reset => {
                self.reset();
            }
            ChainCommand::Clear => {
                for effect in self.effects.iter_mut() {
                    effect.set_enabled(false);
                }
            }
        }
    }

    /// Append a new boxed audio effect to the end of the chain.
    ///
    /// Returns the index of the newly added effect.
    pub fn add_effect(&mut self, effect: Box<dyn AudioEffect>) -> usize {
        let index = self.effects.len();
        self.effects.push(effect);
        index
    }

    /// Insert an effect at a specific index in the chain.
    pub fn insert_effect(&mut self, index: usize, effect: Box<dyn AudioEffect>) {
        if index <= self.effects.len() {
            self.effects.insert(index, effect);
        } else {
            self.effects.push(effect);
        }
    }

    /// Remove and return the effect at the given index.
    pub fn remove_effect(&mut self, index: usize) -> Option<Box<dyn AudioEffect>> {
        if index < self.effects.len() {
            Some(self.effects.remove(index))
        } else {
            None
        }
    }

    /// Swap positions of two effects in the chain.
    ///
    /// Returns true if swap succeeded, false if indices are out of bounds.
    pub fn swap_effects(&mut self, index_a: usize, index_b: usize) -> bool {
        if index_a < self.effects.len() && index_b < self.effects.len() {
            self.effects.swap(index_a, index_b);
            true
        } else {
            false
        }
    }

    /// Move an effect from `from` index to `to` index.
    ///
    /// Returns true if move succeeded, false if indices are out of bounds.
    pub fn move_effect(&mut self, from: usize, to: usize) -> bool {
        let len = self.effects.len();
        if from >= len || to >= len {
            return false;
        }
        if from == to {
            return true;
        }
        let effect = self.effects.remove(from);
        self.effects.insert(to, effect);
        true
    }

    /// Get a shared reference to the effect at index.
    pub fn get_effect(&self, index: usize) -> Option<&dyn AudioEffect> {
        self.effects.get(index).map(|b| &**b as &dyn AudioEffect)
    }

    /// Get a mutable reference to the effect at index.
    pub fn get_effect_mut(&mut self, index: usize) -> Option<&mut (dyn AudioEffect + '_)> {
        self.effects
            .get_mut(index)
            .map(|b| &mut **b as &mut (dyn AudioEffect + '_))
    }

    /// Get a shared reference to the boxed effect at index.
    #[allow(clippy::borrowed_box)]
    pub fn get_boxed(&self, index: usize) -> Option<&Box<dyn AudioEffect>> {
        self.effects.get(index)
    }

    /// Get a mutable reference to the boxed effect at index.
    pub fn get_boxed_mut(&mut self, index: usize) -> Option<&mut Box<dyn AudioEffect>> {
        self.effects.get_mut(index)
    }

    /// Return the number of effects in the chain.
    #[inline]
    pub fn len(&self) -> usize {
        self.effects.len()
    }

    /// Return true if the chain contains no effects.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.effects.is_empty()
    }

    /// Remove all effects from the chain.
    pub fn clear(&mut self) {
        self.effects.clear();
    }

    /// Enable or bypass the entire chain.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Return true if the chain is currently enabled.
    #[inline]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Enable or bypass an individual effect at index.
    pub fn set_effect_enabled(&mut self, index: usize, enabled: bool) -> bool {
        if let Some(effect) = self.effects.get_mut(index) {
            effect.set_enabled(enabled);
            true
        } else {
            false
        }
    }

    /// Bypass an individual effect (`bypassed == true` disables the effect).
    pub fn bypass_effect(&mut self, index: usize, bypassed: bool) -> bool {
        self.set_effect_enabled(index, !bypassed)
    }

    /// Check if an individual effect is enabled.
    pub fn is_effect_enabled(&self, index: usize) -> Option<bool> {
        self.effects.get(index).map(|e| e.is_enabled())
    }

    /// Shared slice of all boxed effects in order.
    pub fn effects(&self) -> &[Box<dyn AudioEffect>] {
        &self.effects
    }

    /// Mutable slice of all boxed effects.
    pub fn effects_mut(&mut self) -> &mut [Box<dyn AudioEffect>] {
        &mut self.effects
    }

    /// Iterator over boxed effects.
    pub fn iter(&self) -> std::slice::Iter<'_, Box<dyn AudioEffect>> {
        self.effects.iter()
    }

    /// Mutable iterator over boxed effects.
    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, Box<dyn AudioEffect>> {
        self.effects.iter_mut()
    }

    /// Process a single stereo frame through all active effects sequentially.
    pub fn process_frame(&mut self, mut frame: AudioFrame) -> AudioFrame {
        self.process_commands();
        if !self.enabled {
            return frame;
        }

        for effect in self.effects.iter_mut() {
            if effect.is_enabled() {
                frame = effect.process_frame(frame).sanitize();
            }
        }
        frame
    }

    /// Process a block of stereo frames in-place through all active effects.
    pub fn process_block(&mut self, frames: &mut [AudioFrame]) {
        self.process_commands();
        if !self.enabled {
            return;
        }

        for effect in self.effects.iter_mut() {
            if effect.is_enabled() {
                effect.process_block(frames);
                for f in frames.iter_mut() {
                    *f = f.sanitize();
                }
            }
        }
    }

    /// Reset internal state buffers for all effects in the chain.
    pub fn reset(&mut self) {
        for effect in self.effects.iter_mut() {
            effect.reset();
        }
    }
}

impl Default for EffectChain {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioEffect for EffectChain {
    #[inline]
    fn process_frame(&mut self, frame: AudioFrame) -> AudioFrame {
        self.process_frame(frame)
    }

    #[inline]
    fn process_block(&mut self, frames: &mut [AudioFrame]) {
        self.process_block(frames);
    }

    #[inline]
    fn reset(&mut self) {
        self.reset();
    }

    #[inline]
    fn name(&self) -> &'static str {
        "Effect Chain"
    }

    #[inline]
    fn is_enabled(&self) -> bool {
        self.is_enabled()
    }

    #[inline]
    fn set_enabled(&mut self, enabled: bool) {
        self.set_enabled(enabled);
    }
}
