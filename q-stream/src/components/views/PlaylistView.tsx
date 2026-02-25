import { motion } from "framer-motion";
import { Clock, Play } from "lucide-react";
import { useStore } from "../../store";
import * as api from "../../api";
import TrackRow from "../cards/TrackRow";

export default function PlaylistView() {
  const { playlistDetail } = useStore();

  if (!playlistDetail) {
    return (
      <div className="flex items-center justify-center h-full text-qs-text-dim">
        No playlist selected
      </div>
    );
  }

  const pl = playlistDetail;
  const coverUrl = pl.images300?.[0] || pl.image_rectangle_mini?.[0];

  const playAll = async () => {
    if (!pl.tracks?.items?.length) return;
    await api.clearQueue();
    for (const track of pl.tracks.items) {
      await api.addToQueue({
        id: track.id.toString(),
        title: track.title,
        artist: track.performer?.name || "Unknown",
        album: track.album?.title || "Unknown",
        duration_seconds: track.duration,
        cover_url: track.album?.image?.large || coverUrl,
        source: { Qobuz: { track_id: track.id } },
        quality_label: track.maximum_bit_depth
          ? `${track.maximum_bit_depth}-bit/${track.maximum_sampling_rate || 44.1}kHz`
          : undefined,
        sample_rate: track.maximum_sampling_rate,
        bit_depth: track.maximum_bit_depth,
      });
    }
    await api.playTrack(pl.tracks.items[0].id);
  };

  return (
    <div className="p-8">
      {/* Header */}
      <div className="flex gap-6 mb-8">
        <motion.div
          initial={{ opacity: 0, scale: 0.9 }}
          animate={{ opacity: 1, scale: 1 }}
          className="w-56 h-56 rounded-xl overflow-hidden shadow-2xl flex-shrink-0"
        >
          {coverUrl ? (
            <img src={coverUrl} alt={pl.name} className="w-full h-full object-cover" />
          ) : (
            <div className="w-full h-full bg-qs-surface flex items-center justify-center text-5xl">
              🎶
            </div>
          )}
        </motion.div>

        <div className="flex flex-col justify-end">
          <p className="text-xs text-qs-text-dim uppercase tracking-wider mb-1">Playlist</p>
          <h1 className="text-4xl font-bold text-white mb-2">{pl.name}</h1>
          {pl.description && (
            <p className="text-sm text-qs-text-dim mb-2 max-w-md">{pl.description}</p>
          )}
          <div className="flex items-center gap-2 text-sm text-qs-text-dim">
            {pl.owner?.name && <span>{pl.owner.name}</span>}
            {pl.tracks_count && (
              <>
                <span>•</span>
                <span>{pl.tracks_count} tracks</span>
              </>
            )}
          </div>

          <motion.button
            whileTap={{ scale: 0.95 }}
            onClick={playAll}
            className="flex items-center gap-2 px-6 py-2.5 bg-qs-accent rounded-full text-sm font-semibold text-white hover:bg-qs-accent-light transition mt-4 w-fit"
          >
            <Play className="w-4 h-4" />
            Play All
          </motion.button>
        </div>
      </div>

      {/* Track list */}
      <div className="space-y-0.5">
        <div className="grid grid-cols-[40px_1fr_1fr_80px] gap-4 px-4 py-2 text-xs text-qs-text-dim uppercase tracking-wider border-b border-white/5">
          <span>#</span>
          <span>Title</span>
          <span>Artist</span>
          <span className="text-right">
            <Clock className="w-3.5 h-3.5 inline" />
          </span>
        </div>
        {pl.tracks?.items?.map((track, i) => (
          <TrackRow key={track.id} track={track} index={i + 1} showNumber />
        ))}
      </div>
    </div>
  );
}
