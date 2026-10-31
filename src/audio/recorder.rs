//! Background WAV audio recorder with lock-free SPSC queue and Hound file writer.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

use crate::core::frame::AudioFrame;
use crate::core::ring_buffer::{AudioConsumer, AudioProducer, AudioRingBuffer};

/// Default capacity for the lock-free recording ring buffer:
/// 65,536 frames = ~1.365 seconds of 48 kHz stereo audio.
pub const DEFAULT_RECORDER_CAPACITY: usize = 65536;

/// Convert floating point audio sample `[-1.0, 1.0]` to signed 24-bit integer PCM `[-8388608, 8388607]`.
#[inline]
pub fn sample_to_i24(sample: f32) -> i32 {
    if !sample.is_finite() {
        return 0;
    }
    let clamped = sample.clamp(-1.0, 1.0);
    if clamped >= 0.0 {
        (clamped * 8_388_607.0).round() as i32
    } else {
        (clamped * 8_388_608.0).round() as i32
    }
}

/// Convert signed 24-bit integer PCM `[-8388608, 8388607]` to floating point audio sample `[-1.0, 1.0]`.
#[inline]
pub fn i24_to_sample(sample: i32) -> f32 {
    let clamped = sample.clamp(-8_388_608, 8_388_607);
    if clamped >= 0 {
        clamped as f32 / 8_388_607.0
    } else {
        clamped as f32 / 8_388_608.0
    }
}

/// Errors that can occur during audio recording operations.
#[derive(Debug)]
pub enum RecorderError {
    /// Attempted to start recording while already actively recording.
    AlreadyRecording,
    /// Attempted to stop recording when not active.
    NotRecording,
    /// Standard I/O filesystem error.
    Io(std::io::Error),
    /// Hound WAV encoding / writing error.
    Hound(hound::Error),
    /// Background file I/O thread communication or panic error.
    Thread(String),
}

impl std::fmt::Display for RecorderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyRecording => write!(f, "Recording is already active"),
            Self::NotRecording => write!(f, "Recording is not currently active"),
            Self::Io(err) => write!(f, "Filesystem I/O error: {}", err),
            Self::Hound(err) => write!(f, "WAV encoding error: {}", err),
            Self::Thread(msg) => write!(f, "Recorder worker thread error: {}", msg),
        }
    }
}

