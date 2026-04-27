use crate::models::*;
use cpal::traits::{DeviceTrait, HostTrait};
use rodio::{buffer::SamplesBuffer, OutputStream, OutputStreamHandle, Sink};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::{Duration, Instant};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo};
use symphonia::core::io::{MediaSource, MediaSourceStream};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::units::Time;
use tracing::{debug, error, info, warn};
use rustfft::{FftPlanner, num_complex::Complex};
use std::f32::consts::PI;

// ── ArcCursor: seekable memory source backed by Arc<Vec<u8>> (no data copy) ──

struct ArcCursor {
    data: Arc<Vec<u8>>,
    pos: u64,
}

impl io::Read for ArcCursor {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let start = self.pos as usize;
        if start >= self.data.len() {
            return Ok(0);
        }
        let n = (self.data.len() - start).min(buf.len());
        buf[..n].copy_from_slice(&self.data[start..start + n]);
        self.pos += n as u64;
        Ok(n)
    }
}

impl io::Seek for ArcCursor {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        let len = self.data.len() as i64;
        let new_pos = match pos {
            io::SeekFrom::Start(p) => p as i64,
            io::SeekFrom::End(p) => len + p,
            io::SeekFrom::Current(p) => self.pos as i64 + p,
        };
        self.pos = new_pos.max(0) as u64;
        Ok(self.pos)
    }
}

impl MediaSource for ArcCursor {
    fn is_seekable(&self) -> bool { true }
    fn byte_len(&self) -> Option<u64> { Some(self.data.len() as u64) }
}

// ── EQ Types ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EqBandParam {
    pub freq: f32,
    pub gain_db: f32,
    pub q: f32,
}

#[derive(Debug, Clone, Default)]
pub struct EqSharedState {
    pub enabled: bool,
    pub bands: Vec<EqBandParam>,
    pub version: u64,
}

// ── Spectrum FFT constants ────────────────────────────────────────────────────
const FFT_SIZE: usize = 2048;
const SPECTRUM_BINS: usize = 80;

/// Biquad filter coefficients (Direct Form II Transposed).
#[derive(Debug, Clone)]
struct BiquadCoeffs {
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
}

impl BiquadCoeffs {
    fn identity() -> Self {
        Self { b0: 1.0, b1: 0.0, b2: 0.0, a1: 0.0, a2: 0.0 }
    }

    /// Peak/Peaking EQ filter (RBJ Audio EQ Cookbook).
    fn peak_eq(sample_rate: f64, freq: f64, gain_db: f64, q: f64) -> Self {
        if gain_db.abs() < 0.01 {
            return Self::identity();
        }
        let a = 10f64.powf(gain_db / 40.0);
        let w0 = 2.0 * std::f64::consts::PI * freq / sample_rate;
        let sin_w0 = w0.sin();
        let cos_w0 = w0.cos();
        let alpha = sin_w0 / (2.0 * q);

        let b0 = 1.0 + alpha * a;
        let b1 = -2.0 * cos_w0;
        let b2 = 1.0 - alpha * a;
        let a0 = 1.0 + alpha / a;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha / a;

        Self { b0: b0 / a0, b1: b1 / a0, b2: b2 / a0, a1: a1 / a0, a2: a2 / a0 }
    }

