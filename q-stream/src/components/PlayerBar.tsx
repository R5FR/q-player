import { useEffect, useRef, useState } from "react";
import { motion } from "framer-motion";
import {
  Play,
  Pause,
  SkipBack,
  SkipForward,
  Volume,
  Volume1,
  Volume2,
  VolumeX,
  Shuffle,
  Repeat,
  Moon,
  Monitor,
  Maximize2,
} from "lucide-react";
import { useStore } from "../store";
import * as api from "../api";

export default function PlayerBar() {
  const {
    playback, setPlayback, setDominantColor,
    shuffle, setShuffle, repeatMode, cycleRepeat,
    setIsSeeking,
    audioDevices, setAudioDevices, selectedDevice, setSelectedDevice,
    sleepTimerEndMs, setSleepTimer,
    setIsFullscreen,
  } = useStore();

  const [seekOverride, setSeekOverride] = useState<number | null>(null);
  const seekClearRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const seekValueRef = useRef<number>(0);
  const prevCoverRef = useRef<string | null>(null);

  const [localVolume, setLocalVolume] = useState(playback.volume);
  const [prevVolume, setPrevVolume] = useState(0.7);
  const [showDeviceMenu, setShowDeviceMenu] = useState(false);
  const [showSleepMenu, setShowSleepMenu] = useState(false);

  const deviceMenuRef = useRef<HTMLDivElement>(null);
  const sleepMenuRef = useRef<HTMLDivElement>(null);

  const track = playback.current_track;

  useEffect(() => {
    setLocalVolume(playback.volume);
  }, [playback.volume]);

  useEffect(() => {
    api.getAudioDevices().then(setAudioDevices).catch(() => {});
  }, []);

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (deviceMenuRef.current && !deviceMenuRef.current.contains(e.target as Node))
        setShowDeviceMenu(false);
      if (sleepMenuRef.current && !sleepMenuRef.current.contains(e.target as Node))
        setShowSleepMenu(false);
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, []);

  useEffect(() => {
    const coverUrl = track?.cover_url;
    if (coverUrl && coverUrl !== prevCoverRef.current) {
      prevCoverRef.current = coverUrl;
      api.extractDominantColor(coverUrl).then(setDominantColor).catch(() => {});
    }
  }, [track?.cover_url]);

  const togglePlay = async () => {
    try {
      if (playback.is_playing) {
        await api.pause();
      } else {
        await api.resume();
      }
      const state = await api.getPlaybackState();
      setPlayback(state);
    } catch (e) {
      console.error(e);
    }
  };

  const handleNext = async () => {
    try {
      const state = await api.nextTrack();
      if (state) setPlayback(state);
    } catch (e) {
      console.error(e);
    }
  };

  const handlePrev = async () => {
    try {
      const state = await api.previousTrack();
      if (state) setPlayback(state);
    } catch (e) {
      console.error(e);
    }
  };

  const handleSeekStart = () => {
    setIsSeeking(true);
  };

  const handleSeekChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const val = Number(e.target.value);
    seekValueRef.current = val;
    setSeekOverride(val);
  };

  const handleSeekCommit = async () => {
    const posMs = seekValueRef.current;
    setSeekOverride(posMs);
    try {
      await api.seek(posMs);
      const state = await api.getPlaybackState();
      setPlayback(state);
    } catch (err) {
      console.error("Seek failed:", err);
    }
    if (seekClearRef.current) clearTimeout(seekClearRef.current);
    seekClearRef.current = setTimeout(() => {
      setSeekOverride(null);
      setIsSeeking(false);
    }, 600);
  };

  const handleShuffle = async () => {
    const next = !shuffle;
    setShuffle(next);
    if (next) {
      try { await api.smartShuffle(); } catch (e) { console.error(e); }
    }
  };

  const handleVolume = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const vol = parseFloat(e.target.value);
    setLocalVolume(vol);
    if (vol > 0) setPrevVolume(vol);
    await api.setVolume(vol);
  };

  const handleMuteToggle = async () => {
    if (localVolume > 0) {
      setPrevVolume(localVolume);
      setLocalVolume(0);
      await api.setVolume(0);
    } else {
      const restore = prevVolume > 0 ? prevVolume : 0.7;
      setLocalVolume(restore);
      await api.setVolume(restore);
    }
  };

  const handleDeviceSelect = async (device: string) => {
    setSelectedDevice(device);
    setShowDeviceMenu(false);
    await api.setAudioDevice(device === "Default" ? null : device);
  };

  const handleSleepTimer = (minutes: number | null) => {
    setSleepTimer(minutes);
    setShowSleepMenu(false);
  };

  const formatTime = (ms: number) => {
    const s = Math.floor(ms / 1000);
    const m = Math.floor(s / 60);
    const sec = s % 60;
    return `${m}:${sec.toString().padStart(2, "0")}`;
  };

  const sleepRemaining = () => {
    if (!sleepTimerEndMs) return null;
    const rem = Math.max(0, sleepTimerEndMs - Date.now());
    const m = Math.ceil(rem / 60000);
    return m > 0 ? `${m}m` : "0m";
  };

  const qualityBadge = () => {
    if (!playback.quality) return null;
    const isHires =
      (playback.bit_depth && playback.bit_depth > 16) ||
      (playback.sample_rate && playback.sample_rate > 44.1);
    return (
      <span className={isHires ? "quality-hires" : "quality-lossless"}>
        {playback.quality}
      </span>
    );
  };

  const VolumeIcon = localVolume === 0 ? VolumeX : localVolume < 0.33 ? Volume : localVolume < 0.66 ? Volume1 : Volume2;

  const currentPos = seekOverride !== null ? seekOverride : playback.position_ms;
  const progress = playback.duration_ms > 0 ? (currentPos / playback.duration_ms) * 100 : 0;

  return (
    <div className="glass-heavy border-t border-qs-text/[0.06] px-5 py-0 relative">
      {/* Lime accent line at top */}
      <div
        className="absolute top-0 left-0 right-0 h-px transition-all duration-1000"
        style={{
          background: `linear-gradient(90deg, transparent 0%, rgb(var(--qs-accent) / 0.5) ${progress}%, rgb(var(--qs-text) / 0.06) ${progress}%, transparent 100%)`,
        }}
      />

      <div className="flex items-center gap-5 h-[80px]">

        {/* ── Track Info ── */}
        <div className="flex items-center gap-3 w-[270px] min-w-[200px]">
          {track ? (
            <>
              <motion.div
                layoutId="player-cover"
                onClick={() => setIsFullscreen(true)}
                className="w-14 h-14 rounded-lg overflow-hidden flex-shrink-0 relative cursor-pointer group/cover"
                style={{
                  boxShadow: playback.is_playing
                    ? "0 0 18px rgb(var(--qs-accent) / 0.25), 0 4px 16px rgb(0 0 0 / 0.5)"
                    : "0 4px 16px rgb(0 0 0 / 0.5)",
                  transition: "box-shadow 0.6s ease",
                }}
              >
                {track.cover_url ? (
                  <img
                    src={track.cover_url}
                    alt={track.album}
                    className="w-full h-full object-cover"
                  />
                ) : (
                  <div className="w-full h-full bg-qs-surface flex items-center justify-center">
                    <span className="text-2xl">🎵</span>
                  </div>
                )}
                {/* Playing indicator overlay */}
                {playback.is_playing && (
                  <div className="absolute inset-0 border border-qs-accent/20 rounded-lg pointer-events-none" />
                )}
                {/* Fullscreen hover overlay */}
                <div className="absolute inset-0 bg-black/55 opacity-0 group-hover/cover:opacity-100 transition-opacity duration-150 flex items-center justify-center pointer-events-none">
                  <Maximize2 className="w-4 h-4 text-white" />
                </div>
              </motion.div>
              <div className="min-w-0 flex-1">
                <p className="font-sans text-sm font-medium text-qs-text truncate leading-snug">
                  {track.title}
                </p>
                <p className="font-sans text-xs text-qs-text-dim truncate mt-0.5">
                  {track.artist}
                </p>
                <div className="mt-1">{qualityBadge()}</div>
              </div>
            </>
          ) : (
            <div className="flex items-center gap-3">
              <div className="w-14 h-14 rounded-lg bg-qs-surface border border-qs-text/6 flex items-center justify-center flex-shrink-0">
                <span className="text-2xl opacity-20">🎵</span>
              </div>
              <p className="font-condensed text-xs uppercase tracking-wider text-qs-text-dim">
                Aucune piste
              </p>
            </div>
          )}
        </div>

        {/* ── Controls & Progress ── */}
        <div className="flex-1 flex flex-col items-center gap-2">
          {/* Transport buttons */}
          <div className="flex items-center gap-5">
            <motion.button
              whileTap={{ scale: 0.9 }}
              onClick={handleShuffle}
              title="Shuffle"
              className={`relative transition-colors ${
                shuffle ? "text-qs-accent" : "text-qs-text-dim hover:text-qs-text"
              }`}
            >
              <Shuffle className="w-4 h-4" />
              {shuffle && (
                <span className="absolute -bottom-1 left-1/2 -translate-x-1/2 w-1 h-1 rounded-full bg-qs-accent shadow-neon-sm" />
              )}
            </motion.button>

            <motion.button
              whileTap={{ scale: 0.9 }}
              onClick={handlePrev}
              className="text-qs-text-dim hover:text-qs-text transition-colors"
            >
              <SkipBack className="w-5 h-5" />
            </motion.button>

            {/* Main play/pause */}
            <motion.button
              whileTap={{ scale: 0.9 }}
              onClick={togglePlay}
              className="btn-play-cyber w-11 h-11 rounded-full flex items-center justify-center"
            >
              {playback.is_playing ? (
                <Pause className="w-4.5 h-4.5 text-qs-accent" style={{ width: 18, height: 18 }} />
              ) : (
                <Play className="text-qs-accent ml-0.5" style={{ width: 18, height: 18 }} />
              )}
            </motion.button>

            <motion.button
              whileTap={{ scale: 0.9 }}
              onClick={handleNext}
              className="text-qs-text-dim hover:text-qs-text transition-colors"
            >
              <SkipForward className="w-5 h-5" />
            </motion.button>

            <motion.button
              whileTap={{ scale: 0.9 }}
              onClick={cycleRepeat}
              title={repeatMode === "off" ? "Repeat off" : repeatMode === "all" ? "Repeat all" : "Repeat one"}
              className={`relative transition-colors ${
                repeatMode !== "off" ? "text-qs-accent" : "text-qs-text-dim hover:text-qs-text"
              }`}
            >
              <Repeat className="w-4 h-4" />
              {repeatMode === "one" && (
                <span className="absolute -top-1.5 -right-1.5 font-mono text-[7px] font-bold leading-none bg-qs-accent text-black rounded-full w-3.5 h-3.5 flex items-center justify-center">
                  1
                </span>
              )}
              {repeatMode !== "off" && (
                <span className="absolute -bottom-1 left-1/2 -translate-x-1/2 w-1 h-1 rounded-full bg-qs-accent shadow-neon-sm" />
              )}
            </motion.button>
          </div>

          {/* Progress / seek */}
          <div className="w-full max-w-[580px] flex items-center gap-3">
            <span className="font-mono text-[10px] text-qs-text-dim w-9 text-right select-none tabular-nums">
              {formatTime(currentPos)}
            </span>
            <div className="flex-1 relative progress-track-container group">
              <div className="progress-track">
                <div className="progress-fill" style={{ width: `${progress}%` }} />
              </div>
              <input
                type="range"
                min={0}
                max={playback.duration_ms || 1}
                step={200}
                value={currentPos}
                onPointerDown={handleSeekStart}
                onChange={handleSeekChange}
                onPointerUp={handleSeekCommit}
                className="absolute inset-0 w-full opacity-0 cursor-pointer"
                style={{ height: "100%", top: 0 }}
              />
              {/* Seek thumb */}
              <div
                className="absolute top-1/2 -translate-y-1/2 w-3 h-3 rounded-full pointer-events-none opacity-0 group-hover:opacity-100 transition-opacity duration-150"
                style={{
                  left: `calc(${progress}% - 6px)`,
                  background: "rgb(var(--qs-accent))",
                  boxShadow: "0 0 10px rgb(var(--qs-accent) / 0.8)",
                }}
              />
            </div>
            <span className="font-mono text-[10px] text-qs-text-dim w-9 select-none tabular-nums">
              {formatTime(playback.duration_ms)}
            </span>
          </div>
        </div>

        {/* ── Right controls ── */}
        <div className="flex items-center gap-3.5 w-[240px] justify-end">
          {/* Volume */}
          <div className="flex items-center gap-1.5">
            <button
              onClick={handleMuteToggle}
              className="text-qs-text-dim hover:text-qs-text transition-colors flex-shrink-0"
            >
              <VolumeIcon className="w-4 h-4" />
            </button>
            <input
              type="range"
              min={0}
              max={1}
              step={0.01}
              value={localVolume}
              onChange={handleVolume}
              className="volume-slider w-20 cursor-pointer"
              style={{
                background: `linear-gradient(to right, rgb(var(--qs-accent)) ${localVolume * 100}%, rgb(var(--qs-accent) / 0.12) ${localVolume * 100}%)`,
              }}
            />
          </div>

          {/* Audio output device */}
          <div className="relative flex-shrink-0" ref={deviceMenuRef}>
            <button
              onClick={() => setShowDeviceMenu(!showDeviceMenu)}
              title="Audio output"
              className={`transition-colors ${
                selectedDevice !== "Default" ? "text-qs-accent" : "text-qs-text-dim hover:text-qs-text"
              }`}
            >
              <Monitor className="w-4 h-4" />
            </button>
            <div
              className={`absolute bottom-full right-0 mb-3 w-56 glass rounded-xl overflow-hidden z-50
                transition-all duration-150 origin-bottom-right
                ${showDeviceMenu ? "opacity-100 scale-100 pointer-events-auto" : "opacity-0 scale-95 pointer-events-none"}`}
            >
              <div className="px-3 py-2 border-b border-qs-text/[0.07]">
                <p className="font-condensed text-[9px] font-semibold text-qs-text-dim uppercase tracking-[0.18em]">
                  Sortie audio
                </p>
              </div>
              {audioDevices.map((dev) => (
                <button
                  key={dev}
                  onClick={() => handleDeviceSelect(dev)}
                  className={`w-full text-left px-3 py-2 font-sans text-xs transition-colors flex items-center gap-2 ${
                    selectedDevice === dev
                      ? "text-qs-accent bg-qs-accent/5"
                      : "text-qs-text-dim hover:text-qs-text hover:bg-qs-accent/5"
                  }`}
                >
                  {selectedDevice === dev && (
                    <span className="w-1 h-1 rounded-full bg-qs-accent flex-shrink-0 shadow-neon-sm" />
                  )}
                  <span className="truncate">{dev}</span>
                </button>
              ))}
            </div>
          </div>

          {/* Sleep timer */}
          <div className="relative flex-shrink-0" ref={sleepMenuRef}>
            <button
              onClick={() => setShowSleepMenu(!showSleepMenu)}
              title="Sleep timer"
              className={`relative transition-colors ${
                sleepTimerEndMs ? "text-qs-accent" : "text-qs-text-dim hover:text-qs-text"
              }`}
            >
              <Moon className="w-4 h-4" />
              {sleepTimerEndMs && (
                <span className="absolute -top-2 -right-2 font-mono text-[8px] font-bold bg-qs-accent text-black rounded-full px-1 leading-tight">
                  {sleepRemaining()}
                </span>
              )}
            </button>
            <div
              className={`absolute bottom-full right-0 mb-3 w-40 glass rounded-xl overflow-hidden z-50
                transition-all duration-150 origin-bottom-right
                ${showSleepMenu ? "opacity-100 scale-100 pointer-events-auto" : "opacity-0 scale-95 pointer-events-none"}`}
            >
              <div className="px-3 py-2 border-b border-qs-text/[0.07]">
                <p className="font-condensed text-[9px] font-semibold text-qs-text-dim uppercase tracking-[0.18em]">
                  Sleep Timer
                </p>
              </div>
              {[null, 15, 30, 45, 60].map((min) => (
                <button
                  key={min ?? "off"}
                  onClick={() => handleSleepTimer(min)}
                  className={`w-full text-left px-3 py-2 font-sans text-xs transition-colors flex items-center gap-2 ${
                    min === null && !sleepTimerEndMs
                      ? "text-qs-accent bg-qs-accent/5"
                      : "text-qs-text-dim hover:text-qs-text hover:bg-qs-accent/5"
                  }`}
                >
                  {min === null ? "Off" : `${min} minutes`}
                </button>
              ))}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
