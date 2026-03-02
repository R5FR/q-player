import { useState } from "react";
import { motion } from "framer-motion";
import { Clock, Play } from "lucide-react";
import { useStore } from "../../store";
import * as api from "../../api";
import TrackRow from "../cards/TrackRow";

export default function AlbumView() {
  const { albumDetail, setPlayback } = useStore();
  const [loadingIdx, setLoadingIdx] = useState<number | null>(null);

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

  /**
   * Build the full album queue then play from the given index.
   * Using `play_from_queue` avoids a redundant Qobuz catalog search —
   * the metadata is already embedded in each queue entry.
   */
  const handlePlayAlbumTrack = async (trackIndex: number) => {
    if (!album.tracks?.items?.length) return;
    setLoadingIdx(trackIndex);
    try {
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
      const state = await api.playFromQueue(trackIndex);
      setPlayback(state);
    } catch (e) {
      console.error("playAlbumTrack error:", e);
    } finally {
      setLoadingIdx(null);
    }
  };

  const playAll = () => handlePlayAlbumTrack(0);

  return (
    <div className="p-8">
      {/* Header */}
      <div className="flex gap-6 mb-8">
        <motion.div
          initial={{ opacity: 0, scale: 0.9 }}
          animate={{ opacity: 1, scale: 1 }}
          className="w-56 h-56 rounded-xl overflow-hidden shadow-2xl flex-shrink-0 scan-overlay"
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
              disabled={loadingIdx !== null}
              className="flex items-center gap-2 px-6 py-2.5 rounded-full text-sm font-semibold text-qs-accent border border-qs-accent/40 bg-qs-accent/10 hover:bg-qs-accent/20 hover:shadow-neon-cyan transition disabled:opacity-60 disabled:cursor-not-allowed"
            >
              {loadingIdx === 0 ? (
                <motion.div
                  animate={{ rotate: 360 }}
                  transition={{ duration: 1, repeat: Infinity, ease: "linear" }}
                  className="w-4 h-4 border-2 border-white border-t-transparent rounded-full"
                />
              ) : (
                <Play className="w-4 h-4" />
              )}
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
        {album.tracks?.items?.map((track, idx) => (
          <TrackRow
            key={track.id}
            track={track}
            index={idx + 1}
            showNumber
            albumCover={coverUrl}
            albumTitle={album.title}
            isLoading={loadingIdx === idx}
            onPlayInAlbum={() => handlePlayAlbumTrack(idx)}
          />
        ))}
      </div>
    </div>
  );
}
