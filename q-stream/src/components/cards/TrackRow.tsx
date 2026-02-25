import { motion } from "framer-motion";
import { Play, Heart, Plus } from "lucide-react";
import type { QobuzTrack } from "../../types";
import * as api from "../../api";
import { useStore } from "../../store";

interface TrackRowProps {
  track: QobuzTrack;
  index?: number;
  showNumber?: boolean;
  showFavorite?: boolean;
  albumCover?: string;
  albumTitle?: string;
}

export default function TrackRow({
  track,
  index,
  showNumber,
  showFavorite,
  albumCover,
  albumTitle,
}: TrackRowProps) {
  const { setPlayback } = useStore();

  const handlePlay = async () => {
    try {
      const state = await api.playTrack(track.id);
      setPlayback(state);
    } catch (e) {
      console.error(e);
    }
  };

  const handleAddToQueue = async () => {
    const coverUrl = track.album?.image?.large || albumCover;
    await api.addToQueue({
      id: track.id.toString(),
      title: track.title,
      artist: track.performer?.name || "Unknown",
      album: track.album?.title || albumTitle || "Unknown",
      duration_seconds: track.duration,
      cover_url: coverUrl,
      source: { Qobuz: { track_id: track.id } },
      quality_label: track.maximum_bit_depth
        ? `${track.maximum_bit_depth}-bit/${track.maximum_sampling_rate || 44.1}kHz`
        : undefined,
      sample_rate: track.maximum_sampling_rate,
      bit_depth: track.maximum_bit_depth,
    });
  };

  const formatDuration = (seconds: number) => {
    const m = Math.floor(seconds / 60);
    const s = seconds % 60;
    return `${m}:${s.toString().padStart(2, "0")}`;
  };

  const isHires =
    (track.maximum_bit_depth && track.maximum_bit_depth > 16) ||
    (track.maximum_sampling_rate && track.maximum_sampling_rate > 44.1);

  return (
    <motion.div
      whileHover={{ backgroundColor: "rgba(255,255,255,0.03)" }}
      className="grid grid-cols-[40px_1fr_1fr_80px] gap-4 px-4 py-2.5 rounded-lg group items-center cursor-pointer"
      onClick={handlePlay}
    >
      {/* Number / Play */}
      <div className="flex items-center justify-center">
        {showNumber && (
          <span className="text-sm text-qs-text-dim group-hover:hidden">
            {index ?? track.track_number}
          </span>
        )}
        <Play className={`w-3.5 h-3.5 text-white ${showNumber ? "hidden group-hover:block" : "opacity-0 group-hover:opacity-100"}`} />
      </div>

      {/* Title + Album cover */}
      <div className="flex items-center gap-3 min-w-0">
        {track.album?.image?.thumbnail && !showNumber && (
          <div className="w-8 h-8 rounded overflow-hidden flex-shrink-0">
            <img
              src={track.album.image.thumbnail}
              alt=""
              className="w-full h-full object-cover"
            />
          </div>
        )}
        <div className="min-w-0">
          <p className="text-sm text-white truncate">{track.title}</p>
          {track.explicit && (
            <span className="inline-block text-[9px] bg-white/10 text-qs-text-dim px-1 rounded mt-0.5">
              E
            </span>
          )}
        </div>
        {isHires && <span className="quality-hires text-[8px] flex-shrink-0">Hi-Res</span>}
      </div>

      {/* Artist */}
      <p className="text-sm text-qs-text-dim truncate">
        {track.performer?.name || "Unknown"}
      </p>

      {/* Duration + Actions */}
      <div className="flex items-center justify-end gap-2">
        <button
          onClick={(e) => {
            e.stopPropagation();
            handleAddToQueue();
          }}
          className="opacity-0 group-hover:opacity-100 transition"
          title="Add to queue"
        >
          <Plus className="w-3.5 h-3.5 text-qs-text-dim hover:text-white" />
        </button>
        {showFavorite && (
          <button
            onClick={(e) => {
              e.stopPropagation();
              api.removeFavorite("track", track.id.toString());
            }}
            className="opacity-0 group-hover:opacity-100 transition"
          >
            <Heart className="w-3.5 h-3.5 text-red-400 fill-red-400" />
          </button>
        )}
        <span className="text-xs text-qs-text-dim">
          {formatDuration(track.duration)}
        </span>
      </div>
    </motion.div>
  );
}
