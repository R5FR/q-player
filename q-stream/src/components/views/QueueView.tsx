import { useEffect } from "react";
import { motion } from "framer-motion";
import { ListMusic, Sparkles, Trash2, Play } from "lucide-react";
import { useStore } from "../../store";
import * as api from "../../api";

export default function QueueView() {
  const { queue, setQueue, playback } = useStore();

  useEffect(() => {
    api.getQueue().then(setQueue).catch(console.error);
  }, []);

  const handleSmartShuffle = async () => {
    try {
      const q = await api.smartShuffle();
      setQueue(q);
    } catch (e) {
      console.error(e);
    }
  };

  const handleClear = async () => {
    await api.clearQueue();
    setQueue({ tracks: [] });
  };

  const formatTime = (seconds: number) => {
    const m = Math.floor(seconds / 60);
    const s = seconds % 60;
    return `${m}:${s.toString().padStart(2, "0")}`;
  };

  return (
    <div className="p-8">
      <div className="flex items-center justify-between mb-6">
        <div className="flex items-center gap-3">
          <ListMusic className="w-6 h-6 text-qs-accent" />
          <h1 className="text-2xl font-bold text-white">Queue</h1>
          <span className="text-sm text-qs-text-dim">
            {queue.tracks.length} tracks
          </span>
        </div>
        <div className="flex gap-2">
          <motion.button
            whileTap={{ scale: 0.95 }}
            onClick={handleSmartShuffle}
            className="flex items-center gap-2 px-4 py-2 glass rounded-xl text-sm font-medium hover:bg-white/10 transition"
          >
            <Sparkles className="w-4 h-4 text-amber-400" />
            Smart Shuffle
          </motion.button>
          <button
            onClick={handleClear}
            className="flex items-center gap-2 px-4 py-2 glass rounded-xl text-sm font-medium text-red-400 hover:bg-red-500/10 transition"
          >
            <Trash2 className="w-4 h-4" />
            Clear
          </button>
        </div>
      </div>

      {queue.tracks.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-20 text-qs-text-dim">
          <ListMusic className="w-12 h-12 mb-4 opacity-30" />
          <p className="text-lg">Your queue is empty</p>
          <p className="text-sm mt-1">
            Add tracks from search, albums, or use Smart Shuffle
          </p>
        </div>
      ) : (
        <div className="space-y-1">
          {queue.tracks.map((track, i) => {
            const isCurrent = i === queue.current_index;
            return (
              <motion.div
                key={`${track.id}-${i}`}
                initial={{ opacity: 0, x: -10 }}
                animate={{ opacity: 1, x: 0 }}
                transition={{ delay: i * 0.02 }}
                className={`flex items-center gap-4 px-4 py-3 rounded-xl transition group ${
                  isCurrent
                    ? "bg-qs-accent/10 border border-qs-accent/20"
                    : "hover:bg-white/5"
                }`}
              >
                <div className="w-8 text-center">
                  {isCurrent ? (
                    <div className="flex items-center gap-0.5 justify-center">
                      <div className="w-0.5 bg-qs-accent rounded eq-bar" />
                      <div className="w-0.5 bg-qs-accent rounded eq-bar" />
                      <div className="w-0.5 bg-qs-accent rounded eq-bar" />
                      <div className="w-0.5 bg-qs-accent rounded eq-bar" />
                    </div>
                  ) : (
                    <span className="text-sm text-qs-text-dim">{i + 1}</span>
                  )}
                </div>

                <div className="w-10 h-10 rounded-lg overflow-hidden bg-qs-surface flex-shrink-0">
                  {track.cover_url ? (
                    <img
                      src={track.cover_url}
                      alt=""
                      className="w-full h-full object-cover"
                    />
                  ) : (
                    <div className="w-full h-full flex items-center justify-center text-lg">
                      🎵
                    </div>
                  )}
                </div>

                <div className="flex-1 min-w-0">
                  <p
                    className={`text-sm font-medium truncate ${
                      isCurrent ? "text-qs-accent-light" : "text-white"
                    }`}
                  >
                    {track.title}
                  </p>
                  <p className="text-xs text-qs-text-dim truncate">
                    {track.artist} • {track.album}
                  </p>
                </div>

                {track.quality_label && (
                  <span
                    className={
                      track.bit_depth && track.bit_depth > 16
                        ? "quality-hires"
                        : "quality-lossless"
                    }
                  >
                    {track.quality_label}
                  </span>
                )}

                {"Local" in track.source && (
                  <span className="quality-local">Local</span>
                )}

                <span className="text-xs text-qs-text-dim w-12 text-right">
                  {formatTime(track.duration_seconds)}
                </span>

                <button
                  onClick={async () => {
                    if ("Qobuz" in track.source) {
                      await api.playTrack(track.source.Qobuz.track_id);
                    } else if ("Local" in track.source) {
                      await api.playLocalTrack(track.source.Local.file_path);
                    }
                  }}
                  className="opacity-0 group-hover:opacity-100 transition"
                >
                  <Play className="w-4 h-4 text-white" />
                </button>
              </motion.div>
            );
          })}
        </div>
      )}
    </div>
  );
}