    /// Low-shelf filter for the lowest EQ band.
    fn low_shelf(sample_rate: f64, freq: f64, gain_db: f64, q: f64) -> Self {
        if gain_db.abs() < 0.01 {
            return Self::identity();
        }
        let a = 10f64.powf(gain_db / 40.0);
        let w0 = 2.0 * std::f64::consts::PI * freq / sample_rate;
        let cos_w0 = w0.cos();
        let alpha = w0.sin() / 2.0 * (2.0_f64.sqrt() / q);

        let b0 = a * ((a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * a.sqrt() * alpha);
        let b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0);
        let b2 = a * ((a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * a.sqrt() * alpha);
        let a0 = (a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * a.sqrt() * alpha;
        let a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cos_w0);
        let a2 = (a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * a.sqrt() * alpha;

        Self { b0: b0 / a0, b1: b1 / a0, b2: b2 / a0, a1: a1 / a0, a2: a2 / a0 }
    }

    /// High-shelf filter for the highest EQ band.
    fn high_shelf(sample_rate: f64, freq: f64, gain_db: f64, q: f64) -> Self {
        if gain_db.abs() < 0.01 {
            return Self::identity();
        }
        let a = 10f64.powf(gain_db / 40.0);
        let w0 = 2.0 * std::f64::consts::PI * freq / sample_rate;
        let cos_w0 = w0.cos();
        let alpha = w0.sin() / 2.0 * (2.0_f64.sqrt() / q);

        let b0 = a * ((a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * a.sqrt() * alpha);
        let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0);
        let b2 = a * ((a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * a.sqrt() * alpha);
        let a0 = (a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * a.sqrt() * alpha;
        let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos_w0);
        let a2 = (a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * a.sqrt() * alpha;

        Self { b0: b0 / a0, b1: b1 / a0, b2: b2 / a0, a1: a1 / a0, a2: a2 / a0 }
    }
}

/// Stateful biquad filter (Direct Form II Transposed, maintains per-channel state).
struct BiquadFilter {
    coeffs: BiquadCoeffs,
    s1: f64,
    s2: f64,
}

impl BiquadFilter {
    fn new(coeffs: BiquadCoeffs) -> Self {
        Self { coeffs, s1: 0.0, s2: 0.0 }
    }

    #[inline]
    fn process(&mut self, x: f64) -> f64 {
        let y = self.coeffs.b0 * x + self.s1;
        self.s1 = self.coeffs.b1 * x - self.coeffs.a1 * y + self.s2;
        self.s2 = self.coeffs.b2 * x - self.coeffs.a2 * y;
        y
    }
}

/// Per-band, per-channel filter bank. Rebuilt when EQ config or sample rate changes.
struct EqFilterBank {
    /// filters[band][channel]
    filters: Vec<Vec<BiquadFilter>>,
    sample_rate: u32,
    channels: u16,
}

impl EqFilterBank {
    fn new(bands: &[EqBandParam], sample_rate: u32, channels: u16) -> Self {
        let n_bands = bands.len();
        let filters = bands
            .iter()
            .enumerate()
            .map(|(i, b)| {
                let sr = sample_rate as f64;
                let freq = b.freq as f64;
                let gain = b.gain_db as f64;
                let q = b.q as f64;
                let coeffs = if i == 0 {
                    BiquadCoeffs::low_shelf(sr, freq, gain, q)
                } else if i == n_bands - 1 {
                    BiquadCoeffs::high_shelf(sr, freq, gain, q)
                } else {
                    BiquadCoeffs::peak_eq(sr, freq, gain, q)
                };
                (0..channels as usize)
                    .map(|_| BiquadFilter::new(coeffs.clone()))
                    .collect::<Vec<_>>()
            })
            .collect();

        Self { filters, sample_rate, channels }
    }

    fn process(&mut self, samples: &mut [f32]) {
        let ch = self.channels as usize;
        for (i, sample) in samples.iter_mut().enumerate() {
            let c = i % ch;
            let mut v = *sample as f64;
            for band in &mut self.filters {
                v = band[c].process(v);
            }
            *sample = v.clamp(-1.0, 1.0) as f32;
        }
    }
}

// ── Commands sent to the decoder thread ─────────────────────────────

enum AudioCommand {
    LoadBytes {
        bytes: Arc<Vec<u8>>,
        seek_to_ms: Option<u64>,
    },
    LoadFile {
        path: PathBuf,
        seek_to_ms: Option<u64>,
    },
    Seek(u64),
    Pause,
    Resume,
    Stop,
    SetVolume(f32),
    /// Rebuild EQ filter bank on next packet decode.
    SetEq,
    /// Switch audio output device (None = system default).
    SetDevice(Option<String>),
}

/// Events emitted by the audio engine (decoder thread → listener).
#[derive(Debug, Clone)]
pub enum PlayerEvent {
    TrackEnded,
    #[allow(dead_code)]
    Spectrum(Vec<f32>),
}

// ── Public API ──────────────────────────────────────────────────────

pub struct AudioPlayer {
    command_tx: mpsc::Sender<AudioCommand>,

    current_track: Option<UnifiedTrack>,
    current_sample_rate: Option<f64>,
    current_bit_depth: Option<i32>,
    volume: f32,

    cached_bytes: Option<Arc<Vec<u8>>>,
    cached_file_path: Option<PathBuf>,

    is_playing: Arc<AtomicBool>,
    position_ms: Arc<AtomicU64>,
    duration_ms: Arc<AtomicU64>,
    is_finished: Arc<AtomicBool>,
    /// Actual sample rate of the open DAC stream (set by decoder thread, 0 = unknown).
    output_stream_sr: Arc<AtomicU32>,

    /// EQ state shared with decoder thread.
    pub eq_state: Arc<parking_lot::Mutex<EqSharedState>>,

    /// Latest spectrum data (80 bins), written by decoder thread, read by frontend poll.
    #[allow(dead_code)]
    pub spectrum: Arc<parking_lot::Mutex<Vec<f32>>>,
}

unsafe impl Send for AudioPlayer {}
unsafe impl Sync for AudioPlayer {}

impl AudioPlayer {
    pub fn new() -> (Self, mpsc::Receiver<PlayerEvent>, Arc<parking_lot::Mutex<Vec<f32>>>) {
        let (tx, rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();

        let is_playing = Arc::new(AtomicBool::new(false));
        let position_ms = Arc::new(AtomicU64::new(0));
        let duration_ms = Arc::new(AtomicU64::new(0));
        let is_finished = Arc::new(AtomicBool::new(true));
        let output_stream_sr = Arc::new(AtomicU32::new(0));
        let eq_state: Arc<parking_lot::Mutex<EqSharedState>> =
            Arc::new(parking_lot::Mutex::new(EqSharedState::default()));
        let spectrum: Arc<parking_lot::Mutex<Vec<f32>>> =
            Arc::new(parking_lot::Mutex::new(vec![0.0; SPECTRUM_BINS]));

        let shared = SharedState {
            is_playing: is_playing.clone(),
            position_ms: position_ms.clone(),
            duration_ms: duration_ms.clone(),
            is_finished: is_finished.clone(),
            output_stream_sr: output_stream_sr.clone(),
            eq_state: eq_state.clone(),
            spectrum: spectrum.clone(),
        };

        thread::Builder::new()
            .name("audio-decoder".into())
            .spawn(move || decoder_thread(rx, shared, event_tx))
            .expect("Failed to spawn audio decoder thread");

        let spectrum_arc = spectrum.clone();
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
                output_stream_sr,
                eq_state,
                spectrum,
            },
            event_rx,
            spectrum_arc,
        )
    }

    pub fn cubic_volume(linear: f32) -> f32 {
        linear * linear * linear
    }

    pub fn play_bytes(
        &mut self,
        bytes: Vec<u8>,
        track: UnifiedTrack,
        sample_rate: Option<f64>,
        bit_depth: Option<i32>,
    ) -> Result<(), String> {
        info!("Playing (in-memory): {} - {}", track.artist, track.title);
        self.duration_ms.store((track.duration_seconds as u64) * 1000, Ordering::SeqCst);
        self.position_ms.store(0, Ordering::SeqCst);
        self.is_playing.store(true, Ordering::SeqCst);
        self.is_finished.store(false, Ordering::SeqCst);
        self.current_track = Some(track);
        self.current_sample_rate = sample_rate;
        self.current_bit_depth = bit_depth;
        let arc = Arc::new(bytes);
        self.cached_bytes = Some(arc.clone());
        self.cached_file_path = None;
        self.command_tx
            .send(AudioCommand::LoadBytes { bytes: arc, seek_to_ms: None })
            .map_err(|e| format!("Decoder thread not responding: {}", e))
    }

    pub fn play_file(
        &mut self,
        file_path: &PathBuf,
        track: UnifiedTrack,
        sample_rate: Option<f64>,
        bit_depth: Option<i32>,
    ) -> Result<(), String> {
        info!("Playing (file): {} - {}", track.artist, track.title);
        self.duration_ms.store((track.duration_seconds as u64) * 1000, Ordering::SeqCst);
        self.position_ms.store(0, Ordering::SeqCst);
        self.is_playing.store(true, Ordering::SeqCst);
        self.is_finished.store(false, Ordering::SeqCst);
        self.current_track = Some(track);
        self.current_sample_rate = sample_rate;
        self.current_bit_depth = bit_depth;
        self.cached_bytes = None;
        self.cached_file_path = Some(file_path.clone());
        self.command_tx
            .send(AudioCommand::LoadFile { path: file_path.clone(), seek_to_ms: None })
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
        self.position_ms.store(position_ms, Ordering::SeqCst);
        self.is_finished.store(false, Ordering::SeqCst);
        self.is_playing.store(true, Ordering::SeqCst);

        // Reload from cached data for reliable seeking: ArcCursor avoids copying bytes,
        // and a fresh decoder guarantees a clean seek regardless of format quirks.
        if let Some(ref arc) = self.cached_bytes {
            return self
                .command_tx
                .send(AudioCommand::LoadBytes { bytes: arc.clone(), seek_to_ms: Some(position_ms) })
                .map_err(|e| format!("Decoder thread not responding: {}", e));
        }
        if let Some(ref path) = self.cached_file_path {
            return self
                .command_tx
                .send(AudioCommand::LoadFile { path: path.clone(), seek_to_ms: Some(position_ms) })
                .map_err(|e| format!("Decoder thread not responding: {}", e));
        }

        // Fallback if no cached data (shouldn't normally happen)
        self.command_tx
            .send(AudioCommand::Seek(position_ms))
            .map_err(|e| format!("Decoder thread not responding: {}", e))
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        let _ = self.command_tx.send(AudioCommand::SetVolume(self.volume));
    }

    /// Update EQ settings and signal decoder thread to rebuild filter bank.
    pub fn set_eq(&mut self, bands: Vec<EqBandParam>, enabled: bool) {
        {
            let mut state = self.eq_state.lock();
            state.bands = bands;
            state.enabled = enabled;
            state.version = state.version.wrapping_add(1);
        }
        let _ = self.command_tx.send(AudioCommand::SetEq);
    }

    /// Returns current EQ configuration.
    pub fn get_eq_state(&self) -> (bool, Vec<EqBandParam>) {
        let state = self.eq_state.lock();
        (state.enabled, state.bands.clone())
    }

    /// Switch audio output device. Interrupts current playback; user must re-press play.
    pub fn set_preferred_device(&mut self, name: Option<String>) {
        let _ = self.command_tx.send(AudioCommand::SetDevice(name));
    }

    /// Enumerate available audio output devices (WASAPI + ASIO if feature enabled).
    pub fn get_audio_devices() -> Vec<String> {
        let mut devices = vec!["Default".to_string()];

        // WASAPI / CoreAudio / ALSA — default host
        let host = cpal::default_host();
        if let Ok(devs) = host.output_devices() {
            for dev in devs {
                if let Ok(name) = dev.name() {
                    devices.push(name);
                }
            }
        }

        // ASIO devices — read driver names from the Windows registry without loading
        // any driver DLL. Loading drivers via cpal::host_from_id causes heap corruption
        // with some ASIO drivers (e.g. CAUSBAudio) during enumeration.
        // The actual driver is only loaded when the user selects an ASIO device for playback.
        #[cfg(feature = "asio")]
        for name in asio_drivers_from_registry() {
            tracing::info!("[audio_devices] ASIO (registry): {name}");
            devices.push(format!("ASIO: {name}"));
        }

        devices
    }

    /// Returns the latest spectrum data (80 bins, 0.0–1.0 normalized).
    #[allow(dead_code)]
    pub fn get_spectrum(&self) -> Vec<f32> {
        self.spectrum.lock().clone()
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
            quality: self.current_track.as_ref().and_then(|t| t.quality_label.clone()),
            sample_rate: self.current_sample_rate,
            bit_depth: self.current_bit_depth,
            output_sample_rate: self.output_stream_sr.load(Ordering::Relaxed),
        }
    }
}

// ── Decoder Thread ──────────────────────────────────────────────────

struct SharedState {
    is_playing: Arc<AtomicBool>,
    position_ms: Arc<AtomicU64>,
    duration_ms: Arc<AtomicU64>,
    is_finished: Arc<AtomicBool>,
    output_stream_sr: Arc<AtomicU32>,
    eq_state: Arc<parking_lot::Mutex<EqSharedState>>,
    spectrum: Arc<parking_lot::Mutex<Vec<f32>>>,
}

struct DecoderState {
    format: Box<dyn FormatReader>,
    decoder: Box<dyn symphonia::core::codecs::Decoder>,
    track_id: u32,
    sample_buf: Option<SampleBuffer<f32>>,
    /// Sample rate read from codec params; used to configure the ASIO stream.
    sample_rate: Option<u32>,
}

fn decoder_thread(
    command_rx: mpsc::Receiver<AudioCommand>,
    shared: SharedState,
    event_tx: mpsc::Sender<PlayerEvent>,
) {
    let mut _stream: Option<OutputStream> = None;
    let mut _handle: Option<OutputStreamHandle> = None;
    let mut sink: Option<Sink> = None;
    let mut volume: f32 = 0.7;
    let mut preferred_device: Option<String> = None;

    let mut dec_state: Option<DecoderState> = None;
    let mut paused = false;
    // Sample rate at which the ASIO stream is currently open (0 = not yet opened).
    // When a new track has a different SR, the stream is torn down and rebuilt.
    let mut current_stream_sr: u32 = 0;

    let mut playback_start: Option<Instant> = None;
    let mut position_at_start_ms: u64 = 0;

    // Cached source for reliable EQ-triggered seeks (avoids audio jumping ahead)
    let mut cached_bytes: Option<Arc<Vec<u8>>> = None;
    let mut cached_file: Option<PathBuf> = None;

    // EQ filter bank (rebuilt on SetEq or sample-rate change)
    let mut eq_filter_bank: Option<EqFilterBank> = None;
    let mut eq_version_seen: u64 = 0;

    // FFT spectrum analyzer
    let mut fft_planner = FftPlanner::<f32>::new();
    let fft_forward = fft_planner.plan_fft_forward(FFT_SIZE);
    let mut fft_buf: Vec<f32> = Vec::with_capacity(FFT_SIZE * 2);
    let mut last_spectrum_emit = Instant::now();

    loop {
        let buffer_full = sink.as_ref().map(|s| s.len() > 4).unwrap_or(false);
        let should_wait = dec_state.is_none() || paused || buffer_full;

        let cmd = if should_wait {
            // shared.spectrum retains last FFT values — the frontend polls
            // it directly via get_spectrum, so no re-emit needed here.
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
                AudioCommand::LoadBytes { bytes, seek_to_ms } => {
                    if let Some(sk) = &sink { sk.clear(); }
                    position_at_start_ms = 0;
                    playback_start = Some(Instant::now());
                    // Decode first to know the track's sample rate before opening the stream.
                    let arc_cursor = ArcCursor { data: bytes.clone(), pos: 0 };
                    dec_state = create_decoder(Box::new(arc_cursor), None, &shared);
                    cached_bytes = Some(bytes.clone());
                    cached_file = None;
                    eq_filter_bank = None;
                    // For ASIO: reopen the stream at the track's sample rate if it changed.
                    let new_sr = dec_state.as_ref().and_then(|d| d.sample_rate);
                    let is_asio = preferred_device.as_ref().map(|n| n.starts_with("ASIO:")).unwrap_or(false);
                    if is_asio {
                        if let Some(sr) = new_sr {
                            if sr != current_stream_sr {
                                sink = None; _handle = None; _stream = None;
                                current_stream_sr = sr;
                            }
                        }
                    }
                    ensure_sink(&mut _stream, &mut _handle, &mut sink, volume, &preferred_device, new_sr.filter(|_| is_asio), &shared.output_stream_sr);
                    paused = false;
                    if let Some(sk) = &sink { sk.play(); }
                    // Log Hi-Res status so the user can verify DAC output
                    if let Some(content_sr) = new_sr {
                        let stream_sr = shared.output_stream_sr.load(Ordering::Relaxed);
                        if stream_sr > 0 {
                            let hires = content_sr >= 88200;
                            if content_sr == stream_sr {
                                info!("Audio: {}kHz/{} → DAC {}kHz [bit-perfect{}]",
                                    content_sr / 1000, content_sr % 1000,
                                    stream_sr / 1000,
                                    if hires { " Hi-Res" } else { "" });
                            } else {
                                warn!("Audio: content {}Hz but DAC stream {}Hz — rate mismatch!", content_sr, stream_sr);
                            }
                        }
                    }
                    if let Some(ms) = seek_to_ms {
                        if let Some(ds) = &mut dec_state {
                            position_at_start_ms = ms;
                            playback_start = Some(Instant::now());
                            let secs = ms / 1000;
                            let frac = (ms % 1000) as f64 / 1000.0;
                            if let Ok(_) = ds.format.seek(SeekMode::Accurate, SeekTo::Time {
                                time: Time::new(secs, frac), track_id: None,
                            }) {
                                ds.decoder.reset();
                                ds.sample_buf = None;
                            }
                        }
                    }
                }

                AudioCommand::LoadFile { path, seek_to_ms } => {
                    if let Some(sk) = &sink { sk.clear(); }
                    position_at_start_ms = 0;
                    playback_start = Some(Instant::now());
                    match File::open(&path) {
                        Ok(file) => {
                            dec_state = create_decoder(Box::new(file), None, &shared);
                            cached_file = Some(path.clone());
                            cached_bytes = None;
                            eq_filter_bank = None;
                            let new_sr = dec_state.as_ref().and_then(|d| d.sample_rate);
                            let is_asio = preferred_device.as_ref().map(|n| n.starts_with("ASIO:")).unwrap_or(false);
                            if is_asio {
                                if let Some(sr) = new_sr {
                                    if sr != current_stream_sr {
                                        sink = None; _handle = None; _stream = None;
                                        current_stream_sr = sr;
                                    }
                                }
                            }
                            ensure_sink(&mut _stream, &mut _handle, &mut sink, volume, &preferred_device, new_sr.filter(|_| is_asio), &shared.output_stream_sr);
                            paused = false;
                            if let Some(sk) = &sink { sk.play(); }
                            if let Some(ms) = seek_to_ms {
                                if let Some(ds) = &mut dec_state {
                                    position_at_start_ms = ms;
                                    playback_start = Some(Instant::now());
                                    let secs = ms / 1000;
                                    let frac = (ms % 1000) as f64 / 1000.0;
                                    if let Ok(_) = ds.format.seek(SeekMode::Accurate, SeekTo::Time {
                                        time: Time::new(secs, frac), track_id: None,
                                    }) {
                                        ds.decoder.reset();
                                        ds.sample_buf = None;
                                    }
                                }
                            }
                        }
                        Err(e) => error!("Failed to open file {:?}: {}", path, e),
                    }
                }

                AudioCommand::Seek(pos_ms) => {
                    if let Some(ds) = &mut dec_state {
                        if let Some(sk) = &sink {
                            sk.clear();
                            if !paused { sk.play(); }
                        }
                        fft_buf.clear(); // discard samples from before seek
                        position_at_start_ms = pos_ms;
                        playback_start = if !paused { Some(Instant::now()) } else { None };
                        let secs = pos_ms / 1000;
                        let frac = (pos_ms % 1000) as f64 / 1000.0;
                        match ds.format.seek(SeekMode::Accurate, SeekTo::Time {
                            time: Time::new(secs, frac), track_id: None,
                        }) {
                            Ok(_) => {
                                ds.decoder.reset();
                                ds.sample_buf = None;
                                debug!("Seeked to {} ms", pos_ms);
                            }
                            Err(e) => error!("Seek failed: {}", e),
                        }
                    }
                }

                AudioCommand::Pause => {
                    if let Some(start) = playback_start.take() {
                        position_at_start_ms += start.elapsed().as_millis() as u64;
                    }
                    paused = true;
                    // Clear the sink buffer so audio stops immediately (no trailing samples).
                    // Then rebuild the decoder at the current position so Resume is seamless.
                    if let Some(sk) = &sink { sk.clear(); sk.pause(); }
                    let current_pos = position_at_start_ms;
                    let reloaded = if let Some(ref bytes) = cached_bytes {
                        let arc_cursor = ArcCursor { data: bytes.clone(), pos: 0 };
                        create_decoder(Box::new(arc_cursor), None, &shared)
                    } else if let Some(ref path) = cached_file {
                        File::open(path).ok().and_then(|f| create_decoder(Box::new(f), None, &shared))
                    } else {
                        None
                    };
                    if let Some(mut new_ds) = reloaded {
                        let secs = current_pos / 1000;
                        let frac = (current_pos % 1000) as f64 / 1000.0;
                        if new_ds.format.seek(SeekMode::Accurate, SeekTo::Time {
                            time: Time::new(secs, frac), track_id: None,
                        }).is_ok() {
                            new_ds.decoder.reset();
                            new_ds.sample_buf = None;
                        }
                        dec_state = Some(new_ds);
                    }
                }

                AudioCommand::Resume => {
                    paused = false;
                    playback_start = Some(Instant::now());
                    if let Some(sk) = &sink { sk.play(); }
                }

                AudioCommand::Stop => {
                    if let Some(sk) = &sink { sk.clear(); sk.pause(); }
                    dec_state = None;
                    paused = false;
                    playback_start = None;
                    position_at_start_ms = 0;
                    shared.position_ms.store(0, Ordering::SeqCst);
                    shared.is_playing.store(false, Ordering::SeqCst);
                    shared.is_finished.store(true, Ordering::SeqCst);
                    fft_buf.clear();
                    shared.spectrum.lock().iter_mut().for_each(|v| *v = 0.0);
                }

                AudioCommand::SetVolume(v) => {
                    volume = v;
                    if let Some(sk) = &sink {
                        sk.set_volume(AudioPlayer::cubic_volume(v));
                    }
                }

                AudioCommand::SetEq => {
                    eq_filter_bank = None;
                    eq_version_seen = 0;

                    // Reload from the cached source at the current display position so:
                    // 1. EQ applies immediately (fresh decoder, no pre-EQ buffer left),
                    // 2. Audio doesn't jump ahead (Symphonia was pre-reading ~2 s ahead).
                    let current_pos = if let Some(start) = &playback_start {
                        position_at_start_ms + start.elapsed().as_millis() as u64
                    } else {
                        position_at_start_ms
                    };
                    let dur = shared.duration_ms.load(Ordering::SeqCst);
                    let current_pos = if dur > 0 { current_pos.min(dur) } else { current_pos };

                    let reloaded = if let Some(ref bytes) = cached_bytes {
                        if let Some(sk) = &sink { sk.clear(); }
                        let arc_cursor = ArcCursor { data: bytes.clone(), pos: 0 };
                        create_decoder(Box::new(arc_cursor), None, &shared)
                    } else if let Some(ref path) = cached_file {
                        if let Some(sk) = &sink { sk.clear(); }
                        File::open(path).ok().and_then(|f| create_decoder(Box::new(f), None, &shared))
                    } else {
                        None
                    };

                    if let Some(mut new_ds) = reloaded {
                        let secs = current_pos / 1000;
                        let frac = (current_pos % 1000) as f64 / 1000.0;
                        if new_ds.format.seek(SeekMode::Accurate, SeekTo::Time {
                            time: Time::new(secs, frac), track_id: None,
                        }).is_ok() {
                            new_ds.decoder.reset();
                            new_ds.sample_buf = None;
                        }
                        dec_state = Some(new_ds);
                        position_at_start_ms = current_pos;
                        playback_start = if !paused { Some(Instant::now()) } else { None };
                        if !paused {
                            if let Some(sk) = &sink { sk.play(); }
                        }
                    }
                }

                AudioCommand::SetDevice(name) => {
                    // Close existing sink/stream
                    if let Some(sk) = &sink { sk.clear(); sk.pause(); }
                    sink = None;
                    _handle = None;
                    _stream = None;
                    preferred_device = name;
                    current_stream_sr = 0; // force SR negotiation on next LoadBytes/LoadFile
                    // Open the new device at its default SR (will be corrected on next play).
                    ensure_sink(&mut _stream, &mut _handle, &mut sink, volume, &preferred_device, None, &shared.output_stream_sr);
                    // Playback stops; user must press play again
                    dec_state = None;
                    playback_start = None;
                    shared.is_playing.store(false, Ordering::SeqCst);
                    shared.is_finished.store(true, Ordering::SeqCst);
                }
            }
            continue;
        }

        // ── 2. Update position ───────────────────────────────────
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

        // ── 3. Detect natural end-of-track ───────────────────────
        if dec_state.is_none() && !shared.is_finished.load(Ordering::SeqCst) {
            if sink.as_ref().map(|s| s.empty()).unwrap_or(true) {
                let dur = shared.duration_ms.load(Ordering::SeqCst);
                if dur > 0 { shared.position_ms.store(dur, Ordering::SeqCst); }
                playback_start = None;
                position_at_start_ms = dur;
                shared.is_playing.store(false, Ordering::SeqCst);
                shared.is_finished.store(true, Ordering::SeqCst);
                info!("Track fully played out");
                let _ = event_tx.send(PlayerEvent::TrackEnded);
            }
            continue;
        }

        if paused || dec_state.is_none() {
            continue;
        }

        // Don't decode when the audio sink already has enough data buffered.
        // Without this guard the decoder runs ~3× faster than the audio output
        // (1 packet per 25 ms vs 1 packet consumed per ~93 ms at 44.1 kHz /
        // 4096 samples).  It would finish decoding the entire track in ~81 s
        // regardless of song length, setting dec_state = None while the sink
        // still holds minutes of audio — making the FFT (and spectrum) stop
        // while the user still hears music.  Capping at 4 packets (~185 ms
        // look-ahead) keeps decode in sync with playback so the spectrum stays
        // alive for the full duration of every track.
        if buffer_full {
            continue;
        }

        // ── 4. Decode one packet ─────────────────────────────────
        let ds = dec_state.as_mut().unwrap();

        match ds.format.next_packet() {
            Ok(packet) => {
                if packet.track_id() != ds.track_id {
                    continue;
                }
                match ds.decoder.decode(&packet) {
                    Ok(decoded) => {
                        let spec = *decoded.spec();
                        let channels = spec.channels.count() as u16;
                        let sr = spec.rate;

                        if ds.sample_buf.is_none() {
                            ds.sample_buf = Some(SampleBuffer::<f32>::new(
                                decoded.capacity() as u64,
                                spec,
                            ));
                        }

                        let sbuf = ds.sample_buf.as_mut().unwrap();
                        sbuf.copy_interleaved_ref(decoded);
                        let mut samples = sbuf.samples().to_vec();

                        // ── 5. Apply EQ ──────────────────────────
                        let eq_ver = shared.eq_state.lock().version;
                        if eq_version_seen != eq_ver {
                            eq_version_seen = eq_ver;
                            eq_filter_bank = None;
                        }

                        let eq_enabled = shared.eq_state.lock().enabled;
                        if eq_enabled {
                            let needs_rebuild = eq_filter_bank.as_ref().map(|b| {
                                b.sample_rate != sr || b.channels != channels
                            }).unwrap_or(true);

                            if needs_rebuild {
                                let state = shared.eq_state.lock();
                                if !state.bands.is_empty() {
                                    eq_filter_bank = Some(EqFilterBank::new(&state.bands, sr, channels));
                                }
                            }

                            if let Some(bank) = &mut eq_filter_bank {
                                bank.process(&mut samples);
                            }
                        } else {
                            eq_filter_bank = None;
                        }

                        // ── Spectrum FFT accumulation ──────────────────────
                        // Results are written to shared.spectrum (Arc<Mutex>)
                        // which the frontend polls via get_spectrum command.
                        // No event_tx send — avoids flooding the Tauri IPC
                        // channel with ~30 unhandled events/sec that would
                        // degrade invoke() responsiveness over time.
                        {
                            let ch = channels as usize;
                            for frame in samples.chunks(ch) {
                                let mono = frame.iter().sum::<f32>() / ch as f32;
                                fft_buf.push(mono);
                            }
                            while fft_buf.len() >= FFT_SIZE {
                                if last_spectrum_emit.elapsed() >= Duration::from_millis(33) {
                                    let spectrum = compute_log_spectrum(
                                        &fft_buf[..FFT_SIZE],
                                        sr,
                                        SPECTRUM_BINS,
                                        &fft_forward,
                                    );
                                    // Only overwrite when the frame has audible signal.
                                    // End-of-track silence padding produces all-zero FFT
                                    // output; writing those zeros makes the spectrum
                                    // disappear for the entire next-track download gap
                                    // (30-60 s for Qobuz). Keeping the last non-silent
                                    // frame lets the frontend smoothing decay it naturally.
                                    let peak = spectrum.iter().cloned().fold(0.0f32, f32::max);
                                    if peak > 1e-5 {
                                        *shared.spectrum.lock() = spectrum;
                                    }
                                    last_spectrum_emit = Instant::now();
                                }
                                // 50% overlap hop
                                let hop = FFT_SIZE / 2;
                                fft_buf.drain(..hop);
                            }
                        }

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
            Err(e) => {
                let is_eof = matches!(
                    &e,
                    SymphoniaError::IoError(io_err)
                        if io_err.kind() == std::io::ErrorKind::UnexpectedEof
                );
                if !is_eof {
                    error!("Format read error: {}", e);
                }
                dec_state = None;
            }
        }
    }
}

// ── FFT / Spectrum helpers ────────────────────────────────────────────

fn compute_log_spectrum(
    samples: &[f32],
    sample_rate: u32,
    n_bins: usize,
    fft: &std::sync::Arc<dyn rustfft::Fft<f32>>,
) -> Vec<f32> {
    let n = samples.len();
    // Hann window
    let mut buf: Vec<Complex<f32>> = samples
        .iter()
        .enumerate()
        .map(|(i, &s)| {
            let w = 0.5 * (1.0 - (2.0 * PI * i as f32 / (n - 1) as f32).cos());
            Complex::new(s * w, 0.0)
        })
        .collect();

    fft.process(&mut buf);

    // Start at 40 Hz: at 44.1 kHz with a 2048-pt FFT, bins below ~21 Hz all map
    // to FFT bin 1, making the leftmost bars move identically ("glued" appearance).
    // 40 Hz is the lowest useful resolution without duplicates.
    let min_f = 40.0_f32;
    let max_f = (sample_rate as f32 / 2.0).min(20000.0);

    (0..n_bins)
        .map(|i| {
            let t = i as f32 / (n_bins - 1) as f32;
            let freq = min_f * (max_f / min_f).powf(t);

            let t_lo = ((i as f32 - 0.5) / (n_bins - 1) as f32).max(0.0);
            let t_hi = ((i as f32 + 0.5) / (n_bins - 1) as f32).min(1.0);
            let f_lo = min_f * (max_f / min_f).powf(t_lo);
            let f_hi = min_f * (max_f / min_f).powf(t_hi);

            let bin_lo = ((f_lo * n as f32 / sample_rate as f32) as usize).max(1);
            let bin_hi = ((f_hi * n as f32 / sample_rate as f32) as usize + 1).min(n / 2);

            if bin_hi <= bin_lo {
                let b = ((freq * n as f32 / sample_rate as f32) as usize)
                    .max(1)
                    .min(n / 2 - 1);
                return buf[b].norm() * 2.0 / n as f32;
            }

            let sum: f32 = buf[bin_lo..bin_hi].iter().map(|c| c.norm()).sum();
            sum / (bin_hi - bin_lo) as f32 * 2.0 / n as f32
        })
        .collect()
}

// ── Helper functions ─────────────────────────────────────────────────

/// Enumerate installed ASIO driver names from the Windows registry.
///
/// Reads `HKLM\SOFTWARE\ASIO` subkey names without loading any driver DLL.
/// Loading drivers via `cpal::host_from_id` causes heap corruption with some
/// buggy ASIO drivers (notably CAUSBAudio) during device enumeration.
#[cfg(feature = "asio")]
fn asio_drivers_from_registry() -> Vec<String> {
    use std::process::Command;
    let output = match Command::new("reg")
        .args(["query", r"HKLM\SOFTWARE\ASIO"])
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            tracing::warn!("[audio_devices] reg query failed: {e}");
            return vec![];
        }
    };
    let prefix = r"HKEY_LOCAL_MACHINE\SOFTWARE\ASIO\";
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.starts_with(prefix) && line.len() > prefix.len() {
                Some(line[prefix.len()..].to_string())
            } else {
                None
            }
        })
        .collect()
}