impl std::error::Error for RecorderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Hound(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for RecorderError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<hound::Error> for RecorderError {
    fn from(err: hound::Error) -> Self {
        Self::Hound(err)
    }
}

/// Real-time background WAV recorder writing 24-bit 48kHz audio.
///
/// Communicates with a dedicated background thread via a lock-free single-producer
/// single-consumer (SPSC) ring buffer (`rtrb`). Disk writes and file I/O never block
/// the real-time audio thread.
pub struct WavRecorder {
    producer: Option<AudioProducer<AudioFrame>>,
    is_recording: Arc<AtomicBool>,
    stop_flag: Option<Arc<AtomicBool>>,
    worker_thread: Option<JoinHandle<Result<(), String>>>,
    frames_recorded: Arc<AtomicU64>,
    dropped_frames: Arc<AtomicU64>,
    sample_rate: u32,
    current_path: Option<PathBuf>,
    capacity: usize,
}

impl WavRecorder {
    /// Create a new WavRecorder with default ring buffer capacity (65,536 frames).
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_RECORDER_CAPACITY)
    }

    /// Create a new WavRecorder with custom ring buffer capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            producer: None,
            is_recording: Arc::new(AtomicBool::new(false)),
            stop_flag: None,
            worker_thread: None,
            frames_recorded: Arc::new(AtomicU64::new(0)),
            dropped_frames: Arc::new(AtomicU64::new(0)),
            sample_rate: 48000,
            current_path: None,
            capacity: capacity.max(256),
        }
    }

    /// Start recording audio frames to the specified WAV file path.
    ///
    /// Configures 24-bit PCM stereo at the given sample rate. Disk writes run on
    /// a dedicated background worker thread.
    pub fn start_recording(
        &mut self,
        path: impl Into<PathBuf>,
        sample_rate: u32,
    ) -> Result<(), RecorderError> {
        if self.is_recording.load(Ordering::SeqCst) {
            return Err(RecorderError::AlreadyRecording);
        }

        let path = path.into();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }

        let spec = hound::WavSpec {
            channels: 2,
            sample_rate,
            bits_per_sample: 24,
            sample_format: hound::SampleFormat::Int,
        };

        // Create the hound WAV file writer before spawning thread to catch early filesystem errors
        let writer = hound::WavWriter::create(&path, spec)?;

        let (producer, consumer) = AudioRingBuffer::<AudioFrame>::new(self.capacity);
        self.producer = Some(producer);

        let stop_flag = Arc::new(AtomicBool::new(false));
        self.stop_flag = Some(stop_flag.clone());

        self.frames_recorded.store(0, Ordering::Relaxed);
        self.dropped_frames.store(0, Ordering::Relaxed);
        self.current_path = Some(path);
        self.sample_rate = sample_rate;
        self.is_recording.store(true, Ordering::SeqCst);

        let thread_handle = std::thread::Builder::new()
            .name("ampku-wav-writer".to_string())
            .spawn(move || Self::worker_loop(writer, consumer, stop_flag))
            .map_err(RecorderError::Io)?;

        self.worker_thread = Some(thread_handle);
        Ok(())
    }

    /// Stop recording, drain remaining frames from the ring buffer, finalize the WAV header,
    /// and close the file.
    pub fn stop_recording(&mut self) -> Result<(), RecorderError> {
        if !self.is_recording.load(Ordering::SeqCst) {
            return Err(RecorderError::NotRecording);
        }

        self.is_recording.store(false, Ordering::SeqCst);

        // Drop the producer so the audio thread stops enqueuing
        drop(self.producer.take());

        // Signal worker thread to finish draining and finalize
        if let Some(stop_flag) = self.stop_flag.take() {
            stop_flag.store(true, Ordering::Release);
        }

        if let Some(handle) = self.worker_thread.take() {
            match handle.join() {
                Ok(result) => {
                    result.map_err(RecorderError::Thread)?;
                }
                Err(_) => {
                    return Err(RecorderError::Thread(
                        "Recorder background thread panicked".to_string(),
                    ));
                }
            }
        }

        Ok(())
    }

    /// Push a single stereo audio frame into the recording queue (real-time safe, non-blocking).
    ///
    /// If the queue is full (due to sustained disk latency), the frame is dropped
    /// and the dropped frames counter is incremented without blocking the audio thread.
    ///
    /// Returns `true` if the frame was successfully queued, `false` otherwise.
    #[inline]
    pub fn push_frame(&mut self, frame: AudioFrame) -> bool {
        if !self.is_recording.load(Ordering::Relaxed) {
            return false;
        }

        if let Some(ref mut prod) = self.producer {
            match prod.push(frame) {
                Ok(()) => {
                    self.frames_recorded.fetch_add(1, Ordering::Relaxed);
                    true
                }
                Err(_) => {
                    self.dropped_frames.fetch_add(1, Ordering::Relaxed);
                    false
                }
            }
        } else {
            false
        }
    }

    /// Push a slice of audio frames into the recording queue (real-time safe, non-blocking).
    ///
    /// Returns the number of frames successfully queued.
    pub fn push_block(&mut self, frames: &[AudioFrame]) -> usize {
        if !self.is_recording.load(Ordering::Relaxed) {
            return 0;
        }

        let mut count = 0;
        for &frame in frames {
            if self.push_frame(frame) {
                count += 1;
            }
        }
        count
    }

    /// Return true if recording is currently active.
    #[inline]
    pub fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::Relaxed)
    }

    /// Total number of stereo frames successfully recorded in current session.
    #[inline]
    pub fn frames_recorded(&self) -> u64 {
        self.frames_recorded.load(Ordering::Relaxed)
    }

    /// Total number of frames dropped due to buffer overflow.
    #[inline]
    pub fn dropped_frames(&self) -> u64 {
        self.dropped_frames.load(Ordering::Relaxed)
    }

    /// Duration of current recording in seconds.
    #[inline]
    pub fn duration_seconds(&self) -> f64 {
        let frames = self.frames_recorded();
        if self.sample_rate > 0 {
            frames as f64 / self.sample_rate as f64
        } else {
            0.0
        }
    }

    /// Configured recording sample rate in Hz.
    #[inline]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Path to current or most recent recording file.
    #[inline]
    pub fn current_path(&self) -> Option<&Path> {
        self.current_path.as_deref()
    }

    /// Capacity of the internal lock-free ring buffer in frames.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Dedicated background thread loop that consumes audio frames and writes them to hound.
    fn worker_loop<W: std::io::Write + std::io::Seek>(
        mut writer: hound::WavWriter<W>,
        mut consumer: AudioConsumer<AudioFrame>,
        stop_flag: Arc<AtomicBool>,
    ) -> Result<(), String> {
        while !stop_flag.load(Ordering::Acquire) || !consumer.is_empty() {
            let mut popped = false;
            while let Ok(frame) = consumer.pop() {
                popped = true;
                let left_i24 = sample_to_i24(frame.left);
                let right_i24 = sample_to_i24(frame.right);

                if let Err(e) = writer.write_sample(left_i24) {
                    return Err(format!("Hound write left sample failed: {}", e));
                }
                if let Err(e) = writer.write_sample(right_i24) {
                    return Err(format!("Hound write right sample failed: {}", e));
                }
            }

            if !popped {
                if stop_flag.load(Ordering::Acquire) {
                    break;
                }
                // Sleep briefly while waiting for audio frames from audio thread
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        }

        // Final drain of any leftover frames
        while let Ok(frame) = consumer.pop() {
            let left_i24 = sample_to_i24(frame.left);
            let right_i24 = sample_to_i24(frame.right);
            if let Err(e) = writer.write_sample(left_i24) {
                return Err(format!("Hound write left sample failed: {}", e));
            }
            if let Err(e) = writer.write_sample(right_i24) {
                return Err(format!("Hound write right sample failed: {}", e));
            }
        }

        writer
            .finalize()
            .map_err(|e| format!("Hound finalize WAV file failed: {}", e))
    }
}

impl Default for WavRecorder {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for WavRecorder {
    fn drop(&mut self) {
        if self.is_recording() {
            let _ = self.stop_recording();
        }
    }
}
