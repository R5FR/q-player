use crate::models::*;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use std::fs::File;
use std::io::{BufReader, Cursor};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info};

/// Hi-Res audio player with double-buffering and gapless playback
///
/// Safety: AudioPlayer is always accessed behind a RwLock in AppState,
/// so only one thread mutates it at a time. OutputStream/Sink are not truly
/// thread-safe, but with single-threaded access via the lock this is safe.
pub struct AudioPlayer {
    /// Current output stream (kept alive to prevent audio drop)
    _stream: Option<OutputStream>,
    stream_handle: Option<OutputStreamHandle>,
    sink: Option<Sink>,

    /// Track metadata
    current_track: Option<UnifiedTrack>,
    current_sample_rate: Option<f64>,
    current_bit_depth: Option<i32>,

    /// Playback state tracking
    is_playing: Arc<AtomicBool>,
    position_ms: Arc<AtomicU64>,
    duration_ms: Arc<AtomicU64>,

    /// Volume (0.0 to 1.0, applied with cubic curve)
    volume: f32,

    /// Pre-queued next track path for gapless
    next_track_queued: Option<PathBuf>,
}

// Safety: Always accessed behind RwLock<AudioPlayer> in AppState
unsafe impl Send for AudioPlayer {}
unsafe impl Sync for AudioPlayer {}

impl AudioPlayer {
    pub fn new() -> Self {
        Self {
            _stream: None,
            stream_handle: None,
            sink: None,
            current_track: None,
            current_sample_rate: None,
            current_bit_depth: None,
            is_playing: Arc::new(AtomicBool::new(false)),
            position_ms: Arc::new(AtomicU64::new(0)),
            duration_ms: Arc::new(AtomicU64::new(0)),
            volume: 0.7,
            next_track_queued: None,
        }
    }

    /// Cubic volume curve for perceptual linearity
    fn cubic_volume(linear: f32) -> f32 {
        linear * linear * linear
    }

    /// Initialize or reinitialize the audio output stream
    fn ensure_stream(&mut self) -> Result<(), String> {
        if self.stream_handle.is_some() {
            return Ok(());
        }

        let (stream, stream_handle) = OutputStream::try_default()
            .map_err(|e| format!("Failed to open audio output: {}", e))?;

        self._stream = Some(stream);
        self.stream_handle = Some(stream_handle);
        Ok(())
    }

    /// Recreate the sink (needed when sample rate changes)
    fn recreate_sink(&mut self) -> Result<(), String> {
        // Drop existing sink
        if let Some(sink) = self.sink.take() {
            sink.stop();
        }
        self._stream = None;
        self.stream_handle = None;

        // Small delay for DAC switching
        std::thread::sleep(Duration::from_millis(100));

        self.ensure_stream()?;

        let handle = self
            .stream_handle
            .as_ref()
            .ok_or("No stream handle available")?;

        let sink = Sink::try_new(handle)
            .map_err(|e| format!("Failed to create audio sink: {}", e))?;

        sink.set_volume(Self::cubic_volume(self.volume));
        self.sink = Some(sink);
        Ok(())
    }

    /// Play a track from raw in-memory bytes (used for network streaming)
    pub fn play_bytes(
        &mut self,
        bytes: Vec<u8>,
        track: UnifiedTrack,
        sample_rate: Option<f64>,
        bit_depth: Option<i32>,
    ) -> Result<(), String> {
        info!("Playing (in-memory): {} - {}", track.artist, track.title);

        let need_recreate = match (self.current_sample_rate, sample_rate) {
            (Some(current), Some(new)) => (current - new).abs() > 0.1,
            (None, _) => true,
            (_, None) => false,
        };

        if need_recreate || self.sink.is_none() {
            debug!(
                "Recreating sink (sample rate: {:?} -> {:?})",
                self.current_sample_rate, sample_rate
            );
            self.recreate_sink()?;
        }

        let cursor = Cursor::new(bytes);
        let source = Decoder::new(cursor)
            .map_err(|e| format!("Failed to decode audio: {}", e))?;

        let duration_ms = source
            .total_duration()
            .map(|d| d.as_millis() as u64)
            .unwrap_or((track.duration_seconds as u64) * 1000);

        if let Some(sink) = &self.sink {
            sink.clear();
            sink.append(source);
            sink.play();
        }

        self.current_track = Some(track);
        self.current_sample_rate = sample_rate;
        self.current_bit_depth = bit_depth;
        self.is_playing.store(true, Ordering::SeqCst);
        self.position_ms.store(0, Ordering::SeqCst);
        self.duration_ms.store(duration_ms, Ordering::SeqCst);
        self.next_track_queued = None;

        Ok(())
    }

