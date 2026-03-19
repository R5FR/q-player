import React, { useEffect, useRef, useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  X,
  Play,
  Pause,
  SkipBack,
  SkipForward,
  Shuffle,
  Repeat,
  Volume,
  Volume1,
  Volume2,
  VolumeX,
  ListMusic,
} from "lucide-react";
import { shallow } from "zustand/shallow";
import { useStore } from "../store";
import * as api from "../api";
import type { QueueState } from "../types";

interface Props {
  onClose?: () => void;
}

// ─────────────────────────────────────────────────────────────────────────────
// SpectrumViz — copie exacte de SpectrumCanvas (EqView) qui fonctionne.
// Seule différence : couleur lime pure au lieu du dégradé lime→orange.
// ─────────────────────────────────────────────────────────────────────────────

const DB_FLOOR = -70;
const DB_REF   =  0;
function linToDB(v: number) { return 20 * Math.log10(Math.max(v, 1e-9)); }
function dbToNorm(dB: number) { return Math.max(0, Math.min(1, (dB - DB_FLOOR) / (DB_REF - DB_FLOOR))); }

// Shared diagnostic stats — written by SpectrumViz, read by SpectrumDiag
const diagStats = {
  polls: 0, successes: 0, errors: 0, rafFrames: 0, lastSuccessMs: 0, maxRaw: 0,
};

const N_BINS = 80;

