import { useEffect, useState } from "react";
import { motion } from "framer-motion";
import { BookMarked } from "lucide-react";
import { useStore } from "../../store";
import * as api from "../../api";
import TrackRow from "../cards/TrackRow";
import AlbumCard from "../cards/AlbumCard";
import PlaylistCard from "../cards/PlaylistCard";

export default function FavoritesView() {
  const { favorites, setFavorites, setView, setViewParam, setAlbumDetail, setPlaylistDetail } =
    useStore();
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [tab, setTab] = useState<"tracks" | "albums" | "artists" | "playlists">("tracks");

  useEffect(() => {
    loadFavorites();
  }, []);

  const loadFavorites = async () => {
    setLoading(true);
    setError(null);
    try {
      const fav = await api.getFavorites();
      setFavorites(fav);
    } catch (e) {
      console.error("getFavorites failed:", e);
      setError(String(e));
    }
    setLoading(false);
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <motion.div
          animate={{ rotate: 360 }}
          transition={{ duration: 2, repeat: Infinity, ease: "linear" }}
          className="w-8 h-8 border-2 border-qs-accent border-t-transparent rounded-full"
        />
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex flex-col items-center justify-center h-full gap-4">
        <p className="text-red-400 font-medium">Failed to load library</p>
        <p className="text-qs-text-dim text-sm max-w-md text-center">{error}</p>
        <button
          onClick={loadFavorites}
          className="px-4 py-2 bg-qs-accent rounded-lg text-sm font-medium hover:opacity-80 transition"
        >
          Retry
        </button>
      </div>
    );
  }

  return (
    <div className="p-8">
      <div className="flex items-center gap-3 mb-6">
        <BookMarked className="w-6 h-6 text-qs-accent" />
        <h1 className="text-2xl font-bold text-white">Bibliothèque</h1>
      </div>

      {/* Onglets */}
      <div className="flex gap-1 mb-6 bg-white/5 rounded-xl p-1 w-fit">
        {(["tracks", "albums", "artists", "playlists"] as const).map((t) => {
          const label = { tracks: "Titres", albums: "Albums", artists: "Artistes", playlists: "Playlists" }[t];
          return (
            <button
              key={t}
              onClick={() => setTab(t)}
              className={`px-4 py-2 rounded-lg text-sm font-medium transition ${
                tab === t ? "bg-white/10 text-white" : "text-qs-text-dim hover:text-white"
              }`}
            >
              {label}
            </button>
          );
        })}
      </div>

      {/* Content */}
      {tab === "tracks" && (
        <div className="space-y-1">
          {favorites?.tracks?.items?.map((track) => (
            <TrackRow key={track.id} track={track} showFavorite />
          )) || (
            <p className="text-qs-text-dim">No favorite tracks yet</p>
          )}
        </div>
      )}

      {tab === "albums" && (
        <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-4">
          {favorites?.albums?.items?.map((album) => (
            <AlbumCard
              key={album.id}
              album={album}
              onClick={async () => {
                const a = await api.getAlbum(album.id);
                setAlbumDetail(a);
                setViewParam(album.id);
                setView("album");
              }}
            />
          )) || (
            <p className="text-qs-text-dim">No favorite albums yet</p>
          )}
        </div>
      )}

      {tab === "artists" && (
        <div className="grid grid-cols-3 md:grid-cols-4 lg:grid-cols-6 gap-4">
          {favorites?.artists?.items?.map((artist) => (
            <button
              key={artist.id}
              onClick={async () => {
                const a = await api.getArtist(artist.id);
                useStore.getState().setArtistDetail(a);
                setViewParam(artist.id.toString());
                setView("artist");
              }}
              className="flex flex-col items-center gap-2 p-4 rounded-xl hover:bg-white/5 transition"
            >
              <div className="w-20 h-20 rounded-full overflow-hidden bg-qs-surface">
                {artist.image?.large ? (
                  <img src={artist.image.large} alt={artist.name} className="w-full h-full object-cover" />
                ) : (
                  <div className="w-full h-full flex items-center justify-center text-2xl">🎤</div>
                )}
              </div>
              <p className="text-sm font-medium text-white truncate w-full text-center">{artist.name}</p>
            </button>
          )) || (
            <p className="text-qs-text-dim">No favorite artists yet</p>
          )}
        </div>
      )}

      {tab === "playlists" && (
        <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-4">
          {favorites?.playlists?.items?.length ? (
            favorites.playlists.items.map((playlist) => (
              <PlaylistCard
                key={playlist.id}
                playlist={playlist}
                onClick={async () => {
                  const p = await api.getPlaylist(playlist.id);
                  setPlaylistDetail(p);
                  setViewParam(playlist.id.toString());
                  setView("playlist");
                }}
              />
            ))
          ) : (
            <p className="text-qs-text-dim col-span-full">No playlists yet</p>
          )}
        </div>
      )}
    </div>
  );
}