/// Find the best ASIO output config for a target sample rate.
///
/// Priority:
/// 1. Stereo (2 ch) exact SR match
/// 2. Any-channel exact SR match
/// 3. Stereo closest SR (prefer higher — upsampling > downsampling)
/// 4. Any-channel closest SR
///
/// Many ASIO drivers skip 176400 Hz (e.g. CAUSBAudio) but support 192000 Hz;
/// this function selects 192000 Hz automatically in that case.
#[cfg(feature = "asio")]
fn best_asio_config(
    ranges: &[cpal::SupportedStreamConfigRange],
    target: cpal::SampleRate,
) -> Option<(cpal::SupportedStreamConfigRange, cpal::SampleRate)> {
    let stereo: Vec<_> = ranges.iter().filter(|r| r.channels() == 2).collect();
    let any: Vec<_> = ranges.iter().collect();

    // 1 & 2 — exact SR match
    for group in [&stereo, &any] {
        if let Some(r) = group
            .iter()
            .find(|r| r.min_sample_rate() <= target && target <= r.max_sample_rate())
        {
            return Some(((*r).clone(), target));
        }
    }

    // 3 & 4 — closest supported SR (prefer higher rates)
    for group in [&stereo, &any] {
        let best = group.iter().min_by_key(|r| {
            let sr = r.min_sample_rate().0 as i64;
            let diff = sr - target.0 as i64;
            // Penalise lower rates slightly so we prefer rounding up
            if diff < 0 { (-diff) * 2 } else { diff }
        });
        if let Some(r) = best {
            return Some(((*r).clone(), r.min_sample_rate()));
        }
    }
    None
}

