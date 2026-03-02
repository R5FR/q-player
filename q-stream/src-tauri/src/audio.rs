use crate::models::*;
use rodio::{buffer::SamplesBuffer, OutputStream, Sink};
use std::fs::File;
use std::io::Cursor;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::{Duration, Instant};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::units::Time;
use tracing::{debug, error, info, warn};

// ── Commands sent to the decoder thread ─────────────────────────────

enum AudioCommand {
    /// Load audio from in-memory bytes. If `seek_to_ms` is Some, seek immediately after loading.
    LoadBytes {
        bytes: Arc<Vec<u8>>,
        seek_to_ms: Option<u64>,
    },
    /// Load audio from a local file. If `seek_to_ms` is Some, seek immediately after loading.
    LoadFile {
        path: PathBuf,
        seek_to_ms: Option<u64>,
    },
    Seek(u64),
    Pause,
    Resume,
    Stop,
    SetVolume(f32),
}

/// Events emitted by the audio engine (decoder thread → listener).
#[derive(Debug, Clone)]
pub enum PlayerEvent {
    /// Current track finished playing completely (sink drained).
    TrackEnded,
}

// ── Public API ──────────────────────────────────────────────────────

/// Hi-Res audio player using symphonia for decoding + rodio for output.
///
/// Architecture inspired by music-player-master:
///   • A dedicated background thread decodes audio via symphonia
///   • Decoded samples are fed to rodio's Sink via SamplesBuffer
///   • Position tracked via Sink::get_pos() + seek offset (actual playback, not decode-ahead)
///   • Seeking uses symphonia's FormatReader::seek (no rodio try_seek hack)
///   • Track-ended events emitted via mpsc channel for instant transitions
pub struct AudioPlayer {
    command_tx: mpsc::Sender<AudioCommand>,

    // Metadata (protected by outer RwLock in AppState)
    current_track: Option<UnifiedTrack>,
    current_sample_rate: Option<f64>,
    current_bit_depth: Option<i32>,
    volume: f32,

    // Cached source so the track can be replayed from any position after it ends.
    // Arc avoids cloning the bytes on every play; the decoder clones once when loading.
    cached_bytes: Option<Arc<Vec<u8>>>,
    cached_file_path: Option<PathBuf>,

    // Shared with decoder thread (atomic)
    is_playing: Arc<AtomicBool>,
    position_ms: Arc<AtomicU64>,
    duration_ms: Arc<AtomicU64>,
    is_finished: Arc<AtomicBool>,
}

// Safety: Always accessed behind RwLock<AudioPlayer> in AppState.
// mpsc::Sender is Send. Atomics are Send+Sync.
unsafe impl Send for AudioPlayer {}
unsafe impl Sync for AudioPlayer {}

