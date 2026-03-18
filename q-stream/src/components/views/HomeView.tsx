import { useEffect, useState } from "react";
import { motion } from "framer-motion";
import { TrendingUp, Star, Flame, Clock, Play, ListMusic, Compass, Music2, Sparkles, X } from "lucide-react";
import { useStore } from "../../store";
import * as api from "../../api";
import type { QobuzAlbumSimple, QobuzPlaylist, QobuzTrack, UnifiedTrack } from "../../types";
import AlbumCard from "../cards/AlbumCard";
import PlaylistCard from "../cards/PlaylistCard";
import TrackRow from "../cards/TrackRow";

// ── Titre de section editorial ───────────────────────────────────────────────
function SectionHeading({
  icon: Icon,
  title,
  badge,
  iconColor = "text-qs-accent",
}: {
  icon: typeof Compass;
  title: string;
  badge?: React.ReactNode;
  iconColor?: string;
}) {
  return (
    <div className="flex items-center justify-between mb-5">
      <div className="flex items-center gap-2.5">
        <Icon className={`w-4 h-4 ${iconColor} opacity-80 flex-shrink-0`} />
        <h2 className="font-condensed text-xs font-semibold text-qs-text-dim uppercase tracking-[0.18em]">
          {title}
        </h2>
        {badge && badge}
      </div>
    </div>
  );
}

// ── Carte compacte "Récemment écouté" ────────────────────────────────────────
function RecentCard({ track, onPlay }: { track: UnifiedTrack; onPlay: () => void }) {
  return (
    <button
      onClick={onPlay}
      className="flex items-center gap-3 p-2 rounded-xl text-left w-full group relative overflow-hidden
                 bg-qs-surface/60 hover:bg-qs-accent/5 border border-qs-text/5 hover:border-qs-accent/15
                 transition-colors duration-150"
    >
      <div className="w-11 h-11 rounded-lg overflow-hidden flex-shrink-0 bg-qs-surface-light">
        {track.cover_url ? (
          <img src={track.cover_url} alt="" className="w-full h-full object-cover" />
        ) : (
          <div className="w-full h-full flex items-center justify-center text-xl">🎵</div>
        )}
      </div>
      <div className="min-w-0 flex-1">
        <p className="text-sm font-medium text-qs-text truncate leading-snug">{track.title}</p>
        <p className="text-xs text-qs-text-dim truncate mt-0.5">{track.artist}</p>
      </div>
      <Play className="w-3.5 h-3.5 text-qs-accent opacity-0 group-hover:opacity-100 flex-shrink-0 mr-1 transition-opacity" />
    </button>
  );
}

// ── Album card with dismiss button ───────────────────────────────────────────
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
                   hover:bg-qs-red/70 transition text-qs-text/70 hover:text-white"
      >
        <X className="w-3 h-3" />
      </button>
    </div>
  );
}

// ── Spinner ───────────────────────────────────────────────────────────────────
function Spinner({ cls }: { cls?: string }) {
  return (
    <motion.div
      animate={{ rotate: 360 }}
      transition={{ duration: 1.5, repeat: Infinity, ease: "linear" }}
      className={`w-4 h-4 border-2 border-t-transparent rounded-full flex-shrink-0 ${cls ?? "border-qs-accent/50"}`}
    />
  );
}