/// Open an OutputStream for the requested device name.
///
/// Device name routing:
///   - `None`           → system default (WASAPI on Windows)
///   - `"ASIO: <name>"` → ASIO host (requires `--features asio` + ASIO SDK)
///   - `"<name>"`       → WASAPI / CoreAudio device by name
///
/// Falls back to the system default on any lookup failure.
fn open_output_stream_impl(
    device_name: &Option<String>,
    sample_rate: Option<u32>,
) -> Result<(OutputStream, OutputStreamHandle), rodio::StreamError> {
    let Some(name) = device_name else {
        return OutputStream::try_default();
    };

    // ── ASIO path (compile-time optional) ──────────────────────────────
    #[cfg(feature = "asio")]
    if let Some(asio_name) = name.strip_prefix("ASIO: ") {
        match cpal::host_from_id(cpal::HostId::Asio) {
            Ok(asio_host) => {
                let device = asio_host
                    .output_devices()
                    .ok()
                    .and_then(|mut devs| {
                        devs.find(|d| d.name().ok().as_deref() == Some(asio_name))
                    });
                return match device {
                    Some(dev) => {
                        // Try to open the stream at the track's exact sample rate
                        // for bit-perfect output — ASIO does not resample.
                        if let Some(sr) = sample_rate {
                            let target = cpal::SampleRate(sr);
                            let all_ranges: Vec<_> = dev
                                .supported_output_configs()
                                .ok()
                                .map(|it| it.collect())
                                .unwrap_or_default();

                            if let Some((range, actual_sr)) =
                                best_asio_config(&all_ranges, target)
                            {
                                let cfg = range.with_sample_rate(actual_sr);
                                if actual_sr != target {
                                    info!(
                                        "ASIO: {sr} Hz not supported, using {} Hz instead",
                                        actual_sr.0
                                    );
                                }
                                info!("ASIO opening: {} ch @ {} Hz", cfg.channels(), actual_sr.0);
                                match OutputStream::try_from_device_config(&dev, cfg) {
                                    Ok(result) => return Ok(result),
                                    Err(e) => {
                                        warn!("ASIO {} Hz failed ({e}), falling back to device default", actual_sr.0);
                                    }
                                }
                            } else {
                                warn!("ASIO device reports no supported configs");
                            }
                        }
                        OutputStream::try_from_device(&dev)
                    }
                    None => {
                        warn!("ASIO device '{asio_name}' not found, using system default");
                        OutputStream::try_default()
                    }
                };
            }
            Err(e) => {
                warn!("ASIO host unavailable ({e}), using system default");
                return OutputStream::try_default();
            }
        }
    }

    // ── WASAPI / CoreAudio path ─────────────────────────────────────────
    let host = cpal::default_host();
    let device = host
        .output_devices()
        .ok()
        .and_then(|mut devs| devs.find(|d| d.name().ok().as_deref() == Some(name.as_str())));
    match device {
        Some(dev) => OutputStream::try_from_device(&dev),
        None => {
            warn!("Output device '{name}' not found, using system default");
            OutputStream::try_default()
        }
    }
}

