import { useEffect, useState } from "react";
import { motion } from "framer-motion";
import { TrendingUp, Star, Flame, Clock, Play, ListMusic, Compass, Music2, Sparkles, X } from "lucide-react";
import { useStore } from "../../store";
import * as api from "../../api";
import type { QobuzAlbumSimple, QobuzPlaylist, QobuzTrack, UnifiedTrack } from "../../types";
import AlbumCard from "../cards/AlbumCard";
import PlaylistCard from "../cards/PlaylistCard";
import TrackRow from "../cards/TrackRow";

// ── Carte compacte "Récemment écouté" (style Spotify pill) ──────────────────
function RecentCard({ track, onPlay }: { track: UnifiedTrack; onPlay: () => void }) {
  return (
    <motion.button
      whileHover={{ backgroundColor: "rgba(0,212,255,0.06)" }}
      whileTap={{ scale: 0.97 }}
      onClick={onPlay}
      className="flex items-center gap-3 p-2 rounded-lg text-left w-full glass-light group relative overflow-hidden"
    >
      <div className="w-12 h-12 rounded-md overflow-hidden flex-shrink-0 bg-qs-surface">
        {track.cover_url ? (
          <img src={track.cover_url} alt="" className="w-full h-full object-cover" />
        ) : (
          <div className="w-full h-full flex items-center justify-center text-xl">🎵</div>
        )}
      </div>
      <div className="min-w-0 flex-1">
        <p className="text-sm font-medium text-white truncate leading-snug">{track.title}</p>
        <p className="text-xs text-qs-text-dim truncate">{track.artist}</p>
      </div>
      <Play className="w-4 h-4 text-qs-accent opacity-0 group-hover:opacity-100 flex-shrink-0 mr-1 transition" />
    </motion.button>
  );
}

// ── Album card with dismiss button ──────────────────────────────────────────
function DismissableAlbumCard({
  album,
  onClick,
  onDismiss,
}: {
  album: QobuzAlbumSimple;
  onClick: () => void;
  onDismiss: () => void;
}) {
  return (
    <div className="relative group/wrap">
      <AlbumCard album={album} onClick={onClick} />
      <button
        onClick={(e) => { e.stopPropagation(); onDismiss(); }}
        title="Pas intéressé"
        className="absolute top-1.5 right-1.5 z-20 w-5 h-5 rounded-full bg-black/60 backdrop-blur-sm
                   flex items-center justify-center opacity-0 group-hover/wrap:opacity-100
                   hover:bg-red-500/70 transition text-white/70 hover:text-white"
      >
        <X className="w-3 h-3" />
      </button>
    </div>
  );
}