impl AudioPlayer {
    pub fn new() -> (Self, mpsc::Receiver<PlayerEvent>) {
        let (tx, rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();

        let is_playing = Arc::new(AtomicBool::new(false));
        let position_ms = Arc::new(AtomicU64::new(0));
        let duration_ms = Arc::new(AtomicU64::new(0));
        let is_finished = Arc::new(AtomicBool::new(true));

        let shared = SharedState {
            is_playing: is_playing.clone(),
            position_ms: position_ms.clone(),
            duration_ms: duration_ms.clone(),
            is_finished: is_finished.clone(),
        };

        let event_tx_clone = event_tx.clone();
        thread::Builder::new()
            .name("audio-decoder".into())
            .spawn(move || decoder_thread(rx, shared, event_tx_clone))
            .expect("Failed to spawn audio decoder thread");

        (
            Self {
                command_tx: tx,
                current_track: None,
                current_sample_rate: None,
                current_bit_depth: None,
                volume: 0.7,
                cached_bytes: None,
                cached_file_path: None,
                is_playing,
                position_ms,
                duration_ms,
                is_finished,
            },
            event_rx,
        )
    }

    /// Cubic volume curve for perceptual linearity
    fn cubic_volume(linear: f32) -> f32 {
        linear * linear * linear
    }

    /// Play a track from raw in-memory bytes (Qobuz streaming)
    pub fn play_bytes(
        &mut self,
        bytes: Vec<u8>,
        track: UnifiedTrack,
        sample_rate: Option<f64>,
        bit_depth: Option<i32>,
    ) -> Result<(), String> {
        info!("Playing (in-memory): {} - {}", track.artist, track.title);

        // Set initial state immediately for UI responsiveness
        self.duration_ms
            .store((track.duration_seconds as u64) * 1000, Ordering::SeqCst);
        self.position_ms.store(0, Ordering::SeqCst);
        self.is_playing.store(true, Ordering::SeqCst);
        self.is_finished.store(false, Ordering::SeqCst);

        self.current_track = Some(track.clone());
        self.current_sample_rate = sample_rate;
        self.current_bit_depth = bit_depth;

        // Cache under Arc so the decoder can reload later (post-end seek)
        let arc = Arc::new(bytes);
        self.cached_bytes = Some(arc.clone());
        self.cached_file_path = None;

        self.command_tx
            .send(AudioCommand::LoadBytes { bytes: arc, seek_to_ms: None })
            .map_err(|e| format!("Decoder thread not responding: {}", e))
    }

    /// Play a track from a local file path
    pub fn play_file(
        &mut self,
        file_path: &PathBuf,
        track: UnifiedTrack,
        sample_rate: Option<f64>,
        bit_depth: Option<i32>,
    ) -> Result<(), String> {
        info!("Playing (file): {} - {}", track.artist, track.title);

        self.duration_ms
            .store((track.duration_seconds as u64) * 1000, Ordering::SeqCst);
        self.position_ms.store(0, Ordering::SeqCst);
        self.is_playing.store(true, Ordering::SeqCst);
        self.is_finished.store(false, Ordering::SeqCst);

        self.current_track = Some(track.clone());
        self.current_sample_rate = sample_rate;
        self.current_bit_depth = bit_depth;

        // Cache file path so the decoder can reload later (post-end seek)
        self.cached_bytes = None;
        self.cached_file_path = Some(file_path.clone());

        self.command_tx
            .send(AudioCommand::LoadFile {
                path: file_path.clone(),
                seek_to_ms: None,
            })
            .map_err(|e| format!("Decoder thread not responding: {}", e))
    }

    pub fn pause(&mut self) {
        self.is_playing.store(false, Ordering::SeqCst);
        let _ = self.command_tx.send(AudioCommand::Pause);
    }

    pub fn resume(&mut self) {
        self.is_playing.store(true, Ordering::SeqCst);
        let _ = self.command_tx.send(AudioCommand::Resume);
    }

    pub fn stop(&mut self) {
        self.is_playing.store(false, Ordering::SeqCst);
        self.position_ms.store(0, Ordering::SeqCst);
        self.is_finished.store(true, Ordering::SeqCst);
        self.current_track = None;
        let _ = self.command_tx.send(AudioCommand::Stop);
    }

    pub fn seek(&mut self, position_ms: u64) -> Result<(), String> {
        // Immediately update for UI responsiveness
        self.position_ms.store(position_ms, Ordering::SeqCst);

        // When the track has finished, the decoder is gone.  Reload from cache
        // and start playing from the requested position (post-end scrubbing).
        if self.is_finished.load(Ordering::SeqCst) {
            self.is_finished.store(false, Ordering::SeqCst);
            self.is_playing.store(true, Ordering::SeqCst);

            if let Some(ref arc) = self.cached_bytes {
                return self
                    .command_tx
                    .send(AudioCommand::LoadBytes {
                        bytes: arc.clone(),
                        seek_to_ms: Some(position_ms),
                    })
                    .map_err(|e| format!("Decoder thread not responding: {}", e));
            } else if let Some(ref path) = self.cached_file_path {
                return self
                    .command_tx
                    .send(AudioCommand::LoadFile {
                        path: path.clone(),
                        seek_to_ms: Some(position_ms),
                    })
                    .map_err(|e| format!("Decoder thread not responding: {}", e));
            }
            // No cache — nothing to reload (shouldn't normally happen)
            return Ok(());
        }

        self.command_tx
            .send(AudioCommand::Seek(position_ms))
            .map_err(|e| format!("Decoder thread not responding: {}", e))
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        let _ = self.command_tx.send(AudioCommand::SetVolume(self.volume));
    }

    #[allow(dead_code)]
    pub fn is_finished(&self) -> bool {
        self.is_finished.load(Ordering::SeqCst)
    }

    pub fn playback_state(&self) -> PlaybackState {
        let finished = self.is_finished.load(Ordering::SeqCst);
        let is_playing = self.is_playing.load(Ordering::SeqCst) && !finished;

        PlaybackState {
            is_playing,
            current_track: self.current_track.clone(),
            position_ms: self.position_ms.load(Ordering::SeqCst),
            duration_ms: self.duration_ms.load(Ordering::SeqCst),
            volume: self.volume,
            quality: self
                .current_track
                .as_ref()
                .and_then(|t| t.quality_label.clone()),
            sample_rate: self.current_sample_rate,
            bit_depth: self.current_bit_depth,
        }
    }
}

// ── Decoder Thread ──────────────────────────────────────────────────

/// Shared atomic state between AudioPlayer and the decoder thread.
struct SharedState {
    is_playing: Arc<AtomicBool>,
    position_ms: Arc<AtomicU64>,
    duration_ms: Arc<AtomicU64>,
    is_finished: Arc<AtomicBool>,
}

/// Active symphonia format reader + decoder for the current track.
struct DecoderState {
    format: Box<dyn FormatReader>,
    decoder: Box<dyn symphonia::core::codecs::Decoder>,
    track_id: u32,
    sample_buf: Option<SampleBuffer<f32>>,
}

/// Background thread: decodes audio packets via symphonia and feeds
/// interleaved f32 samples to the rodio Sink for output.
fn decoder_thread(
    command_rx: mpsc::Receiver<AudioCommand>,
    shared: SharedState,
    event_tx: mpsc::Sender<PlayerEvent>,
) {
    // Audio output — created lazily, persists across tracks
    let mut _stream: Option<OutputStream> = None;
    let mut sink: Option<Sink> = None;
    let mut volume: f32 = 0.7;

    // Current decoder (Some while a track is loaded)
    let mut dec_state: Option<DecoderState> = None;
    let mut paused = false;

    // Position tracking via Instant (time-based, independent of rodio internals).
    // Rationale: rodio 0.19 Sink::get_pos() resets for each appended SamplesBuffer
    // source, so it cannot be used to track cumulative playback position.
    let mut playback_start: Option<Instant> = None;
    let mut position_at_start_ms: u64 = 0;

    loop {
        // ── 1. Receive commands ──────────────────────────────────
        // Block when idle / paused / Sink buffer full → save CPU.
        // Non-blocking when actively decoding.
        let buffer_full = sink
            .as_ref()
            .map(|s| s.len() > 20)
            .unwrap_or(false);
        let should_wait = dec_state.is_none() || paused || buffer_full;

        let cmd = if should_wait {
            match command_rx.recv_timeout(Duration::from_millis(25)) {
                Ok(cmd) => Some(cmd),
                Err(mpsc::RecvTimeoutError::Timeout) => None,
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    info!("Decoder thread shutting down");
                    break;
                }
            }
        } else {
            match command_rx.try_recv() {
                Ok(cmd) => Some(cmd),
                Err(mpsc::TryRecvError::Empty) => None,
                Err(mpsc::TryRecvError::Disconnected) => {
                    info!("Decoder thread shutting down");
                    break;
                }
            }
        };

        if let Some(cmd) = cmd {
            match cmd {
                // ── Load from in-memory bytes ────────────────────
                AudioCommand::LoadBytes { bytes, seek_to_ms } => {
                    if let Some(sk) = &sink {
                        sk.clear();
                    }
                    ensure_sink(&mut _stream, &mut sink, volume);
                    position_at_start_ms = 0;
                    playback_start = Some(Instant::now());

                    // Arc<Vec<u8>> → clone once into Cursor for symphonia
                    let cursor = Cursor::new((*bytes).clone());
                    dec_state = create_decoder(
                        Box::new(cursor),
                        Some("audio/flac"),
                        &shared,
                    );
                    paused = false;
                    if let Some(sk) = &sink {
                        sk.play();
                    }

                    // Immediate seek (used when restarting a finished track)
                    if let Some(ms) = seek_to_ms {
                        if let Some(ds) = &mut dec_state {
                            position_at_start_ms = ms;
                            playback_start = Some(Instant::now());
                            let secs = ms / 1000;
                            let frac = (ms % 1000) as f64 / 1000.0;
                            match ds.format.seek(
                                SeekMode::Accurate,
                                SeekTo::Time {
                                    time: Time::new(secs, frac),
                                    track_id: None,
                                },
                            ) {
                                Ok(_) => {
                                    ds.decoder.reset();
                                    ds.sample_buf = None;
                                    debug!("Post-load seek to {}ms", ms);
                                }
                                Err(e) => error!("Post-load seek failed: {}", e),
                            }
                        }
                    }
                }

                // ── Load from local file ─────────────────────────
                AudioCommand::LoadFile { path, seek_to_ms } => {
                    if let Some(sk) = &sink {
                        sk.clear();
                    }
                    ensure_sink(&mut _stream, &mut sink, volume);
                    position_at_start_ms = 0;
                    playback_start = Some(Instant::now());

                    match File::open(&path) {
                        Ok(file) => {
                            dec_state =
                                create_decoder(Box::new(file), None, &shared);
                            paused = false;
                            if let Some(sk) = &sink {
                                sk.play();
                            }

                            // Immediate seek (used when restarting a finished track)
                            if let Some(ms) = seek_to_ms {
                                if let Some(ds) = &mut dec_state {
                                    position_at_start_ms = ms;
                                    playback_start = Some(Instant::now());
                                    let secs = ms / 1000;
                                    let frac = (ms % 1000) as f64 / 1000.0;
                                    match ds.format.seek(
                                        SeekMode::Accurate,
                                        SeekTo::Time {
                                            time: Time::new(secs, frac),
                                            track_id: None,
                                        },
                                    ) {
                                        Ok(_) => {
                                            ds.decoder.reset();
                                            ds.sample_buf = None;
                                            debug!("Post-load seek to {}ms", ms);
                                        }
                                        Err(e) => error!("Post-load seek failed: {}", e),
                                    }
                                }
                            }
                        }
                        Err(e) => error!("Failed to open file {:?}: {}", path, e),
                    }
                }

                // ── Seek ─────────────────────────────────────────
                AudioCommand::Seek(pos_ms) => {
                    if let Some(ds) = &mut dec_state {
                        // 1. Clear buffered audio so old samples don't play
                        if let Some(sk) = &sink {
                            sk.clear();
                            if !paused {
                                sk.play();
                            }
                        }

                        // 2. Update Instant-based position tracker
                        position_at_start_ms = pos_ms;
                        playback_start = if !paused {
                            Some(Instant::now())
                        } else {
                            None
                        };

                        // 3. Seek the symphonia FormatReader
                        let secs = pos_ms / 1000;
                        let frac = (pos_ms % 1000) as f64 / 1000.0;
                        match ds.format.seek(
                            SeekMode::Accurate,
                            SeekTo::Time {
                                time: Time::new(secs, frac),
                                track_id: None,
                            },
                        ) {
                            Ok(_) => {
                                // 4. Reset decoder to avoid glitches
                                ds.decoder.reset();
                                ds.sample_buf = None;
                                debug!("Seeked to {} ms", pos_ms);
                            }
                            Err(e) => error!("Seek failed: {}", e),
                        }
                    }
                }

                AudioCommand::Pause => {
                    // Freeze position at the current elapsed value
                    if let Some(start) = playback_start.take() {
                        position_at_start_ms += start.elapsed().as_millis() as u64;
                    }
                    paused = true;
                    if let Some(sk) = &sink {
                        sk.pause();
                    }
                }
                AudioCommand::Resume => {
                    paused = false;
                    playback_start = Some(Instant::now());
                    if let Some(sk) = &sink {
                        sk.play();
                    }
                }
                AudioCommand::Stop => {
                    if let Some(sk) = &sink {
                        sk.clear();
                        sk.pause();
                    }
                    dec_state = None;
                    paused = false;
                    playback_start = None;
                    position_at_start_ms = 0;
                    shared.position_ms.store(0, Ordering::SeqCst);
                    shared.is_playing.store(false, Ordering::SeqCst);
                    shared.is_finished.store(true, Ordering::SeqCst);
                }
                AudioCommand::SetVolume(v) => {
                    volume = v;
                    if let Some(sk) = &sink {
                        sk.set_volume(AudioPlayer::cubic_volume(v));
                    }
                }
            }
            continue; // drain all pending commands before decoding
        }

        // ── 2. Update position via Instant-based tracking ───────
        //    Instant elapsed since last play/seek = true playback position.
        //    This avoids rodio's Sink::get_pos() which resets to 0 for
        //    every new SamplesBuffer source appended to the queue.
        {
            let current_pos = if let Some(start) = &playback_start {
                position_at_start_ms + start.elapsed().as_millis() as u64
            } else {
                position_at_start_ms
            };
            let dur = shared.duration_ms.load(Ordering::SeqCst);
            let clamped = if dur > 0 { current_pos.min(dur) } else { current_pos };
            shared.position_ms.store(clamped, Ordering::SeqCst);
        }

        // ── 3. Detect natural end-of-track (sink fully drained) ─
        if dec_state.is_none() && !shared.is_finished.load(Ordering::SeqCst) {
            if sink.as_ref().map(|s| s.empty()).unwrap_or(true) {
                // Freeze position at duration for clean UI
                let dur = shared.duration_ms.load(Ordering::SeqCst);
                if dur > 0 {
                    shared.position_ms.store(dur, Ordering::SeqCst);
                }
                playback_start = None; // stop Instant from advancing past end
                position_at_start_ms = dur;
                shared.is_playing.store(false, Ordering::SeqCst);
                shared.is_finished.store(true, Ordering::SeqCst);
                info!("Track fully played out");
                // Notify listeners (Tauri event bridge)
                let _ = event_tx.send(PlayerEvent::TrackEnded);
            }
            continue;
        }

        // ── 4. Skip decoding when paused or idle ────────────────
        if paused || dec_state.is_none() {
            continue;
        }

        // ── 5. Decode one packet ────────────────────────────────
        let ds = dec_state.as_mut().unwrap();

        match ds.format.next_packet() {
            Ok(packet) => {
                // Skip packets belonging to other tracks in the container
                if packet.track_id() != ds.track_id {
                    continue;
                }

                match ds.decoder.decode(&packet) {
                    Ok(decoded) => {
                        let spec = *decoded.spec();
                        let channels = spec.channels.count() as u16;
                        let sr = spec.rate;

                        // Ensure the sample buffer has enough capacity
                        if ds.sample_buf.is_none() {
                            ds.sample_buf = Some(SampleBuffer::<f32>::new(
                                decoded.capacity() as u64,
                                spec,
                            ));
                        }

                        let sbuf = ds.sample_buf.as_mut().unwrap();
                        sbuf.copy_interleaved_ref(decoded);
                        let samples = sbuf.samples().to_vec();

                        if let Some(sk) = &sink {
                            sk.append(SamplesBuffer::new(channels, sr, samples));
                        }
                    }
                    Err(SymphoniaError::DecodeError(msg)) => {
                        warn!("Skipping malformed audio packet: {}", msg);
                    }
                    Err(e) => {
                        error!("Decode error: {}", e);
                        dec_state = None;
                    }
                }
            }
            // End of stream — decoder exhausted; let the Sink drain
            Err(e) => {
                let is_eof = matches!(
                    &e,
                    SymphoniaError::IoError(io_err)
                        if io_err.kind() == std::io::ErrorKind::UnexpectedEof
                );
                if is_eof {
                    info!(
                        "Decoder reached end of stream, waiting for sink to drain"
                    );
                } else {
                    error!("Format read error: {}", e);
                }
                dec_state = None;
                // Don't mark finished yet — the Sink still has buffered audio
            }
        }
    }
}

