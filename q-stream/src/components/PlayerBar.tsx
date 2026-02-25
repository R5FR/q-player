import { useEffect, useRef, useState } from "react";
import { motion } from "framer-motion";
import {
  Play,
  Pause,
  SkipBack,
  SkipForward,
  Volume2,
  VolumeX,
  Shuffle,
  Repeat,
  ChevronUp,
} from "lucide-react";
import { useStore } from "../store";
import * as api from "../api";

export default function PlayerBar() {
  const { playback, setPlayback, setDominantColor, shuffle, setShuffle, repeatMode, cycleRepeat } = useStore();
  const [showVolSlider, setShowVolSlider] = useState(false);
  const progressRef = useRef<HTMLDivElement>(null);
  const prevCoverRef = useRef<string | null>(null);

  const track = playback.current_track;
  const progress =
    playback.duration_ms > 0
      ? (playback.position_ms / playback.duration_ms) * 100
      : 0;

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

  const handleSeek = (e: React.MouseEvent<HTMLDivElement>) => {
    if (!progressRef.current || playback.duration_ms === 0) return;
    const rect = progressRef.current.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const pct = Math.max(0, Math.min(1, x / rect.width));
    const posMs = Math.round(pct * playback.duration_ms);
    api.seek(posMs);
  };

  const handleShuffle = async () => {
    const next = !shuffle;
    setShuffle(next);
    if (next) {
      try {
        await api.smartShuffle();
      } catch (e) {
        console.error(e);
      }
    }
  };

  const handleVolume = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const vol = parseFloat(e.target.value);
    await api.setVolume(vol);
  };

  const formatTime = (ms: number) => {
    const s = Math.floor(ms / 1000);
    const m = Math.floor(s / 60);
    const sec = s % 60;
    return `${m}:${sec.toString().padStart(2, "0")}`;
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

  return (
    <div className="glass-heavy border-t border-white/10 px-4 py-2">
      <div className="flex items-center gap-4 h-[72px]">
        {/* Track Info */}
        <div className="flex items-center gap-3 w-[280px] min-w-[200px]">
          {track ? (
            <>
              <motion.div
                layoutId="player-cover"
                className="w-14 h-14 rounded-lg overflow-hidden shadow-lg flex-shrink-0"
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
              </motion.div>
              <div className="min-w-0">
                <p className="text-sm font-medium text-white truncate">
                  {track.title}
                </p>
                <p className="text-xs text-qs-text-dim truncate">
                  {track.artist}
                </p>
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
              className={`transition relative ${shuffle ? 'text-qs-accent' : 'text-qs-text-dim hover:text-white'}`}
            >
              <Shuffle className="w-4 h-4" />
              {shuffle && <span className="absolute -bottom-1 left-1/2 -translate-x-1/2 w-1 h-1 rounded-full bg-qs-accent" />}
            </button>
            <button
              onClick={handlePrev}
              className="text-qs-text-dim hover:text-white transition"
            >
              <SkipBack className="w-5 h-5" />
            </button>
            <motion.button
              whileTap={{ scale: 0.9 }}
              onClick={togglePlay}
              className="w-9 h-9 rounded-full bg-white flex items-center justify-center hover:scale-105 transition"
            >
              {playback.is_playing ? (
                <Pause className="w-4 h-4 text-black" />
              ) : (
                <Play className="w-4 h-4 text-black ml-0.5" />
              )}
            </motion.button>
            <button
              onClick={handleNext}
              className="text-qs-text-dim hover:text-white transition"
            >
              <SkipForward className="w-5 h-5" />
            </button>
            <button
              onClick={cycleRepeat}
              title={repeatMode === 'off' ? 'Repeat off' : repeatMode === 'all' ? 'Repeat all' : 'Repeat one'}
              className={`transition relative ${repeatMode !== 'off' ? 'text-qs-accent' : 'text-qs-text-dim hover:text-white'}`}
            >
              <Repeat className="w-4 h-4" />
              {repeatMode === 'one' && (
                <span className="absolute -top-1.5 -right-1.5 text-[8px] font-bold leading-none bg-qs-accent text-black rounded-full w-3 h-3 flex items-center justify-center">1</span>
              )}
              {repeatMode !== 'off' && <span className="absolute -bottom-1 left-1/2 -translate-x-1/2 w-1 h-1 rounded-full bg-qs-accent" />}
            </button>
          </div>

          {/* Progress bar */}
          <div className="w-full max-w-[600px] flex items-center gap-2">
            <span className="text-[10px] text-qs-text-dim w-10 text-right">
              {formatTime(playback.position_ms)}
            </span>
            <div
              ref={progressRef}
              onClick={handleSeek}
              className="flex-1 h-1 bg-white/10 rounded-full cursor-pointer group relative"
            >
              <motion.div
                className="h-full bg-white rounded-full relative"
                style={{ width: `${progress}%` }}
              >
                <div className="absolute right-0 top-1/2 -translate-y-1/2 w-3 h-3 bg-white rounded-full opacity-0 group-hover:opacity-100 transition shadow" />
              </motion.div>
            </div>
            <span className="text-[10px] text-qs-text-dim w-10">
              {formatTime(playback.duration_ms)}
            </span>
          </div>
        </div>

        {/* Volume */}
        <div className="flex items-center gap-2 w-[180px] justify-end">
          <button
            onClick={() => setShowVolSlider(!showVolSlider)}
            className="text-qs-text-dim hover:text-white transition"
          >
            {playback.volume === 0 ? (
              <VolumeX className="w-4 h-4" />
            ) : (
              <Volume2 className="w-4 h-4" />
            )}
          </button>
          <input
            type="range"
            min={0}
            max={1}
            step={0.01}
            value={playback.volume}
            onChange={handleVolume}
            className="w-24 h-1 accent-white bg-white/10 rounded-full appearance-none cursor-pointer
              [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-3 [&::-webkit-slider-thumb]:h-3
              [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-white"
          />
        </div>
      </div>
    </div>
  );
}
