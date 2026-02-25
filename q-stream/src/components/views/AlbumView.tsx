import { motion } from "framer-motion";
import { Clock, Play } from "lucide-react";
import { useStore } from "../../store";
import * as api from "../../api";
import TrackRow from "../cards/TrackRow";

export default function AlbumView() {
  const { albumDetail } = useStore();

  if (!albumDetail) {
    return (
      <div className="flex items-center justify-center h-full text-qs-text-dim">
        No album selected
      </div>
    );
  }

  const album = albumDetail;
  const coverUrl = album.image?.large || album.image?.small;
  const isHires = album.hires_available;

  const playAll = async () => {
    if (!album.tracks?.items?.length) return;
    await api.clearQueue();
    for (const track of album.tracks.items) {
      await api.addToQueue({
        id: track.id.toString(),
        title: track.title,
        artist: track.performer?.name || album.artist?.name || "Unknown",
        album: album.title,
        duration_seconds: track.duration,
        cover_url: coverUrl,
        source: { Qobuz: { track_id: track.id } },
        quality_label: track.maximum_bit_depth
          ? `${track.maximum_bit_depth}-bit/${track.maximum_sampling_rate || 44.1}kHz`
          : undefined,
        sample_rate: track.maximum_sampling_rate,
        bit_depth: track.maximum_bit_depth,
      });
    }
    // Play first track
    await api.playTrack(album.tracks.items[0].id);
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
            <img src={coverUrl} alt={album.title} className="w-full h-full object-cover" />
          ) : (
            <div className="w-full h-full bg-qs-surface flex items-center justify-center text-5xl">
              💿
            </div>
          )}
        </motion.div>

        <div className="flex flex-col justify-end">
          <p className="text-xs text-qs-text-dim uppercase tracking-wider mb-1">Album</p>
          <h1 className="text-4xl font-bold text-white mb-2">{album.title}</h1>
          <div className="flex items-center gap-2 text-sm text-qs-text-dim">
            <span className="text-white font-medium">{album.artist?.name}</span>
            {album.release_date_original && (
              <>
                <span>•</span>
                <span>{album.release_date_original.split("-")[0]}</span>
              </>
            )}
            {album.tracks_count && (
              <>
                <span>•</span>
                <span>{album.tracks_count} tracks</span>
              </>
            )}
            {isHires && <span className="quality-hires ml-2">Hi-Res</span>}
          </div>
          {album.label && (
            <p className="text-xs text-qs-text-dim mt-1">{album.label.name}</p>
          )}

          <div className="flex items-center gap-3 mt-4">
            <motion.button
              whileTap={{ scale: 0.95 }}
              onClick={playAll}
              className="flex items-center gap-2 px-6 py-2.5 bg-qs-accent rounded-full text-sm font-semibold text-white hover:bg-qs-accent-light transition"
            >
              <Play className="w-4 h-4" />
              Play All
            </motion.button>
          </div>
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
        {album.tracks?.items?.map((track) => (
          <TrackRow
            key={track.id}
            track={track}
            showNumber
            albumCover={coverUrl}
            albumTitle={album.title}
          />
        ))}
      </div>
    </div>
  );
}