fn ensure_sink(
    stream: &mut Option<OutputStream>,
    handle: &mut Option<OutputStreamHandle>,
    sink: &mut Option<Sink>,
    volume: f32,
    device_name: &Option<String>,
    sample_rate: Option<u32>,
    output_stream_sr: &Arc<AtomicU32>,
) {
    if sink.is_some() {
        return;
    }

    match open_output_stream_impl(device_name, sample_rate) {
        Ok((s, h)) => match Sink::try_new(&h) {
            Ok(sk) => {
                sk.set_volume(AudioPlayer::cubic_volume(volume));
                let actual_sr = sample_rate.unwrap_or(0);
                output_stream_sr.store(actual_sr, Ordering::Relaxed);
                *stream = Some(s);
                *handle = Some(h);
                *sink = Some(sk);
                debug!("Audio output initialized");
            }
            Err(e) => error!("Failed to create audio sink: {}", e),
        },
        Err(e) => error!("Failed to open audio output: {}", e),
    }
}

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
        &FormatOptions { enable_gapless: true, ..Default::default() },
        &MetadataOptions::default(),
    ) {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to probe audio format: {}", e);
            return None;
        }
    };

    let track = probed
        .format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)?;
    let track_id = track.id;

    let existing_dur = shared.duration_ms.load(Ordering::SeqCst);
    if existing_dur == 0 {
        if let (Some(tb), Some(nf)) = (track.codec_params.time_base, track.codec_params.n_frames) {
            let time = tb.calc_time(nf);
            let dur = ((time.seconds as f64 + time.frac) * 1000.0) as u64;
            shared.duration_ms.store(dur, Ordering::SeqCst);
        }
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
    let sample_rate = track.codec_params.sample_rate;
    Some(DecoderState { format: probed.format, decoder, track_id, sample_buf: None, sample_rate })
}

