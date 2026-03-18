import { motion } from "framer-motion";
import { Play } from "lucide-react";
import type { QobuzAlbumSimple } from "../../types";

interface AlbumCardProps {
  album: QobuzAlbumSimple;
  onClick: () => void;
}

export default function AlbumCard({ album, onClick }: AlbumCardProps) {
  const coverUrl = album.image?.large || album.image?.thumbnail;
  const isHires =
    (album.maximum_bit_depth && album.maximum_bit_depth > 16) ||
    (album.maximum_sampling_rate && album.maximum_sampling_rate > 44.1);

  return (
    <button
      onClick={onClick}
      className="group text-left p-3 rounded-xl hover:bg-qs-accent/5 transition-all w-full"
    >
      <div className="relative mb-3">
        <motion.div
          whileHover={{ scale: 1.05 }}
          whileTap={{ scale: 0.97 }}
          transition={{ type: "spring", stiffness: 300, damping: 20 }}
          className="aspect-square rounded-lg overflow-hidden shadow-lg bg-qs-surface scan-overlay"
        >
          {coverUrl ? (
            <img
              src={coverUrl}
              alt={album.title}
              className="w-full h-full object-cover"
              loading="lazy"
            />
          ) : (
            <div className="w-full h-full flex items-center justify-center text-4xl">
              💿
            </div>
          )}
          {/* Play button overlay */}
          <div className="absolute bottom-2 right-2 w-10 h-10 rounded-full btn-play-cyber flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity">
            <Play className="w-4 h-4 text-qs-accent ml-0.5" />
          </div>
          {/* Hi-Res badge */}
          {isHires && (
            <span className="absolute top-2 left-2 quality-hires text-[8px]">
              Hi-Res
            </span>
          )}
        </motion.div>
      </div>
      <p className="text-sm font-medium text-qs-text truncate">{album.title}</p>
      <p className="text-xs text-qs-text-dim truncate mt-0.5">
        {album.artist?.name}
        {album.release_date_original &&
          ` • ${album.release_date_original.split("-")[0]}`}
      </p>
    </button>
  );
}