export default function HomeView() {
  const {
    setView, setViewParam, setAlbumDetail, setPlaylistDetail,
    session, lastfmUser, recentlyPlayed, setPlayback,
    dismissedAlbums, dismissAlbum,
  } = useStore();

  const [recoAlbums, setRecoAlbums] = useState<QobuzAlbumSimple[]>([]);
  const [recoLoading, setRecoLoading] = useState(true);
  const [userPlaylists, setUserPlaylists] = useState<QobuzPlaylist[]>([]);
  const [trendingTracks, setTrendingTracks] = useState<QobuzTrack[]>([]);
  const [trendingLoading, setTrendingLoading] = useState(true);
  const [featuredAlbums, setFeaturedAlbums] = useState<QobuzAlbumSimple[]>([]);
  const [editorialPlaylists, setEditorialPlaylists] = useState<QobuzPlaylist[]>([]);
  const [editorialLoading, setEditorialLoading] = useState(true);
  const [knownArtistAlbums, setKnownArtistAlbums] = useState<QobuzAlbumSimple[]>([]);
  const [knownArtistLoading, setKnownArtistLoading] = useState(true);
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
    api.getRecentPlaybackRecs(lastfmUser.user_name)
      .then((albums) => setRecoAlbums((prev) => {
        const ids = new Set(prev.map((a) => a.id));
        return [...prev, ...albums.filter((a) => !ids.has(a.id))].slice(0, 12);
      }))
      .catch(() => {});
    loadGenreExploration(lastfmUser.user_name);
  }, [session.logged_in, lastfmUser?.user_name]);

  const loadReco = async () => {
    setRecoLoading(true);
    try { setRecoAlbums((await api.getLibraryDiscovery()).slice(0, 12)); } catch {}
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

  const visibleReco = recoAlbums.filter((a) => !dismissedAlbums.includes(a.id));
  const visibleKnownArtist = knownArtistAlbums.filter((a) => !dismissedAlbums.includes(a.id));
  const visibleGenre = genreAlbums.filter((a) => !dismissedAlbums.includes(a.id));

  const hour = new Date().getHours();
  const greeting = hour < 12 ? "Bonjour" : hour < 18 ? "Bon après-midi" : "Bonsoir";

  return (
    <div className="p-7 space-y-9 pb-12">
      {/* ── En-tête ── */}
      <motion.div
        initial={{ opacity: 0, y: 12 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.3 }}
      >
        <h1 className="font-sans text-2xl font-semibold text-qs-text tracking-tight">
          {greeting}{session.user_name ? <>, <span className="text-qs-accent font-medium">{session.user_name}</span></> : ""}
        </h1>
        <p className="font-condensed text-[10px] font-medium text-qs-text-dim uppercase tracking-[0.18em] mt-1.5">
          {new Date().toLocaleDateString("fr-FR", { weekday: "long", month: "long", day: "numeric" })}
        </p>
      </motion.div>

      {/* ── TOP : Récemment écouté ── */}
      {recentlyPlayed.length > 0 && (
        <section>
          <SectionHeading icon={Clock} title="Récemment écouté" />
          <div className="grid grid-cols-2 md:grid-cols-3 gap-2">
            {recentlyPlayed.map((track) => (
              <RecentCard key={track.id} track={track} onPlay={() => playRecentItem(track)} />
            ))}
          </div>
        </section>
      )}

      {/* ── Recommandations ── */}
      <section>
        <SectionHeading
          icon={Compass}
          title="Recommandations"
          badge={
            lastfmUser && (
              <span className="text-[9px] text-qs-text-dim px-1.5 py-0.5 rounded bg-qs-text/5 tracking-wider uppercase ml-1">
                Last.fm
              </span>
            )
          }
        />
        {recoLoading ? (
          <div className="flex items-center gap-3 text-qs-text-dim text-sm py-3">
            <Spinner />
            Chargement…
          </div>
        ) : visibleReco.length > 0 ? (
          <div className="grid grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-3">
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
          <SectionHeading
            icon={ListMusic}
            title="Tes playlists"
            iconColor="text-qs-accent-2"
          />
          <div className="grid grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-3">
            {userPlaylists.map((pl) => (
              <PlaylistCard key={pl.id} playlist={pl} onClick={() => openPlaylist(pl.id)} />
            ))}
          </div>
        </section>
      )}

      {/* ── Albums d'artistes favoris ── */}
      {!knownArtistLoading && visibleKnownArtist.length > 0 && (
        <section>
          <SectionHeading
            icon={Music2}
            title="Artistes favoris"
            badge={
              <span className="text-[9px] text-qs-text-dim px-1.5 py-0.5 rounded bg-qs-text/5 tracking-wider uppercase ml-1">
                à découvrir
              </span>
            }
          />
          <div className="grid grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-3">
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
          <SectionHeading
            icon={Sparkles}
            title="Explorer tes genres"
            iconColor="text-qs-accent-2"
            badge={
              <span className="text-[9px] text-qs-text-dim px-1.5 py-0.5 rounded bg-qs-text/5 tracking-wider uppercase ml-1">
                MusicBrainz
              </span>
            }
          />
          <div className="grid grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-3">
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
        <SectionHeading
          icon={Flame}
          title="Tendances"
          iconColor="text-qs-accent-2"
          badge={
            <span className="text-[9px] text-qs-text-dim px-1.5 py-0.5 rounded bg-qs-text/5 tracking-wider uppercase ml-1">
              Last.fm
            </span>
          }
        />
        {trendingLoading ? (
          <div className="flex items-center gap-3 text-qs-text-dim text-sm py-4">
            <Spinner cls="border-qs-accent-2/50" />
            Chargement des tendances…
          </div>
        ) : trendingTracks.length > 0 ? (
          <div className="space-y-0.5">
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
          <SectionHeading icon={TrendingUp} title="Nouvelles sorties" />
          <div className="grid grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-3">
            {featuredAlbums.slice(0, 12).map((album) => (
              <AlbumCard key={album.id} album={album} onClick={() => openAlbum(album.id)} />
            ))}
          </div>
        </section>
      )}

      {/* ── Sélection éditoriale ── */}
      {!editorialLoading && editorialPlaylists.length > 0 && (
        <section>
          <SectionHeading
            icon={Star}
            title="Sélection éditoriale"
            iconColor="text-qs-accent-2"
          />
          <div className="grid grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-3">
            {editorialPlaylists.slice(0, 12).map((pl) => (
              <PlaylistCard key={pl.id} playlist={pl} onClick={() => openPlaylist(pl.id)} />
            ))}
          </div>
        </section>
      )}

      {editorialLoading && (
        <div className="flex justify-center py-8">
          <Spinner />
        </div>
      )}
    </div>
  );
}