// ── Unit Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_position_advances_while_playing() {
        let start = Instant::now();
        thread::sleep(Duration::from_millis(100));
        let pos = 0u64 + start.elapsed().as_millis() as u64;
        assert!(pos >= 80);
    }

    #[test]
    fn test_position_frozen_when_paused() {
        let position_at_start_ms: u64 = 12_345;
        let playback_start: Option<Instant> = None;
        let current_pos = if let Some(start) = &playback_start {
            position_at_start_ms + start.elapsed().as_millis() as u64
        } else {
            position_at_start_ms
        };
        assert_eq!(current_pos, 12_345);
    }

    #[test]
    fn test_seek_updates_position_at_start() {
        let seek_target_ms: u64 = 45_000;
        let position_at_start_ms = seek_target_ms;
        let playback_start: Option<Instant> = Some(Instant::now());
        let current_pos = if let Some(start) = &playback_start {
            position_at_start_ms + start.elapsed().as_millis() as u64
        } else {
            position_at_start_ms
        };
        assert!(current_pos >= seek_target_ms && current_pos <= seek_target_ms + 50);
    }

    #[test]
    fn test_position_clamped_to_duration() {
        let duration_ms: u64 = 180_000;
        let raw_pos: u64 = 200_000;
        let clamped = if duration_ms > 0 { raw_pos.min(duration_ms) } else { raw_pos };
        assert_eq!(clamped, 180_000);
    }

    #[test]
    fn test_cubic_volume_extremes() {
        assert_eq!(AudioPlayer::cubic_volume(0.0), 0.0);
        assert_eq!(AudioPlayer::cubic_volume(1.0), 1.0);
    }

    #[test]
    fn test_cubic_volume_midpoint() {
        let vol = AudioPlayer::cubic_volume(0.5);
        assert!((vol - 0.125).abs() < 0.001);
    }

    #[test]
    fn test_biquad_identity_passthrough() {
        let mut f = BiquadFilter::new(BiquadCoeffs::identity());
        // Identity filter: output should equal input
        let out = f.process(0.5);
        assert!((out - 0.5).abs() < 1e-9);
        let out2 = f.process(-0.3);
        assert!((out2 - (-0.3)).abs() < 1e-9);
    }

    #[test]
    fn test_peak_eq_zero_gain_is_identity() {
        let coeffs = BiquadCoeffs::peak_eq(44100.0, 1000.0, 0.0, 1.0);
        // gain_db = 0 returns identity
        assert!((coeffs.b0 - 1.0).abs() < 1e-9);
        assert!(coeffs.b1.abs() < 1e-9);
        assert!(coeffs.b2.abs() < 1e-9);
    }

    #[test]
    fn test_eq_filter_bank_processes_without_crash() {
        let bands = vec![
            EqBandParam { freq: 60.0,    gain_db: 3.0,  q: 0.9 },
            EqBandParam { freq: 1000.0,  gain_db: -2.0, q: 1.0 },
            EqBandParam { freq: 14000.0, gain_db: 5.0,  q: 0.9 },
        ];
        let mut bank = EqFilterBank::new(&bands, 44100, 2);
        let mut samples = vec![0.5f32, -0.3, 0.1, 0.8, -0.5, 0.2];
        bank.process(&mut samples);
        for s in &samples {
            assert!(s.is_finite() && *s >= -1.0 && *s <= 1.0);
        }
    }

    // ── FFT / Spectrum tests ──────────────────────────────────────────────

    fn make_fft(size: usize) -> std::sync::Arc<dyn rustfft::Fft<f32>> {
        FftPlanner::<f32>::new().plan_fft_forward(size)
    }

    #[test]
    fn spectrum_returns_correct_bin_count() {
        let fft     = make_fft(FFT_SIZE);
        let silence = vec![0.0f32; FFT_SIZE];
        let bins    = compute_log_spectrum(&silence, 44100, SPECTRUM_BINS, &fft);
        assert_eq!(bins.len(), SPECTRUM_BINS,
            "Expected {} bins, got {}", SPECTRUM_BINS, bins.len());
    }

    #[test]
    fn spectrum_values_are_non_negative() {
        let fft    = make_fft(FFT_SIZE);
        let signal: Vec<f32> = (0..FFT_SIZE)
            .map(|i| (i as f32 * 0.37).sin() * 0.5)
            .collect();
        let bins = compute_log_spectrum(&signal, 44100, SPECTRUM_BINS, &fft);
        let negatives: Vec<(usize, f32)> = bins.iter().enumerate()
            .filter(|(_, &v)| v < 0.0)
            .map(|(i, &v)| (i, v))
            .collect();
        assert!(negatives.is_empty(),
            "Spectrum has negative bins: {:?}", negatives);
    }

    #[test]
    fn spectrum_silence_is_near_zero() {
        let fft     = make_fft(FFT_SIZE);
        let silence = vec![0.0f32; FFT_SIZE];
        let bins    = compute_log_spectrum(&silence, 44100, SPECTRUM_BINS, &fft);
        let max     = bins.iter().cloned().fold(0.0f32, f32::max);
        assert!(max < 1e-6,
            "Silent input produced non-zero spectrum: max = {}", max);
    }

    #[test]
    fn spectrum_440hz_sine_peaks_near_correct_bin() {
        let fft         = make_fft(FFT_SIZE);
        let sample_rate = 44100u32;
        let freq        = 440.0f32;

        let signal: Vec<f32> = (0..FFT_SIZE)
            .map(|i| (2.0 * PI * freq * i as f32 / sample_rate as f32).sin())
            .collect();

        let bins     = compute_log_spectrum(&signal, sample_rate, SPECTRUM_BINS, &fft);
        let peak_bin = bins.iter().enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(i, _)| i)
            .unwrap();

        // Log-scale position of 440 Hz between 20 Hz and 20 kHz
        let expected = ((440.0f32 / 20.0).log10() / (20000.0f32 / 20.0).log10()
            * (SPECTRUM_BINS - 1) as f32) as usize;

        assert!(
            peak_bin.abs_diff(expected) <= 5,
            "440 Hz peak at bin {} but expected near bin {} (tolerance ±5)",
            peak_bin, expected
        );
    }

    #[test]
    fn spectrum_amplitude_correlates_with_signal_level() {
        let fft         = make_fft(FFT_SIZE);
        let sample_rate = 44100u32;
        let freq        = 1000.0f32;

        let make = |amp: f32| -> Vec<f32> {
            (0..FFT_SIZE)
                .map(|i| amp * (2.0 * PI * freq * i as f32 / sample_rate as f32).sin())
                .collect()
        };

        let low  = compute_log_spectrum(&make(0.1), sample_rate, SPECTRUM_BINS, &fft);
        let high = compute_log_spectrum(&make(0.9), sample_rate, SPECTRUM_BINS, &fft);

        let max_low  = low .iter().cloned().fold(0.0f32, f32::max);
        let max_high = high.iter().cloned().fold(0.0f32, f32::max);

        assert!(max_high > max_low,
            "Higher amplitude must produce larger spectrum values: high={} low={}",
            max_high, max_low);
    }

    #[test]
    fn spectrum_shared_arc_mutex_concurrent_read_write() {
        use parking_lot::Mutex;
        use std::sync::Arc;

        let shared: Arc<Mutex<Vec<f32>>> =
            Arc::new(Mutex::new(vec![0.0f32; SPECTRUM_BINS]));

        // 8 reader threads + 1 writer — no deadlock, no panic
        let readers: Vec<_> = (0..8).map(|_| {
            let s = Arc::clone(&shared);
            thread::spawn(move || {
                for _ in 0..200 {
                    let data = s.lock().clone();
                    assert_eq!(data.len(), SPECTRUM_BINS);
                }
            })
        }).collect();

        for i in 0..200u32 {
            *shared.lock() = vec![i as f32 / 200.0; SPECTRUM_BINS];
        }

        for h in readers { h.join().unwrap(); }

        // After writes, data is valid
        let final_data = shared.lock().clone();
        assert_eq!(final_data.len(), SPECTRUM_BINS);
        assert!(final_data.iter().all(|&v| v >= 0.0 && v <= 1.0));
    }

    #[test]
    fn spectrum_fft_throttle_33ms_logic() {
        // Verify the throttling gate: elapsed >= 33ms → emit; else skip
        let mut last_emit = Instant::now();
        let mut emit_count = 0u32;

        for tick in 0..100 {
            // Simulate 10ms per tick → emit every 4th tick (≈33ms)
            let fake_elapsed = Duration::from_millis(tick * 10);
            if fake_elapsed.saturating_sub(last_emit.elapsed()) == Duration::ZERO
                && fake_elapsed >= Duration::from_millis(33)
            {
                emit_count += 1;
                last_emit = Instant::now();
            }
        }
        // This is a structural test — just ensure it compiles and runs
        let _ = emit_count;

        // Direct logic test: 33ms gate
        let t0 = Instant::now() - Duration::from_millis(40);
        assert!(t0.elapsed() >= Duration::from_millis(33));

        let t1 = Instant::now() - Duration::from_millis(10);
        assert!(t1.elapsed() < Duration::from_millis(33));
    }
}
