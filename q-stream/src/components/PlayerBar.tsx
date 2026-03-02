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
} from "lucide-react";
import { useStore } from "../store";
import * as api from "../api";

export default function PlayerBar() {
  const { playback, setPlayback, setDominantColor, shuffle, setShuffle, repeatMode, cycleRepeat } = useStore();
  const [showVolSlider, setShowVolSlider] = useState(false);
  const [seekOverride, setSeekOverride] = useState<number | null>(null);
  const seekClearRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const prevCoverRef = useRef<string | null>(null);

  const track = playback.current_track;

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

  // Seek via native range input — onChange = live preview, onPointerUp = commit to backend
  const handleSeekChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setSeekOverride(Number(e.target.value));
  };

  const handleSeekCommit = async (e: React.PointerEvent<HTMLInputElement>) => {
    const posMs = Number((e.target as HTMLInputElement).value);
    setSeekOverride(posMs);
    try {
      await api.seek(posMs);
      // Refresh immediately — critical when restarting a finished track so the
      // UI switches back to "playing" without waiting for the next 500ms poll.
      const state = await api.getPlaybackState();
      setPlayback(state);
    } catch (err) {
      console.error(err);
    }
    // Keep override briefly while the decoder thread processes the seek
    if (seekClearRef.current) clearTimeout(seekClearRef.current);
    seekClearRef.current = setTimeout(() => setSeekOverride(null), 300);
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
              className="btn-play-cyber w-9 h-9 rounded-full flex items-center justify-center"
            >
              {playback.is_playing ? (
                <Pause className="w-4 h-4 text-qs-accent" />
              ) : (
                <Play className="w-4 h-4 text-qs-accent ml-0.5" />
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

          {/* Progress / seek */}
          <div className="w-full max-w-[600px] flex items-center gap-2">
            <span className="text-[10px] text-qs-text-dim w-10 text-right select-none font-mono">
              {formatTime(seekOverride !== null ? seekOverride : playback.position_ms)}
            </span>
            <div className="flex-1 relative progress-track-container group">
              <div className="progress-track">
                <div
                  className="progress-fill"
                  style={{
                    width: `${playback.duration_ms > 0 ? ((seekOverride !== null ? seekOverride : playback.position_ms) / playback.duration_ms) * 100 : 0}%`,
                  }}
                />
              </div>
              {/* Invisible range input on top for interaction */}
              <input
                type="range"
                min={0}
                max={playback.duration_ms || 1}
                step={500}
                value={seekOverride !== null ? seekOverride : playback.position_ms}
                onChange={handleSeekChange}
                onPointerUp={handleSeekCommit}
                className="absolute inset-0 w-full opacity-0 cursor-pointer"
                style={{ height: '20px', top: '-8px' }}
              />
              {/* Thumb indicator */}
              <div
                className="absolute top-1/2 -translate-y-1/2 w-2.5 h-2.5 rounded-full pointer-events-none
                  opacity-0 group-hover:opacity-100 transition-opacity duration-150"
                style={{
                  left: `calc(${playback.duration_ms > 0 ? ((seekOverride !== null ? seekOverride : playback.position_ms) / playback.duration_ms) * 100 : 0}% - 5px)`,
                  background: '#00d4ff',
                  boxShadow: '0 0 8px rgba(0,212,255,0.8)',
                }}
              />
            </div>
            <span className="text-[10px] text-qs-text-dim w-10 select-none font-mono">
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
            className="w-24 h-1 rounded-full appearance-none cursor-pointer bg-qs-text-dim/20
              accent-qs-accent
              [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-2.5 [&::-webkit-slider-thumb]:h-2.5
              [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-qs-accent
              [&::-webkit-slider-thumb]:shadow-[0_0_6px_rgba(0,212,255,0.7)]"
          />
        </div>
      </div>
    </div>
  );
}
