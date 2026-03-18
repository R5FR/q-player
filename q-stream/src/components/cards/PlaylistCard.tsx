import { motion } from "framer-motion";
import { Play } from "lucide-react";
import type { QobuzPlaylist } from "../../types";

interface PlaylistCardProps {
  playlist: QobuzPlaylist;
  onClick: () => void;
}

export default function PlaylistCard({ playlist, onClick }: PlaylistCardProps) {
  const coverUrl = playlist.images300?.[0] || playlist.image_rectangle_mini?.[0];

  return (
    <button
      onClick={onClick}
      className="group text-left p-3 rounded-xl hover:bg-white/5 transition-all w-full"
    >
      <div className="relative mb-3">
        <motion.div
          whileHover={{ scale: 1.05 }}
          whileTap={{ scale: 0.97 }}
          transition={{ type: "spring", stiffness: 300, damping: 20 }}
          className="aspect-square rounded-lg overflow-hidden shadow-lg bg-qs-surface"
        >
          {coverUrl ? (
            <img
              src={coverUrl}
              alt={playlist.name}
              className="w-full h-full object-cover"
              loading="lazy"
            />
          ) : (
            <div className="w-full h-full flex items-center justify-center text-4xl">
              🎶
            </div>
          )}
          <div className="absolute bottom-2 right-2 w-10 h-10 rounded-full btn-play-cyber flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity">
            <Play className="w-4 h-4 text-qs-accent ml-0.5" />
          </div>
        </motion.div>
      </div>
      <p className="text-sm font-medium text-qs-text truncate">{playlist.name}</p>
      <p className="text-xs text-qs-text-dim truncate mt-0.5">
        {playlist.owner?.name || "Qobuz"}
        {playlist.tracks_count ? ` • ${playlist.tracks_count} tracks` : ""}
      </p>
    </button>
  );
}
