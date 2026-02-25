import { useEffect, useState } from "react";
import { motion } from "framer-motion";
import { TrendingUp, Star, Flame } from "lucide-react";
import { useStore } from "../../store";
import * as api from "../../api";
import type { QobuzAlbumSimple, QobuzPlaylist, QobuzTrack } from "../../types";
import AlbumCard from "../cards/AlbumCard";
import PlaylistCard from "../cards/PlaylistCard";
import TrackRow from "../cards/TrackRow";

export default function HomeView() {
  const { setView, setViewParam, setAlbumDetail, session } = useStore();
  const [featuredAlbums, setFeaturedAlbums] = useState<QobuzAlbumSimple[]>([]);
  const [playlists, setPlaylists] = useState<QobuzPlaylist[]>([]);
  const [trendingTracks, setTrendingTracks] = useState<QobuzTrack[]>([]);
  const [loading, setLoading] = useState(true);
  const [trendingLoading, setTrendingLoading] = useState(true);

  useEffect(() => {
    if (!session.logged_in) return;
    loadHome();
    loadTrending();
  }, [session.logged_in]);

  const loadHome = async () => {
    setLoading(true);
    try {
      const [albums, pls] = await Promise.all([
        api.getFeaturedAlbums(),
        api.getFeaturedPlaylists(),
      ]);
      setFeaturedAlbums(albums.items || []);
      setPlaylists(pls.items || []);
    } catch (e) {
      console.error("Failed to load home:", e);
    }
    setLoading(false);
  };

  const loadTrending = async () => {
    setTrendingLoading(true);
    try {
      const tracks = await api.getTrendingTracks();
      setTrendingTracks(tracks);
    } catch (e) {
      console.error("Failed to load trending:", e);
    }
    setTrendingLoading(false);
  };

  const openAlbum = async (albumId: string) => {
    try {
      const album = await api.getAlbum(albumId);
      setAlbumDetail(album);
      setViewParam(albumId);
      setView("album");
    } catch (e) {
      console.error(e);
    }
  };

  const openPlaylist = async (playlistId: number) => {
    try {
      const pl = await api.getPlaylist(playlistId);
      useStore.getState().setPlaylistDetail(pl);
      setViewParam(playlistId.toString());
      setView("playlist");
    } catch (e) {
      console.error(e);
    }
  };

  return (
    <div className="p-8 space-y-10">
      {/* Greeting */}
      <div>
        <motion.h1
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          className="text-3xl font-bold text-white"
        >
          Welcome back{session.user_name ? `, ${session.user_name}` : ""}
        </motion.h1>
        <p className="text-qs-text-dim mt-1">
          {new Date().toLocaleDateString("en-US", { weekday: "long", month: "long", day: "numeric" })}
        </p>
      </div>

      {/* Trending — Last.fm chart ✕ Qobuz catalog */}
      <section>
        <div className="flex items-center gap-2 mb-4">
          <Flame className="w-5 h-5 text-orange-400" />
          <h2 className="text-xl font-semibold text-white">Trending Right Now</h2>
          <span className="text-xs text-qs-text-dim ml-1 px-1.5 py-0.5 rounded bg-white/5">via Last.fm</span>
        </div>
        {trendingLoading ? (
          <div className="flex items-center gap-3 text-qs-text-dim text-sm py-4">
            <motion.div
              animate={{ rotate: 360 }}
              transition={{ duration: 1.5, repeat: Infinity, ease: "linear" }}
              className="w-4 h-4 border-2 border-orange-400/50 border-t-transparent rounded-full flex-shrink-0"
            />
            Cross-referencing Last.fm charts with the Qobuz catalog…
          </div>
        ) : trendingTracks.length > 0 ? (
          <div className="space-y-1">
            {trendingTracks.slice(0, 10).map((track) => (
              <TrackRow key={track.id} track={track} />
            ))}
          </div>
        ) : (
          <p className="text-qs-text-dim text-sm py-2">Could not load trending tracks.</p>
        )}
      </section>

      {/* New Releases */}
      {!loading && featuredAlbums.length > 0 && (
        <section>
          <div className="flex items-center gap-2 mb-4">
            <TrendingUp className="w-5 h-5 text-qs-accent" />
            <h2 className="text-xl font-semibold text-white">New Releases</h2>
          </div>
          <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-4">
            {featuredAlbums.slice(0, 10).map((album) => (
              <AlbumCard
                key={album.id}
                album={album}
                onClick={() => openAlbum(album.id)}
              />
            ))}
          </div>
        </section>
      )}

      {/* Editor's Picks */}
      {!loading && playlists.length > 0 && (
        <section>
          <div className="flex items-center gap-2 mb-4">
            <Star className="w-5 h-5 text-amber-400" />
            <h2 className="text-xl font-semibold text-white">Editor's Picks</h2>
          </div>
          <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-4">
            {playlists.slice(0, 10).map((pl) => (
              <PlaylistCard
                key={pl.id}
                playlist={pl}
                onClick={() => openPlaylist(pl.id)}
              />
            ))}
          </div>
        </section>
      )}

      {loading && (
        <div className="flex justify-center py-8">
          <motion.div
            animate={{ rotate: 360 }}
            transition={{ duration: 2, repeat: Infinity, ease: "linear" }}
            className="w-6 h-6 border-2 border-qs-accent border-t-transparent rounded-full"
          />
        </div>
      )}
    </div>
  );
}
