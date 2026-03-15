import { useEffect, useRef, useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
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
  SlidersHorizontal,
  Moon,
  Monitor,
  ChevronDown,
} from "lucide-react";
import { useStore } from "../store";
import * as api from "../api";
import EqPanel from "./EqPanel";

export default function PlayerBar() {
  const {
    playback, setPlayback, setDominantColor,
    shuffle, setShuffle, repeatMode, cycleRepeat,
    isSeeking, setIsSeeking,
    showEqPanel, setShowEqPanel,
    eqEnabled, eqBands,
    audioDevices, setAudioDevices, selectedDevice, setSelectedDevice,
    sleepTimerEndMs, setSleepTimer,
  } = useStore();

  const [seekOverride, setSeekOverride] = useState<number | null>(null);
  const seekClearRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  // Tracks the latest drag value independently of React render cycle
  const seekValueRef = useRef<number>(0);
  const prevCoverRef = useRef<string | null>(null);

  // Local volume for immediate UI feedback (avoids 500ms polling lag)
  const [localVolume, setLocalVolume] = useState(playback.volume);
  const [prevVolume, setPrevVolume] = useState(0.7);
  const [showDeviceMenu, setShowDeviceMenu] = useState(false);
  const [showSleepMenu, setShowSleepMenu] = useState(false);

  const deviceMenuRef = useRef<HTMLDivElement>(null);
  const sleepMenuRef = useRef<HTMLDivElement>(null);

  const track = playback.current_track;

  // Keep localVolume in sync with backend polling
  useEffect(() => {
    setLocalVolume(playback.volume);
  }, [playback.volume]);

  // Load audio devices on mount
  useEffect(() => {
    api.getAudioDevices().then(setAudioDevices).catch(() => {});
  }, []);

  // Close dropdowns on outside click
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

  // Extract dominant color when cover changes
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

  // Seek — onPointerDown = lock polling, onChange = live preview, onPointerUp = commit
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
    <div className="glass-heavy border-t border-white/10 px-4 py-2 relative">
      <div className="flex items-center gap-4 h-[72px]">
        {/* Track Info */}
        <div className="flex items-center gap-3 w-[280px] min-w-[200px]">
          {track ? (
            <>
              <motion.div layoutId="player-cover" className="w-14 h-14 rounded-lg overflow-hidden shadow-lg flex-shrink-0">
                {track.cover_url ? (
                  <img src={track.cover_url} alt={track.album} className="w-full h-full object-cover" />
                ) : (
                  <div className="w-full h-full bg-qs-surface flex items-center justify-center">
                    <span className="text-2xl">🎵</span>
                  </div>
                )}
              </motion.div>
              <div className="min-w-0">
                <p className="text-sm font-medium text-white truncate">{track.title}</p>
                <p className="text-xs text-qs-text-dim truncate">{track.artist}</p>
                <div className="mt-0.5">{qualityBadge()}</div>
              </div>
            </>
          ) : (
            <div className="flex items-center gap-3">
              <div className="w-14 h-14 rounded-lg bg-qs-surface flex items-center justify-center">
                <span className="text-2xl opacity-30">🎵</span>
              </div>
              <p className="text-sm text-qs-text-dim">No track playing</p>
            </div>
          )}
        </div>

        {/* Controls & Progress */}
        <div className="flex-1 flex flex-col items-center gap-1.5">
          {/* Buttons */}
          <div className="flex items-center gap-4">
            <button
              onClick={handleShuffle}
              title="Shuffle"
              className={`transition relative ${shuffle ? "text-qs-accent" : "text-qs-text-dim hover:text-white"}`}
            >
              <Shuffle className="w-4 h-4" />
              {shuffle && <span className="absolute -bottom-1 left-1/2 -translate-x-1/2 w-1 h-1 rounded-full bg-qs-accent" />}
            </button>
            <button onClick={handlePrev} className="text-qs-text-dim hover:text-white transition">
              <SkipBack className="w-5 h-5" />
            </button>
            <motion.button
              whileTap={{ scale: 0.9 }}
              onClick={togglePlay}
              className="btn-play-cyber w-9 h-9 rounded-full flex items-center justify-center"
            >
              {playback.is_playing ? (
                <Pause className="w-4 h-4 text-qs-accent" />
              ) : (
                <Play className="w-4 h-4 text-qs-accent ml-0.5" />
              )}
            </motion.button>
            <button onClick={handleNext} className="text-qs-text-dim hover:text-white transition">
              <SkipForward className="w-5 h-5" />
            </button>
            <button
              onClick={cycleRepeat}
              title={repeatMode === "off" ? "Repeat off" : repeatMode === "all" ? "Repeat all" : "Repeat one"}
              className={`transition relative ${repeatMode !== "off" ? "text-qs-accent" : "text-qs-text-dim hover:text-white"}`}
            >
              <Repeat className="w-4 h-4" />
              {repeatMode === "one" && (
                <span className="absolute -top-1.5 -right-1.5 text-[8px] font-bold leading-none bg-qs-accent text-black rounded-full w-3 h-3 flex items-center justify-center">1</span>
              )}
              {repeatMode !== "off" && <span className="absolute -bottom-1 left-1/2 -translate-x-1/2 w-1 h-1 rounded-full bg-qs-accent" />}
            </button>
          </div>

          {/* Progress / seek */}
          <div className="w-full max-w-[600px] flex items-center gap-2">
            <span className="text-[10px] text-qs-text-dim w-10 text-right select-none font-mono">
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
              <div
                className="absolute top-1/2 -translate-y-1/2 w-2.5 h-2.5 rounded-full pointer-events-none opacity-0 group-hover:opacity-100 transition-opacity duration-150"
                style={{
                  left: `calc(${progress}% - 5px)`,
                  background: "#00d4ff",
                  boxShadow: "0 0 8px rgba(0,212,255,0.8)",
                }}
              />
            </div>
            <span className="text-[10px] text-qs-text-dim w-10 select-none font-mono">
              {formatTime(playback.duration_ms)}
            </span>
          </div>
        </div>

        {/* Right controls: Volume + EQ + Device + Sleep */}
        <div className="flex items-center gap-3 w-[240px] justify-end">
          {/* Volume */}
          <div className="flex items-center gap-1.5">
            <button onClick={handleMuteToggle} className="text-qs-text-dim hover:text-white transition flex-shrink-0">
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
                background: `linear-gradient(to right, #00d4ff ${localVolume * 100}%, rgba(0,212,255,0.15) ${localVolume * 100}%)`,
              }}
            />
          </div>

          {/* EQ toggle */}
          <button
            onClick={() => setShowEqPanel(!showEqPanel)}
            title="Equalizer"
            className={`transition flex-shrink-0 ${eqEnabled || showEqPanel ? "text-qs-accent" : "text-qs-text-dim hover:text-white"}`}
          >
            <SlidersHorizontal className="w-4 h-4" />
          </button>

          {/* Audio output device */}
          <div className="relative flex-shrink-0" ref={deviceMenuRef}>
            <button
              onClick={() => setShowDeviceMenu(!showDeviceMenu)}
              title="Audio output"
              className={`transition ${selectedDevice !== "Default" ? "text-qs-accent" : "text-qs-text-dim hover:text-white"}`}
            >
              <Monitor className="w-4 h-4" />
            </button>
            <AnimatePresence>
              {showDeviceMenu && (
                <motion.div
                  initial={{ opacity: 0, y: 8 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0, y: 8 }}
                  className="absolute bottom-full right-0 mb-2 w-56 glass rounded-xl border border-white/10 overflow-hidden z-50"
                >
                  <div className="px-3 py-2 border-b border-white/5">
                    <p className="text-[10px] text-qs-text-dim uppercase tracking-wider font-semibold">Audio Output</p>
                  </div>
                  {audioDevices.map((dev) => (
                    <button
                      key={dev}
                      onClick={() => handleDeviceSelect(dev)}
                      className={`w-full text-left px-3 py-2 text-xs transition flex items-center gap-2 ${
                        selectedDevice === dev ? "text-qs-accent bg-qs-accent/5" : "text-qs-text-dim hover:text-white hover:bg-white/5"
                      }`}
                    >
                      {selectedDevice === dev && <span className="w-1 h-1 rounded-full bg-qs-accent flex-shrink-0" />}
                      <span className="truncate">{dev}</span>
                    </button>
                  ))}
                </motion.div>
              )}
            </AnimatePresence>
          </div>

          {/* Sleep timer */}
          <div className="relative flex-shrink-0" ref={sleepMenuRef}>
            <button
              onClick={() => setShowSleepMenu(!showSleepMenu)}
              title="Sleep timer"
              className={`transition relative ${sleepTimerEndMs ? "text-qs-accent" : "text-qs-text-dim hover:text-white"}`}
            >
              <Moon className="w-4 h-4" />
              {sleepTimerEndMs && (
                <span className="absolute -top-2 -right-2 text-[8px] font-bold bg-qs-accent text-black rounded-full px-1 leading-tight">
                  {sleepRemaining()}
                </span>
              )}
            </button>
            <AnimatePresence>
              {showSleepMenu && (
                <motion.div
                  initial={{ opacity: 0, y: 8 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0, y: 8 }}
                  className="absolute bottom-full right-0 mb-2 w-40 glass rounded-xl border border-white/10 overflow-hidden z-50"
                >
                  <div className="px-3 py-2 border-b border-white/5">
                    <p className="text-[10px] text-qs-text-dim uppercase tracking-wider font-semibold">Sleep Timer</p>
                  </div>
                  {[null, 15, 30, 45, 60].map((min) => (
                    <button
                      key={min ?? "off"}
                      onClick={() => handleSleepTimer(min)}
                      className={`w-full text-left px-3 py-2 text-xs transition flex items-center gap-2 ${
                        (min === null && !sleepTimerEndMs) ? "text-qs-accent bg-qs-accent/5" : "text-qs-text-dim hover:text-white hover:bg-white/5"
                      }`}
                    >
                      {min === null ? "Off" : `${min} minutes`}
                    </button>
                  ))}
                </motion.div>
              )}
            </AnimatePresence>
          </div>
        </div>
      </div>

      {/* EQ Panel (floats above the player bar) */}
      <AnimatePresence>
        {showEqPanel && <EqPanel />}
      </AnimatePresence>
    </div>
  );
}