function SpectrumViz() {
  const containerRef = useRef<HTMLDivElement>(null);
  const smoothRef    = useRef(new Float32Array(N_BINS).fill(0));
  const rawRef       = useRef(new Float32Array(N_BINS).fill(0));
  const rafRef       = useRef(0);
  const lastDataRef  = useRef(new Float32Array(N_BINS).fill(0));
  const staleCount   = useRef(0);

  // ── Poll spectrum data ──────────────────────────────────────────────
  useEffect(() => {
    let alive = true;
    const timer = setInterval(() => {
      if (!alive) return;
      diagStats.polls++;
      api.getSpectrum()
        .then(data => {
          if (!alive || !data?.length) return;
          let different = false;
          for (let i = 0; i < Math.min(data.length, N_BINS); i++) {
            if (data[i] !== lastDataRef.current[i]) { different = true; break; }
          }
          if (different) {
            staleCount.current = 0;
            for (let i = 0; i < Math.min(data.length, rawRef.current.length); i++) {
              rawRef.current[i]      = data[i];
              lastDataRef.current[i] = data[i];
            }
            diagStats.successes++;
            diagStats.lastSuccessMs = Date.now();
            diagStats.maxRaw = Math.max(...data);
          } else {
            staleCount.current++;
            if (staleCount.current > 10) rawRef.current.fill(0);
          }
        })
        .catch(() => { diagStats.errors++; });
    }, 50);
    return () => { alive = false; clearInterval(timer); };
  }, []);

  // ── Draw loop — updates bar heights via direct DOM style writes ─────
  // Uses div elements instead of <canvas> to avoid WebView2/Chromium
  // compositing-layer caching that freezes canvas content inside
  // framer-motion animated containers.
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;
    const bars = container.children;

    const draw = () => {
      rafRef.current = requestAnimationFrame(draw);
      diagStats.rafFrames++;
      for (let i = 0; i < N_BINS; i++) {
        const target = Math.pow(dbToNorm(linToDB(rawRef.current[i])), 0.75);
        const delta  = target - smoothRef.current[i];
        smoothRef.current[i] += delta * (delta > 0 ? 0.55 : 0.07);
        const bar = bars[i] as HTMLDivElement | undefined;
        if (bar) {
          bar.style.height = `${Math.max(0.3, smoothRef.current[i] * 100)}%`;
        }
      }
    };
    draw();
    return () => cancelAnimationFrame(rafRef.current);
  }, []);

  return (
    <div
      ref={containerRef}
      style={{
        display: "flex", alignItems: "flex-end",
        width: "100%", height: "100%", gap: "1px",
      }}
    >
      {Array.from({ length: N_BINS }, (_, i) => (
        <div
          key={i}
          style={{
            flex: 1,
            height: "0.3%",
            borderRadius: "2px 2px 0 0",
            background:
              "linear-gradient(to bottom, rgba(183,255,46,0.27), rgba(183,255,46,0.025))",
          }}
        />
      ))}
    </div>
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// Diagnostic overlay — press Ctrl+Shift+D in fullscreen to toggle
// ─────────────────────────────────────────────────────────────────────────────

function SpectrumDiag() {
  const [snap, setSnap] = useState({ ...diagStats, age: 0 });

  useEffect(() => {
    const t = setInterval(() => {
      const age = diagStats.lastSuccessMs > 0
        ? Math.round((Date.now() - diagStats.lastSuccessMs) / 100) / 10
        : -1;
      setSnap({ ...diagStats, age });
    }, 500);
    return () => clearInterval(t);
  }, []);

  const successRate = snap.polls > 0
    ? Math.round((snap.successes / snap.polls) * 100)
    : 0;

  const isStale = snap.age > 2;

  return (
    <div style={{
      position: "fixed", top: 12, left: "50%", transform: "translateX(-50%)",
      zIndex: 99999, background: "rgba(0,0,0,0.85)", backdropFilter: "blur(8px)",
      border: "1px solid rgba(183,255,46,0.3)", borderRadius: 8, padding: "8px 14px",
      fontFamily: "monospace", fontSize: 11, color: "#b7ff2e", lineHeight: 1.7,
      pointerEvents: "none", minWidth: 260,
    }}>
      <div style={{ color: "rgba(183,255,46,0.5)", marginBottom: 2 }}>◆ SPECTRUM DIAG (Ctrl+Shift+D)</div>
      <div>polls:    <b>{snap.polls}</b> ({snap.polls > 0 ? Math.round(snap.polls / 50) : 0}s uptime)</div>
      <div>success:  <b style={{ color: successRate === 100 ? "#b7ff2e" : "#ff6432" }}>{snap.successes}</b> ({successRate}%)</div>
      <div>errors:   <b style={{ color: snap.errors > 0 ? "#ff6432" : "#b7ff2e" }}>{snap.errors}</b></div>
      <div>RAF:      <b>{snap.rafFrames}</b> frames</div>
      <div>last ok:  <b style={{ color: isStale ? "#ff6432" : "#b7ff2e" }}>
        {snap.age < 0 ? "never" : `${snap.age}s ago`}
      </b> {isStale ? "⚠ STALE" : "✓"}</div>
      <div>max raw:  <b>{snap.maxRaw.toFixed(4)}</b> {snap.maxRaw < 0.0001 ? "⚠ ZERO" : "✓"}</div>
    </div>
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// Main component
// ─────────────────────────────────────────────────────────────────────────────

export default function FullscreenPlayer({ onClose }: Props) {
  const {
    playback, setPlayback,
    shuffle, setShuffle,
    repeatMode, cycleRepeat,
    setIsSeeking,
    queue, setQueue, setIsFullscreen,
  } = useStore(s => ({
    playback:        s.playback,
    setPlayback:     s.setPlayback,
    shuffle:         s.shuffle,
    setShuffle:      s.setShuffle,
    repeatMode:      s.repeatMode,
    cycleRepeat:     s.cycleRepeat,
    setIsSeeking:    s.setIsSeeking,
    queue:           s.queue,
    setQueue:        s.setQueue,
    setIsFullscreen: s.setIsFullscreen,
  }), shallow);

  const [seekOverride, setSeekOverride]   = useState<number | null>(null);
  const [localVolume,  setLocalVolume]    = useState(playback.volume);
  const [prevVolume,   setPrevVolume]     = useState(0.7);
  const [showQueue,    setShowQueue]      = useState(false);
  const [showDiag,     setShowDiag]      = useState(false);
  const [queueData,    setQueueData]      = useState<QueueState>(queue);

  const seekVal  = useRef<number>(0);
  const seekTmr  = useRef<ReturnType<typeof setTimeout> | null>(null);

  const track      = playback.current_track;
  const currentPos = seekOverride ?? playback.position_ms;
  const progress   = playback.duration_ms > 0 ? (currentPos / playback.duration_ms) * 100 : 0;

  // ── Effects ──────────────────────────────────────────────────────────────

  useEffect(() => {
    api.getQueue().then(q => { setQueueData(q); setQueue(q); }).catch(() => {});
  }, []);

  useEffect(() => { setLocalVolume(playback.volume); }, [playback.volume]);

  useEffect(() => {
    const fn = (e: KeyboardEvent) => {
      if (e.key === "Escape") setIsFullscreen(false);
      if (e.key === "D" && e.ctrlKey && e.shiftKey) setShowDiag(v => !v);
    };
    document.addEventListener("keydown", fn);
    return () => document.removeEventListener("keydown", fn);
  }, []);

  // ── Playback handlers ────────────────────────────────────────────────────

  const togglePlay = async () => {
    try {
      if (playback.is_playing) await api.pause(); else await api.resume();
      setPlayback(await api.getPlaybackState());
    } catch {}
  };
  const handleNext = async () => { try { const s = await api.nextTrack();    if (s) setPlayback(s); } catch {} };
  const handlePrev = async () => { try { const s = await api.previousTrack(); if (s) setPlayback(s); } catch {} };

  const handleSeekStart  = () => setIsSeeking(true);
  const handleSeekChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const v = +e.target.value; seekVal.current = v; setSeekOverride(v);
  };
  const handleSeekCommit = async () => {
    const p = seekVal.current; setSeekOverride(p);
    try { await api.seek(p); setPlayback(await api.getPlaybackState()); } catch {}
    if (seekTmr.current) clearTimeout(seekTmr.current);
    seekTmr.current = setTimeout(() => { setSeekOverride(null); setIsSeeking(false); }, 600);
  };

  const handleShuffle = async () => { const n = !shuffle; setShuffle(n); if (n) try { await api.smartShuffle(); } catch {} };
  const handleVolume  = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const v = +e.target.value; setLocalVolume(v); if (v > 0) setPrevVolume(v); await api.setVolume(v);
  };
  const handleMute = async () => {
    if (localVolume > 0) { setPrevVolume(localVolume); setLocalVolume(0); await api.setVolume(0); }
    else { const r = prevVolume > 0 ? prevVolume : 0.7; setLocalVolume(r); await api.setVolume(r); }
  };

  const fmt = (ms: number) => { const s = Math.floor(ms / 1000); return `${Math.floor(s / 60)}:${(s % 60).toString().padStart(2, "0")}`; };

  const VolumeIcon = localVolume === 0 ? VolumeX : localVolume < 0.33 ? Volume : localVolume < 0.66 ? Volume1 : Volume2;
  const isHires    = (playback.bit_depth && playback.bit_depth > 16) || (playback.sample_rate && playback.sample_rate > 44.1);
  const upcoming   = queueData.tracks.slice((queueData.current_index ?? -1) + 1);

  // ── Render ───────────────────────────────────────────────────────────────

  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      transition={{ duration: 0.25 }}
      className="fixed inset-0 z-[9999] overflow-hidden"
      style={{ background: "#03030a" }}
    >
      {/* ──────── DIAGNOSTIC OVERLAY (Ctrl+Shift+D) ──────── */}
      {showDiag && <SpectrumDiag />}

      {/* ──────── LAYER 1 : immersive blurred backdrop ──────── */}
      <AnimatePresence>
        {track?.cover_url && (
          <motion.div
            key={track.cover_url}
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 1.0 }}
            className="absolute inset-0 pointer-events-none"
            style={{
              backgroundImage:    `url(${track.cover_url})`,
              backgroundSize:     "cover",
              backgroundPosition: "center",
              filter:             "blur(72px) saturate(3.2) brightness(0.42)",
              transform:          "scale(1.25)",
            }}
          />
        )}
      </AnimatePresence>

      {/* ──────── LAYER 2 : vignette + darkening gradients ──────── */}
      {/* radial vignette — darkens edges */}
      <div className="absolute inset-0 pointer-events-none" style={{
        background: "radial-gradient(ellipse 80% 75% at 50% 42%, transparent 10%, rgba(3,3,10,0.6) 100%)",
      }} />
      {/* bottom gradient — ensures controls stay readable over spectrum */}
      <div className="absolute bottom-0 left-0 right-0 pointer-events-none" style={{
        height: "52%",
        background: "linear-gradient(to bottom, transparent 0%, rgba(3,3,10,0.72) 40%, rgba(3,3,10,0.96) 100%)",
      }} />
      {/* top gradient — top-bar legibility */}
      <div className="absolute top-0 left-0 right-0 pointer-events-none" style={{
        height: "22%",
        background: "linear-gradient(to top, transparent, rgba(3,3,10,0.65))",
      }} />

      {/* ──────── LAYER 3 : spectrum — fills bottom third ──────── */}
      <div className="absolute bottom-0 left-0 right-0 pointer-events-none" style={{ height: "38%", opacity: 0.32, filter: "blur(0.5px)" }}>
        <SpectrumViz />
      </div>

      {/* ──────── LAYER 4 : content ──────── */}
      <div className="relative z-10 h-full w-full flex flex-col">

        {/* ── Top bar ── */}
        <div className="flex-shrink-0 flex items-center justify-between px-6 pt-5 h-14">
          {/* Close */}
          <motion.button
            whileTap={{ scale: 0.9 }}
            onClick={() => setIsFullscreen(false)}
            title="Fermer (Échap)"
            className="w-9 h-9 rounded-full flex items-center justify-center transition-all duration-150"
            style={{ color: "rgba(255,255,255,0.45)", background: "rgba(0,0,0,0.35)", backdropFilter: "blur(12px)", border: "1px solid rgba(255,255,255,0.1)" }}
            onMouseEnter={e => { e.currentTarget.style.color = "#fff"; e.currentTarget.style.background = "rgba(0,0,0,0.55)"; }}
            onMouseLeave={e => { e.currentTarget.style.color = "rgba(255,255,255,0.45)"; e.currentTarget.style.background = "rgba(0,0,0,0.35)"; }}
          >
            <X style={{ width: 16, height: 16 }} />
          </motion.button>

          {/* Label */}
          <p className="font-condensed text-[9px] font-semibold uppercase tracking-[0.3em]" style={{ color: "rgba(255,255,255,0.28)" }}>
            En lecture
          </p>

          {/* Queue toggle */}
          <motion.button
            whileTap={{ scale: 0.9 }}
            onClick={() => setShowQueue(v => !v)}
            title="File d'attente"
            className="w-9 h-9 rounded-full flex items-center justify-center transition-all duration-150"
            style={{
              color:      showQueue ? "rgb(var(--qs-accent))" : "rgba(255,255,255,0.45)",
              background: showQueue ? "rgb(var(--qs-accent) / 0.12)" : "rgba(0,0,0,0.35)",
              backdropFilter: "blur(12px)",
              border: `1px solid ${showQueue ? "rgb(var(--qs-accent) / 0.25)" : "rgba(255,255,255,0.1)"}`,
            }}
          >
            <ListMusic style={{ width: 16, height: 16 }} />
          </motion.button>
        </div>

        {/* ── Centre : album art + track info ── */}
        <div className="flex-1 flex flex-col items-center justify-center gap-5 px-8 min-h-0">

          {/* Album artwork */}
          <motion.div
            layoutId="player-cover"
            animate={playback.is_playing ? { y: [0, -7, 0] } : { y: 0 }}
            transition={playback.is_playing
              ? { y: { duration: 4.5, ease: "easeInOut", repeat: Infinity } }
              : { y: { duration: 0.6, ease: "easeOut" } }
            }
            className="rounded-2xl overflow-hidden flex-shrink-0"
            style={{
              width:  "min(320px, 34vh)",
              height: "min(320px, 34vh)",
              boxShadow: playback.is_playing
                ? "0 0 0 1px rgba(255,255,255,0.07), 0 0 80px rgb(var(--qs-accent) / 0.18), 0 45px 90px rgba(0,0,0,0.95)"
                : "0 0 0 1px rgba(255,255,255,0.05), 0 45px 90px rgba(0,0,0,0.9)",
              transition: "box-shadow 0.9s ease",
            }}
          >
            {track?.cover_url ? (
              <img src={track.cover_url} alt={track?.album} className="w-full h-full object-cover" />
            ) : (
              <div className="w-full h-full flex items-center justify-center" style={{ background: "rgb(14,14,22)" }}>
                <span className="opacity-20 text-6xl">🎵</span>
              </div>
            )}
          </motion.div>

          {/* Track info — cross-fades on track change */}
          <AnimatePresence mode="wait">
            <motion.div
              key={track?.id ?? "empty"}
              initial={{ opacity: 0, y: 12 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -8 }}
              transition={{ duration: 0.35 }}
              className="text-center w-full max-w-md px-4"
            >
              <h1
                className="font-display leading-none text-white truncate"
                style={{
                  fontSize: "clamp(34px, 5vw, 64px)",
                  letterSpacing: "0.05em",
                  textShadow: "0 4px 32px rgba(0,0,0,0.85)",
                }}
              >
                {track?.title ?? "—"}
              </h1>
              <p
                className="font-condensed text-lg uppercase tracking-[0.12em] mt-1.5 truncate"
                style={{ color: "rgba(255,255,255,0.5)", textShadow: "0 2px 16px rgba(0,0,0,0.7)" }}
              >
                {track?.artist ?? ""}
              </p>

              {/* Quality tags */}
              {(playback.quality || playback.sample_rate || playback.bit_depth) && (
                <div className="mt-3 flex items-center justify-center gap-1.5 flex-wrap">
                  {playback.quality && (
                    <span className={isHires ? "quality-hires" : "quality-lossless"}>{playback.quality}</span>
                  )}
                  {playback.sample_rate && (
                    <span className="font-mono text-[9px] px-1.5 py-0.5 rounded"
                      style={{ color: "rgba(255,255,255,0.3)", background: "rgba(255,255,255,0.07)", border: "1px solid rgba(255,255,255,0.08)" }}>
                      {playback.sample_rate}kHz
                    </span>
                  )}
                  {playback.bit_depth && (
                    <span className="font-mono text-[9px] px-1.5 py-0.5 rounded"
                      style={{ color: "rgba(255,255,255,0.3)", background: "rgba(255,255,255,0.07)", border: "1px solid rgba(255,255,255,0.08)" }}>
                      {playback.bit_depth}bit
                    </span>
                  )}
                </div>
              )}
            </motion.div>
          </AnimatePresence>
        </div>

        {/* ── Bottom : seek + controls + volume ── */}
        <div className="flex-shrink-0 w-full max-w-xl mx-auto px-8 pb-12">

          {/* Seek bar */}
          <div className="mb-6">
            <div className="relative group mb-2" style={{ height: 24 }}>
              {/* Track */}
              <div className="absolute top-1/2 left-0 right-0 -translate-y-1/2 rounded-full overflow-hidden" style={{ height: 3, background: "rgba(255,255,255,0.1)" }}>
                <div style={{
                  height: "100%",
                  width: `${progress}%`,
                  background: "rgb(var(--qs-accent))",
                  boxShadow: "0 0 10px rgb(var(--qs-accent) / 0.5)",
                  transition: seekOverride !== null ? "none" : "width 0.5s linear",
                }} />
              </div>
              {/* Range input (invisible) */}
              <input type="range" min={0} max={playback.duration_ms || 1} step={200} value={currentPos}
                onPointerDown={handleSeekStart} onChange={handleSeekChange} onPointerUp={handleSeekCommit}
                className="absolute inset-0 w-full opacity-0 cursor-pointer" />
              {/* Thumb */}
              <div className="absolute top-1/2 -translate-y-1/2 rounded-full pointer-events-none opacity-0 group-hover:opacity-100 transition-opacity duration-150"
                style={{ width: 14, height: 14, left: `calc(${progress}% - 7px)`, background: "rgb(var(--qs-accent))", boxShadow: "0 0 14px rgb(var(--qs-accent) / 0.8)" }} />
            </div>
            <div className="flex justify-between">
              <span className="font-mono text-[10px] tabular-nums" style={{ color: "rgba(255,255,255,0.3)" }}>{fmt(currentPos)}</span>
              <span className="font-mono text-[10px] tabular-nums" style={{ color: "rgba(255,255,255,0.3)" }}>{fmt(playback.duration_ms)}</span>
            </div>
          </div>

          {/* Main controls */}
          <div className="flex items-center justify-center gap-9 mb-5">
            {/* Shuffle */}
            <motion.button whileTap={{ scale: 0.88 }} onClick={handleShuffle} className="relative transition-colors duration-150"
              style={{ color: shuffle ? "rgb(var(--qs-accent))" : "rgba(255,255,255,0.38)" }}>
              <Shuffle className="w-5 h-5" />
              {shuffle && <span className="absolute -bottom-1.5 left-1/2 -translate-x-1/2 w-1 h-1 rounded-full" style={{ background: "rgb(var(--qs-accent))" }} />}
            </motion.button>

            {/* Prev */}
            <motion.button whileTap={{ scale: 0.88 }} onClick={handlePrev}
              className="transition-colors duration-150" style={{ color: "rgba(255,255,255,0.62)" }}>
              <SkipBack className="w-7 h-7" />
            </motion.button>

            {/* Play / Pause */}
            <motion.button
              whileTap={{ scale: 0.92 }}
              onClick={togglePlay}
              className="w-[72px] h-[72px] rounded-full flex items-center justify-center"
              style={{
                background: "rgb(var(--qs-accent))",
                boxShadow: "0 0 45px rgb(var(--qs-accent) / 0.45), 0 0 22px rgb(var(--qs-accent) / 0.22), 0 14px 32px rgba(0,0,0,0.65)",
              }}
            >
              {playback.is_playing
                ? <Pause  style={{ width: 28, height: 28, color: "rgb(var(--qs-bg))" }} />
                : <Play   style={{ width: 28, height: 28, color: "rgb(var(--qs-bg))", marginLeft: 4 }} />
              }
            </motion.button>

            {/* Next */}
            <motion.button whileTap={{ scale: 0.88 }} onClick={handleNext}
              className="transition-colors duration-150" style={{ color: "rgba(255,255,255,0.62)" }}>
              <SkipForward className="w-7 h-7" />
            </motion.button>

            {/* Repeat */}
            <motion.button whileTap={{ scale: 0.88 }} onClick={cycleRepeat} className="relative transition-colors duration-150"
              style={{ color: repeatMode !== "off" ? "rgb(var(--qs-accent))" : "rgba(255,255,255,0.38)" }}>
              <Repeat className="w-5 h-5" />
              {repeatMode === "one" && (
                <span className="absolute -top-2 -right-2 font-mono text-[7px] font-bold rounded-full w-3.5 h-3.5 flex items-center justify-center"
                  style={{ background: "rgb(var(--qs-accent))", color: "#03030a" }}>1</span>
              )}
              {repeatMode !== "off" && (
                <span className="absolute -bottom-1.5 left-1/2 -translate-x-1/2 w-1 h-1 rounded-full" style={{ background: "rgb(var(--qs-accent))" }} />
              )}
            </motion.button>
          </div>

          {/* Volume */}
          <div className="flex items-center gap-3 w-44 mx-auto">
            <button onClick={handleMute} className="flex-shrink-0 transition-colors duration-150" style={{ color: "rgba(255,255,255,0.38)" }}>
              <VolumeIcon className="w-4 h-4" />
            </button>
            <input type="range" min={0} max={1} step={0.01} value={localVolume} onChange={handleVolume}
              className="volume-slider flex-1 cursor-pointer"
              style={{ background: `linear-gradient(to right, rgb(var(--qs-accent)) ${localVolume * 100}%, rgb(var(--qs-accent) / 0.15) ${localVolume * 100}%)` }}
            />
          </div>
        </div>
      </div>

      {/* ──────── QUEUE PANEL — slides from right ──────── */}
      <AnimatePresence>
        {showQueue && (
          <motion.div
            initial={{ x: "100%" }}
            animate={{ x: 0 }}
            exit={{ x: "100%" }}
            transition={{ type: "spring", stiffness: 320, damping: 32 }}
            className="absolute right-0 top-0 bottom-0 z-20 w-80 flex flex-col"
            style={{
              background:     "rgba(3,3,10,0.82)",
              backdropFilter: "blur(40px)",
              borderLeft:     "1px solid rgba(255,255,255,0.07)",
            }}
          >
            {/* Header */}
            <div className="flex items-center gap-2 px-5 py-4 flex-shrink-0" style={{ borderBottom: "1px solid rgba(255,255,255,0.06)" }}>
              <ListMusic className="w-3.5 h-3.5" style={{ color: "rgba(255,255,255,0.28)" }} />
              <span className="font-condensed text-[9px] font-semibold uppercase tracking-[0.22em]" style={{ color: "rgba(255,255,255,0.28)" }}>
                File d'attente
              </span>
              {upcoming.length > 0 && (
                <span className="font-mono text-[9px]" style={{ color: "rgba(255,255,255,0.18)" }}>+{upcoming.length}</span>
              )}
              <button
                onClick={() => setShowQueue(false)}
                className="ml-auto w-6 h-6 flex items-center justify-center rounded-full transition-colors duration-150"
                style={{ color: "rgba(255,255,255,0.28)", background: "rgba(255,255,255,0.06)" }}
                onMouseEnter={e => { e.currentTarget.style.color = "#fff"; e.currentTarget.style.background = "rgba(255,255,255,0.12)"; }}
                onMouseLeave={e => { e.currentTarget.style.color = "rgba(255,255,255,0.28)"; e.currentTarget.style.background = "rgba(255,255,255,0.06)"; }}
              >
                <X style={{ width: 11, height: 11 }} />
              </button>
            </div>

            {/* Currently playing */}
            {track && (
              <div className="mx-4 mt-4 mb-2 p-3 rounded-xl flex-shrink-0"
                style={{ background: "rgb(var(--qs-accent) / 0.06)", border: "1px solid rgb(var(--qs-accent) / 0.12)" }}>
                <p className="font-condensed text-[8px] font-semibold uppercase tracking-[0.18em] mb-2" style={{ color: "rgb(var(--qs-accent))" }}>
                  En lecture
                </p>
                <div className="flex items-center gap-2.5">
                  {track.cover_url && (
                    <img src={track.cover_url} alt={track.album} className="w-10 h-10 rounded-lg object-cover flex-shrink-0" />
                  )}
                  <div className="min-w-0">
                    <p className="font-sans text-xs font-medium text-white truncate">{track.title}</p>
                    <p className="font-sans text-[11px] truncate" style={{ color: "rgba(255,255,255,0.4)" }}>{track.artist}</p>
                  </div>
                </div>
              </div>
            )}

            {/* Upcoming */}
            <div className="flex-1 overflow-y-auto px-4 pb-4 space-y-0.5 mt-1" style={{ scrollbarWidth: "none" }}>
              {upcoming.length === 0 ? (
                <div className="flex flex-col items-center justify-center h-full gap-3 opacity-40">
                  <ListMusic className="w-7 h-7" style={{ color: "rgba(255,255,255,0.3)" }} />
                  <p className="font-condensed text-xs uppercase tracking-wider" style={{ color: "rgba(255,255,255,0.3)" }}>
                    Aucun titre à venir
                  </p>
                </div>
              ) : upcoming.map((t, i) => (
                <div key={t.id}
                  className="flex items-center gap-3 px-2 py-2 rounded-lg cursor-default transition-colors duration-100"
                  onMouseEnter={e => e.currentTarget.style.background = "rgba(255,255,255,0.05)"}
                  onMouseLeave={e => e.currentTarget.style.background = "transparent"}
                >
                  <span className="font-mono text-[10px] w-4 text-right flex-shrink-0" style={{ color: "rgba(255,255,255,0.15)" }}>
                    {i + 1}
                  </span>
                  {t.cover_url
                    ? <img src={t.cover_url} alt={t.album} className="w-8 h-8 rounded-md object-cover flex-shrink-0" />
                    : <div className="w-8 h-8 rounded-md flex-shrink-0" style={{ background: "rgba(255,255,255,0.07)" }} />
                  }
                  <div className="min-w-0 flex-1">
                    <p className="font-sans text-xs truncate" style={{ color: "rgba(255,255,255,0.65)" }}>{t.title}</p>
                    <p className="font-sans text-[10px] truncate" style={{ color: "rgba(255,255,255,0.28)" }}>{t.artist}</p>
                  </div>
                </div>
              ))}
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </motion.div>
  );
}