export default function HomeView() {
  const {
    setView, setViewParam, setAlbumDetail, setPlaylistDetail,
    session, lastfmUser, recentlyPlayed, setPlayback,
    dismissedAlbums, dismissAlbum,
  } = useStore();

  // Recommandations (discovery + last.fm)
  const [recoAlbums, setRecoAlbums] = useState<QobuzAlbumSimple[]>([]);
  const [recoLoading, setRecoLoading] = useState(true);

  // Tes playlists
  const [userPlaylists, setUserPlaylists] = useState<QobuzPlaylist[]>([]);

  // Tendances
  const [trendingTracks, setTrendingTracks] = useState<QobuzTrack[]>([]);
  const [trendingLoading, setTrendingLoading] = useState(true);

  // Nouvelles sorties + éditorial
  const [featuredAlbums, setFeaturedAlbums] = useState<QobuzAlbumSimple[]>([]);
  const [editorialPlaylists, setEditorialPlaylists] = useState<QobuzPlaylist[]>([]);
  const [editorialLoading, setEditorialLoading] = useState(true);

  // Albums d'artistes favoris (inconnus)
  const [knownArtistAlbums, setKnownArtistAlbums] = useState<QobuzAlbumSimple[]>([]);
  const [knownArtistLoading, setKnownArtistLoading] = useState(true);

  // Exploration par genre
  const [genreAlbums, setGenreAlbums] = useState<QobuzAlbumSimple[]>([]);
  const [genreLoading, setGenreLoading] = useState(true);

  useEffect(() => {
    if (!session.logged_in) return;
    loadReco();
    loadTrending();
    loadEditorial();
    loadKnownArtistAlbums();
    api.getUserPlaylists().then(setUserPlaylists).catch(() => {});
  }, [session.logged_in]);

  useEffect(() => {
    if (!session.logged_in || !lastfmUser) return;
    // Layer Last.fm recent-playback recs on top of library discovery
    api.getRecentPlaybackRecs(lastfmUser.user_name)
      .then((albums) => setRecoAlbums((prev) => {
        const ids = new Set(prev.map((a) => a.id));
        return [...prev, ...albums.filter((a) => !ids.has(a.id))].slice(0, 12);
      }))
      .catch(() => {});
    // Genre exploration requires Last.fm
    loadGenreExploration(lastfmUser.user_name);
  }, [session.logged_in, lastfmUser?.user_name]);

  const loadReco = async () => {
    setRecoLoading(true);
    try {
      const albums = await api.getLibraryDiscovery();
      setRecoAlbums(albums.slice(0, 12));
    } catch {}
    setRecoLoading(false);
  };

  const loadTrending = async () => {
    setTrendingLoading(true);
    try { setTrendingTracks(await api.getTrendingTracks()); } catch {}
    setTrendingLoading(false);
  };

  const loadEditorial = async () => {
    setEditorialLoading(true);
    try {
      const [albums, pls] = await Promise.all([api.getFeaturedAlbums(), api.getFeaturedPlaylists()]);
      setFeaturedAlbums(albums.items || []);
      setEditorialPlaylists(pls.items || []);
    } catch {}
    setEditorialLoading(false);
  };

  const loadKnownArtistAlbums = async () => {
    setKnownArtistLoading(true);
    try { setKnownArtistAlbums(await api.getUnknownAlbumsByKnownArtists()); } catch {}
    setKnownArtistLoading(false);
  };

  const loadGenreExploration = async (username: string) => {
    setGenreLoading(true);
    try { setGenreAlbums(await api.getGenreExploration(username)); } catch {}
    setGenreLoading(false);
  };

  const openAlbum = async (albumId: string) => {
    const album = await api.getAlbum(albumId);
    setAlbumDetail(album);
    setViewParam(albumId);
    setView("album");
  };

  const openPlaylist = async (playlistId: number) => {
    const pl = await api.getPlaylist(playlistId);
    setPlaylistDetail(pl);
    setViewParam(playlistId.toString());
    setView("playlist");
  };

  const playRecentItem = async (track: UnifiedTrack) => {
    try {
      const state = "Qobuz" in track.source
        ? await api.playTrack(track.source.Qobuz.track_id)
        : await api.playLocalTrack(track.source.Local.file_path);
      setPlayback(state);
    } catch (e) { console.error(e); }
  };

  const Spinner = ({ cls }: { cls: string }) => (
    <motion.div
      animate={{ rotate: 360 }}
      transition={{ duration: 1.5, repeat: Infinity, ease: "linear" }}
      className={`w-4 h-4 border-2 ${cls} border-t-transparent rounded-full flex-shrink-0`}
    />
  );

  // Filter dismissed albums from all recommendation lists
  const visibleReco = recoAlbums.filter((a) => !dismissedAlbums.includes(a.id));
  const visibleKnownArtist = knownArtistAlbums.filter((a) => !dismissedAlbums.includes(a.id));
  const visibleGenre = genreAlbums.filter((a) => !dismissedAlbums.includes(a.id));

  return (
    <div className="p-8 space-y-10">
      {/* ── En-tête ── */}
      <div>
        <motion.h1
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          className="text-3xl font-bold text-white"
        >
          Bonjour{session.user_name ? `, ${session.user_name}` : ""}
        </motion.h1>
        <p className="text-qs-text-dim mt-1 text-sm">
          {new Date().toLocaleDateString("fr-FR", { weekday: "long", month: "long", day: "numeric" })}
        </p>
      </div>

      {/* ── TOP : Récemment écouté (pills) ── */}
      <section>
        <div className="flex items-center gap-2 mb-3">
          <Clock className="w-4 h-4 text-qs-text-dim" />
          <h2 className="text-base font-semibold text-white">Récemment écouté</h2>
        </div>
        {recentlyPlayed.length === 0 ? (
          <p className="text-sm text-qs-text-dim py-2">
            Lance une écoute pour retrouver tes morceaux ici.
          </p>
        ) : (
          <div className="grid grid-cols-2 md:grid-cols-3 gap-2">
            {recentlyPlayed.map((track) => (
              <RecentCard key={track.id} track={track} onPlay={() => playRecentItem(track)} />
            ))}
          </div>
        )}
      </section>

      {/* ── Recommandations ── */}
      <section>
        <div className="flex items-center gap-2 mb-4">
          <Compass className="w-5 h-5 text-qs-accent" />
          <h2 className="text-xl font-semibold text-white">Recommandations</h2>
          {lastfmUser && (
            <span className="text-[10px] text-qs-text-dim px-1.5 py-0.5 rounded bg-white/5">
              via Last.fm
            </span>
          )}
        </div>
        {recoLoading ? (
          <div className="flex items-center gap-3 text-qs-text-dim text-sm py-3">
            <Spinner cls="border-qs-accent/50" />
            Chargement…
          </div>
        ) : visibleReco.length > 0 ? (
          <div className="grid grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4">
            {visibleReco.slice(0, 12).map((album) => (
              <DismissableAlbumCard
                key={album.id}
                album={album}
                onClick={() => openAlbum(album.id)}
                onDismiss={() => dismissAlbum(album.id)}
              />
            ))}
          </div>
        ) : (
          <p className="text-sm text-qs-text-dim py-3">
            Ajoute des favoris pour obtenir des recommandations.
          </p>
        )}
      </section>

      {/* ── Tes playlists ── */}
      {userPlaylists.length > 0 && (
        <section>
          <div className="flex items-center gap-2 mb-4">
            <ListMusic className="w-5 h-5 text-qs-accent-2" />
            <h2 className="text-xl font-semibold text-white">Tes playlists</h2>
          </div>
          <div className="grid grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4">
            {userPlaylists.map((pl) => (
              <PlaylistCard key={pl.id} playlist={pl} onClick={() => openPlaylist(pl.id)} />
            ))}
          </div>
        </section>
      )}

      {/* ── Albums d'artistes favoris ── */}
      {!knownArtistLoading && visibleKnownArtist.length > 0 && (
        <section>
          <div className="flex items-center gap-2 mb-4">
            <Music2 className="w-5 h-5 text-qs-accent" />
            <h2 className="text-xl font-semibold text-white">Artistes favoris</h2>
            <span className="text-xs text-qs-text-dim ml-1 px-1.5 py-0.5 rounded bg-white/5">
              albums à découvrir
            </span>
          </div>
          <div className="grid grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4">
            {visibleKnownArtist.slice(0, 12).map((album) => (
              <DismissableAlbumCard
                key={album.id}
                album={album}
                onClick={() => openAlbum(album.id)}
                onDismiss={() => dismissAlbum(album.id)}
              />
            ))}
          </div>
        </section>
      )}

      {/* ── Exploration par genre ── */}
      {lastfmUser && !genreLoading && visibleGenre.length > 0 && (
        <section>
          <div className="flex items-center gap-2 mb-4">
            <Sparkles className="w-5 h-5 text-qs-accent-2" />
            <h2 className="text-xl font-semibold text-white">Explorer tes genres</h2>
            <span className="text-xs text-qs-text-dim ml-1 px-1.5 py-0.5 rounded bg-white/5">
              via Last.fm + MusicBrainz
            </span>
          </div>
          <div className="grid grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4">
            {visibleGenre.slice(0, 12).map((album) => (
              <DismissableAlbumCard
                key={album.id}
                album={album}
                onClick={() => openAlbum(album.id)}
                onDismiss={() => dismissAlbum(album.id)}
              />
            ))}
          </div>
        </section>
      )}

      {/* ── Tendances ── */}
      <section>
        <div className="flex items-center gap-2 mb-4">
          <Flame className="w-5 h-5 text-orange-400" />
          <h2 className="text-xl font-semibold text-white">Tendances</h2>
          <span className="text-xs text-qs-text-dim ml-1 px-1.5 py-0.5 rounded bg-white/5">via Last.fm</span>
        </div>
        {trendingLoading ? (
          <div className="flex items-center gap-3 text-qs-text-dim text-sm py-4">
            <Spinner cls="border-orange-400/50" />
            Chargement des tendances…
          </div>
        ) : trendingTracks.length > 0 ? (
          <div className="space-y-1">
            {trendingTracks.slice(0, 10).map((track) => (
              <TrackRow key={track.id} track={track} />
            ))}
          </div>
        ) : (
          <p className="text-qs-text-dim text-sm py-2">Impossible de charger les tendances.</p>
        )}
      </section>

      {/* ── Nouvelles sorties ── */}
      {!editorialLoading && featuredAlbums.length > 0 && (
        <section>
          <div className="flex items-center gap-2 mb-4">
            <TrendingUp className="w-5 h-5 text-qs-accent" />
            <h2 className="text-xl font-semibold text-white">Nouvelles sorties</h2>
          </div>
          <div className="grid grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4">
            {featuredAlbums.slice(0, 12).map((album) => (
              <AlbumCard key={album.id} album={album} onClick={() => openAlbum(album.id)} />
            ))}
          </div>
        </section>
      )}

      {/* ── Sélection éditoriale ── */}
      {!editorialLoading && editorialPlaylists.length > 0 && (
        <section>
          <div className="flex items-center gap-2 mb-4">
            <Star className="w-5 h-5 text-amber-400" />
            <h2 className="text-xl font-semibold text-white">Sélection éditoriale</h2>
          </div>
          <div className="grid grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4">
            {editorialPlaylists.slice(0, 12).map((pl) => (
              <PlaylistCard key={pl.id} playlist={pl} onClick={() => openPlaylist(pl.id)} />
            ))}
          </div>
        </section>
      )}

      {editorialLoading && (
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