// ── Helper functions for the decoder thread ─────────────────────────

/// Ensure the audio output sink exists (create lazily, persist across tracks).
fn ensure_sink(
    stream: &mut Option<OutputStream>,
    sink: &mut Option<Sink>,
    volume: f32,
) {
    if sink.is_some() {
        return;
    }
    match OutputStream::try_default() {
        Ok((s, handle)) => match Sink::try_new(&handle) {
            Ok(sk) => {
                sk.set_volume(AudioPlayer::cubic_volume(volume));
                *stream = Some(s);
                *sink = Some(sk);
                debug!("Audio output initialized");
            }
            Err(e) => error!("Failed to create audio sink: {}", e),
        },
        Err(e) => error!("Failed to open audio output: {}", e),
    }
}

/// Probe a media source and create a symphonia decoder for the first
/// supported audio track.
fn create_decoder(
    source: Box<dyn symphonia::core::io::MediaSource>,
    mime_hint: Option<&str>,
    shared: &SharedState,
) -> Option<DecoderState> {
    let mss = MediaSourceStream::new(source, Default::default());
    let mut hint = Hint::new();
    if let Some(mime) = mime_hint {
        hint.mime_type(mime);
    }

    let probed = match symphonia::default::get_probe().format(
        &hint,
        mss,
        &FormatOptions {
            enable_gapless: true,
            ..Default::default()
        },
        &MetadataOptions::default(),
    ) {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to probe audio format: {}", e);
            return None;
        }
    };

    // Find the first audio track with a known codec
    let track = probed
        .format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)?;

    let track_id = track.id;

    // Only compute duration from symphonia if no external duration was set
    // (e.g., from Qobuz API). This avoids potential time_base mismatches
    // between the format reader and decoder.
    let existing_dur = shared.duration_ms.load(Ordering::SeqCst);
    if existing_dur == 0 {
        if let (Some(tb), Some(nf)) =
            (track.codec_params.time_base, track.codec_params.n_frames)
        {
            let time = tb.calc_time(nf);
            let dur = ((time.seconds as f64 + time.frac) * 1000.0) as u64;
            shared.duration_ms.store(dur, Ordering::SeqCst);
            debug!("Duration from symphonia: {}ms (tb={:?}, nf={})", dur, tb, nf);
        }
    } else {
        debug!("Using pre-set duration: {}ms", existing_dur);
    }

    let decoder = match symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
    {
        Ok(dec) => dec,
        Err(e) => {
            error!("Failed to create audio decoder: {}", e);
            return None;
        }
    };

    shared.is_finished.store(false, Ordering::SeqCst);
    debug!("Symphonia decoder initialized (track_id={})", track_id);

    Some(DecoderState {
        format: probed.format,
        decoder,
        track_id,
        sample_buf: None,
    })
}