    /// Play a track from a local file path
    pub fn play_file(
        &mut self,
        file_path: &PathBuf,
        track: UnifiedTrack,
        sample_rate: Option<f64>,
        bit_depth: Option<i32>,
    ) -> Result<(), String> {
        info!("Playing: {} - {}", track.artist, track.title);

        // Check if we need to recreate the sink (sample rate change)
        let need_recreate = match (self.current_sample_rate, sample_rate) {
            (Some(current), Some(new)) => (current - new).abs() > 0.1,
            (None, _) => true,
            (_, None) => false,
        };

        if need_recreate || self.sink.is_none() {
            debug!(
                "Recreating sink (sample rate change: {:?} -> {:?})",
                self.current_sample_rate, sample_rate
            );
            self.recreate_sink()?;
        }

        let file = File::open(file_path)
            .map_err(|e| format!("Failed to open audio file: {}", e))?;
        let reader = BufReader::new(file);

        let source = Decoder::new(reader)
            .map_err(|e| format!("Failed to decode audio: {}", e))?;

        let duration_ms = source
            .total_duration()
            .map(|d| d.as_millis() as u64)
            .unwrap_or((track.duration_seconds as u64) * 1000);

        // Clear current playback and append new source
        if let Some(sink) = &self.sink {
            sink.clear();
            sink.append(source);
            sink.play();
        }

        self.current_track = Some(track);
        self.current_sample_rate = sample_rate;
        self.current_bit_depth = bit_depth;
        self.is_playing.store(true, Ordering::SeqCst);
        self.position_ms.store(0, Ordering::SeqCst);
        self.duration_ms.store(duration_ms, Ordering::SeqCst);
        self.next_track_queued = None;

        Ok(())
    }

    /// Pre-queue the next track for gapless playback (double-buffer)
    pub fn queue_next_for_gapless(
        &mut self,
        file_path: &PathBuf,
        next_sample_rate: Option<f64>,
    ) -> Result<(), String> {
        // Can only do gapless if sample rates match
        let can_gapless = match (self.current_sample_rate, next_sample_rate) {
            (Some(current), Some(next)) => (current - next).abs() < 0.1,
            _ => true,
        };

        if !can_gapless {
            debug!("Cannot do gapless: sample rate mismatch");
            return Ok(());
        }

        let file = File::open(file_path)
            .map_err(|e| format!("Failed to open next track: {}", e))?;
        let reader = BufReader::new(file);

        let source = Decoder::new(reader)
            .map_err(|e| format!("Failed to decode next track: {}", e))?;

        if let Some(sink) = &self.sink {
            sink.append(source);
            self.next_track_queued = Some(file_path.clone());
            debug!("Next track queued for gapless playback");
        }

        Ok(())
    }

    pub fn pause(&mut self) {
        if let Some(sink) = &self.sink {
            sink.pause();
            self.is_playing.store(false, Ordering::SeqCst);
        }
    }

    pub fn resume(&mut self) {
        if let Some(sink) = &self.sink {
            sink.play();
            self.is_playing.store(true, Ordering::SeqCst);
        }
    }

    pub fn stop(&mut self) {
        if let Some(sink) = &self.sink {
            sink.stop();
        }
        self.is_playing.store(false, Ordering::SeqCst);
        self.position_ms.store(0, Ordering::SeqCst);
        self.current_track = None;
        self.next_track_queued = None;
    }

    pub fn seek(&mut self, position_ms: u64) {
        if let Some(sink) = &self.sink {
            let _ = sink.try_seek(Duration::from_millis(position_ms));
            self.position_ms.store(position_ms, Ordering::SeqCst);
        }
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        if let Some(sink) = &self.sink {
            sink.set_volume(Self::cubic_volume(self.volume));
        }
    }

    pub fn is_finished(&self) -> bool {
        self.sink.as_ref().map(|s| s.empty()).unwrap_or(true)
    }

    pub fn playback_state(&self) -> PlaybackState {
        // Use rodio's real-time position from the sink (accurate during playback).
        // Fall back to the stored value (e.g. right after seek before first tick).
        let position_ms = self
            .sink
            .as_ref()
            .map(|s| s.get_pos().as_millis() as u64)
            .unwrap_or_else(|| self.position_ms.load(Ordering::SeqCst));

        // Treat track as stopped when the sink buffer is empty (natural end-of-track).
        let is_playing = self.is_playing.load(Ordering::SeqCst)
            && self
                .sink
                .as_ref()
                .map(|s| !s.empty())
                .unwrap_or(false);

        PlaybackState {
            is_playing,
            current_track: self.current_track.clone(),
            position_ms,
            duration_ms: self.duration_ms.load(Ordering::SeqCst),
            volume: self.volume,
            quality: self.current_track.as_ref().and_then(|t| t.quality_label.clone()),
            sample_rate: self.current_sample_rate,
            bit_depth: self.current_bit_depth,
        }
    }
}