// ── Unit Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    // ── Position tracking logic ──────────────────────────────────────

    /// Position must advance while playback_start is Some.
    #[test]
    fn test_position_advances_while_playing() {
        let start = Instant::now();
        thread::sleep(Duration::from_millis(100));

        let position_at_start_ms: u64 = 0;
        let elapsed = start.elapsed().as_millis() as u64;
        let pos = position_at_start_ms + elapsed;

        assert!(
            pos >= 80,
            "Position should have advanced ~100ms, got {}ms",
            pos
        );
    }

    /// When paused (playback_start = None), position must not advance.
    #[test]
    fn test_position_frozen_when_paused() {
        let position_at_start_ms: u64 = 12_345;
        let playback_start: Option<Instant> = None;

        let current_pos = if let Some(start) = &playback_start {
            position_at_start_ms + start.elapsed().as_millis() as u64
        } else {
            position_at_start_ms
        };

        assert_eq!(
            current_pos, 12_345,
            "Paused position must stay at 12345ms"
        );
    }

    /// After a seek, position_at_start_ms reflects the seek target.
    #[test]
    fn test_seek_updates_position_at_start() {
        let seek_target_ms: u64 = 45_000; // 45 seconds into the track
        let position_at_start_ms = seek_target_ms;
        // Simulate: playback just resumed from seek position
        let playback_start: Option<Instant> = Some(Instant::now());

        let current_pos = if let Some(start) = &playback_start {
            position_at_start_ms + start.elapsed().as_millis() as u64
        } else {
            position_at_start_ms
        };

        // Should be very close to seek_target (within 50ms overhead)
        assert!(
            current_pos >= seek_target_ms && current_pos <= seek_target_ms + 50,
            "Immediately after seek, position should be ~{}ms, got {}ms",
            seek_target_ms,
            current_pos
        );
    }

    /// Position must be clamped to track duration (no overshoot).
    #[test]
    fn test_position_clamped_to_duration() {
        let duration_ms: u64 = 180_000; // 3 minutes
        let raw_pos: u64 = 200_000; // beyond end

        let clamped = if duration_ms > 0 {
            raw_pos.min(duration_ms)
        } else {
            raw_pos
        };

        assert_eq!(clamped, 180_000, "Position must be clamped to duration");
    }

    /// Duration = 0 means no clamping (unknown duration).
    #[test]
    fn test_no_clamp_when_duration_unknown() {
        let duration_ms: u64 = 0;
        let raw_pos: u64 = 5_000;

        let clamped = if duration_ms > 0 {
            raw_pos.min(duration_ms)
        } else {
            raw_pos
        };

        assert_eq!(clamped, 5_000, "With unknown duration, position is unclipped");
    }

    /// Pause then resume: elapsed time before pause is preserved.
    #[test]
    fn test_pause_resume_accumulates_position() {
        // Simulate 200ms of playback, then pause
        let mut position_at_start_ms: u64 = 0;
        let start = Instant::now();
        thread::sleep(Duration::from_millis(200));

        // Pause: save elapsed into position_at_start_ms
        position_at_start_ms += start.elapsed().as_millis() as u64;
        let position_after_pause = position_at_start_ms;

        // Resume: new Instant starts from position_after_pause
        let resume_start = Instant::now();
        thread::sleep(Duration::from_millis(100));
        let pos_after_resume =
            position_at_start_ms + resume_start.elapsed().as_millis() as u64;

        assert!(
            position_after_pause >= 150,
            "After 200ms play, position should be ≥150ms, got {}ms",
            position_after_pause
        );
        assert!(
            pos_after_resume >= position_after_pause + 50,
            "After 100ms more, position should have advanced from {}ms, got {}ms",
            position_after_pause,
            pos_after_resume
        );
    }

    // ── Volume curve ─────────────────────────────────────────────────

    #[test]
    fn test_cubic_volume_extremes() {
        assert_eq!(AudioPlayer::cubic_volume(0.0), 0.0);
        assert_eq!(AudioPlayer::cubic_volume(1.0), 1.0);
    }

    #[test]
    fn test_cubic_volume_midpoint() {
        let vol = AudioPlayer::cubic_volume(0.5);
        assert!(
            (vol - 0.125).abs() < 0.001,
            "cubic(0.5) = 0.125, got {}",
            vol
        );
    }

    #[test]
    fn test_cubic_volume_perceptual_curve() {
        // Cubic curve: lower values are quieter than linear
        let linear_half = 0.5_f32;
        let cubic_half = AudioPlayer::cubic_volume(linear_half);
        assert!(
            cubic_half < linear_half,
            "Cubic volume at 0.5 ({}) must be less than linear (0.5)",
            cubic_half
        );
    }

    // ── Duration from track metadata ─────────────────────────────────

    #[test]
    fn test_duration_from_duration_seconds() {
        let duration_seconds: i32 = 247; // 4:07
        let duration_ms = (duration_seconds as u64) * 1000;
        assert_eq!(duration_ms, 247_000);
    }

    // ── Time formatting (mirrors frontend formatTime) ─────────────────

    fn format_time_ms(ms: u64) -> String {
        let s = ms / 1000;
        let m = s / 60;
        let sec = s % 60;
        format!("{}:{:02}", m, sec)
    }

    #[test]
    fn test_format_time_zero() {
        assert_eq!(format_time_ms(0), "0:00");
    }

    #[test]
    fn test_format_time_one_minute() {
        assert_eq!(format_time_ms(60_000), "1:00");
    }

    #[test]
    fn test_format_time_mixed() {
        assert_eq!(format_time_ms(90_500), "1:30");
        assert_eq!(format_time_ms(245_000), "4:05");
    }

    #[test]
    fn test_format_time_long_track() {
        assert_eq!(format_time_ms(3_661_000), "61:01");
    }

    // ── Progress bar percentage ───────────────────────────────────────

    fn progress_pct(position_ms: u64, duration_ms: u64) -> f64 {
        if duration_ms > 0 {
            (position_ms as f64 / duration_ms as f64) * 100.0
        } else {
            0.0
        }
    }

    #[test]
    fn test_progress_at_start() {
        assert_eq!(progress_pct(0, 60_000), 0.0);
    }

    #[test]
    fn test_progress_at_end() {
        assert_eq!(progress_pct(60_000, 60_000), 100.0);
    }

    #[test]
    fn test_progress_midpoint() {
        assert!((progress_pct(30_000, 60_000) - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_progress_zero_duration_is_zero() {
        assert_eq!(progress_pct(5_000, 0), 0.0);
    }
}
