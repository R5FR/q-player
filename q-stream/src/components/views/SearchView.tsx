import { useState, useCallback, useEffect, useRef } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { Search as SearchIcon, X, Disc3, Mic2, Music, ListMusic } from "lucide-react";
import { useStore } from "../../store";
import * as api from "../../api";
import type { SearchHistoryEntry } from "../../types";
import AlbumCard from "../cards/AlbumCard";
import PlaylistCard from "../cards/PlaylistCard";
import TrackRow from "../cards/TrackRow";

// ── Types ──────────────────────────────────────────────────────────────────
type SearchTab = "all" | "tracks" | "albums" | "artists" | "playlists";

const TABS: { id: SearchTab; label: string }[] = [
  { id: "all",       label: "Tout"      },
  { id: "tracks",    label: "Titres"    },
  { id: "albums",    label: "Albums"    },
  { id: "artists",   label: "Artistes"  },
  { id: "playlists", label: "Playlists" },
];

// ── Helpers ────────────────────────────────────────────────────────────────
const EntryTypeIcon = ({ type }: { type: string }) => {
  switch (type) {
    case "album":    return <Disc3 className="w-4 h-4" />;
    case "artist":   return <Mic2 className="w-4 h-4" />;
    case "track":    return <Music className="w-4 h-4" />;
    case "playlist": return <ListMusic className="w-4 h-4" />;
    default:         return <SearchIcon className="w-4 h-4" />;
  }
};

// ── Spinner ────────────────────────────────────────────────────────────────
function Spinner() {
  return (
    <motion.div
      animate={{ rotate: 360 }}
      transition={{ duration: 1.5, repeat: Infinity, ease: "linear" }}
      className="w-5 h-5 border-2 border-qs-accent border-t-transparent rounded-full"
    />
  );
}

// ── ArtistCard (inline, pas besoin de fichier séparé) ─────────────────────
function ArtistCard({ artist, onClick }: { artist: any; onClick: () => void }) {
  return (
    <button
      onClick={onClick}
      className="flex flex-col items-center gap-2.5 p-3 rounded-xl hover:bg-qs-accent/5 border border-transparent hover:border-qs-accent/15 transition-all w-full group"
    >
      <div className="w-full aspect-square rounded-full overflow-hidden bg-qs-surface relative">
        {artist.image?.large ? (
          <img src={artist.image.large} alt={artist.name} className="w-full h-full object-cover" />
        ) : (
          <div className="w-full h-full flex items-center justify-center text-3xl">🎤</div>
        )}
      </div>
      <p className="font-sans text-sm font-medium text-qs-text truncate w-full text-center">
        {artist.name}
      </p>
    </button>
  );
}

// ── Composant principal ────────────────────────────────────────────────────
export default function SearchView() {
  const {
    searchQuery, searchResults, setSearchResults,
    setView, setViewParam, setAlbumDetail, setArtistDetail, setPlaylistDetail,
    searchHistory, addSearchHistoryEntry, removeSearchHistoryEntry, setPlayback,
  } = useStore();

  const [loading, setLoading]     = useState(false);
  const [activeTab, setActiveTab] = useState<SearchTab>("all");
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const doSearch = useCallback(async (query: string) => {
    if (!query.trim()) { setSearchResults(null); return; }
    setLoading(true);
    try { setSearchResults(await api.search(query)); } catch (e) { console.error(e); }
    setLoading(false);
  }, [setSearchResults]);

  useEffect(() => {
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => doSearch(searchQuery), 350);
    return () => { if (debounceRef.current) clearTimeout(debounceRef.current); };
  }, [searchQuery, doSearch]);

  // ── Counts per tab ────────────────────────────────────────────────────
  const counts: Record<SearchTab, number> = {
    all:       0,
    tracks:    searchResults?.tracks?.items?.length    ?? 0,
    albums:    searchResults?.albums?.items?.length    ?? 0,
    artists:   searchResults?.artists?.items?.length   ?? 0,
    playlists: searchResults?.playlists?.items?.length ?? 0,
  };

  // ── Navigation helpers ────────────────────────────────────────────────
  const openAlbum = async (albumId: string, title: string, artist: string, coverUrl?: string) => {
    const album = await api.getAlbum(albumId);
    setAlbumDetail(album); setViewParam(albumId); setView("album");
    addSearchHistoryEntry({ id: albumId, title, subtitle: artist, cover_url: coverUrl, entry_type: "album" });
  };

  const openArtist = async (artistId: number, name: string, coverUrl?: string) => {
    const a = await api.getArtist(artistId);
    setArtistDetail(a); setViewParam(artistId.toString()); setView("artist");
    addSearchHistoryEntry({ id: artistId.toString(), title: name, subtitle: "Artiste", cover_url: coverUrl, entry_type: "artist" });
  };

  const openPlaylist = async (playlistId: number, name: string, coverUrl?: string) => {
    const pl = await api.getPlaylist(playlistId);
    setPlaylistDetail(pl); setViewParam(playlistId.toString()); setView("playlist");
    addSearchHistoryEntry({ id: playlistId.toString(), title: name, subtitle: "Playlist", cover_url: coverUrl, entry_type: "playlist" });
  };

  const playTrackAndRecord = async (trackId: number, title: string, artist: string, coverUrl?: string) => {
    try {
      const state = await api.playTrack(trackId);
      setPlayback(state);
      addSearchHistoryEntry({ id: trackId.toString(), title, subtitle: artist, cover_url: coverUrl, entry_type: "track" });
    } catch (e) { console.error(e); }
  };

  const openHistoryEntry = async (entry: SearchHistoryEntry) => {
    try {
      switch (entry.entry_type) {
        case "album":    { const a  = await api.getAlbum(entry.id);                  setAlbumDetail(a);    setViewParam(entry.id); setView("album");    break; }
        case "artist":   { const a  = await api.getArtist(parseInt(entry.id));       setArtistDetail(a);   setViewParam(entry.id); setView("artist");   break; }
        case "playlist": { const pl = await api.getPlaylist(parseInt(entry.id));     setPlaylistDetail(pl); setViewParam(entry.id); setView("playlist"); break; }
        case "track":    { const st = await api.playTrack(parseInt(entry.id));       setPlayback(st);                                                     break; }
      }
      addSearchHistoryEntry(entry);
    } catch (e) { console.error(e); }
  };

  const showHistory = !searchQuery.trim() && searchHistory.length > 0;
  const hasResults  = !!searchResults;

  return (
    <div className="flex flex-col h-full">

      {/* ── Onglets de filtre (visibles uniquement quand il y a des résultats) ── */}
      {hasResults && !loading && (
        <div className="flex-shrink-0 flex items-end gap-0 px-7 pt-5 border-b border-qs-text/[0.07]">
          {TABS.map((tab) => {
            const count  = counts[tab.id];
            const active = activeTab === tab.id;
            // Griser les onglets sans résultats (sauf "Tout")
            const empty  = tab.id !== "all" && count === 0;
            return (
              <button
                key={tab.id}
                onClick={() => !empty && setActiveTab(tab.id)}
                disabled={empty}
                className={[
                  "relative pb-3 px-4 font-condensed text-xs font-semibold uppercase tracking-[0.16em] transition-colors duration-150",
                  active
                    ? "text-qs-accent"
                    : empty
                    ? "text-qs-text-dim/40 cursor-default"
                    : "text-qs-text-dim hover:text-qs-text",
                ].join(" ")}
              >
                {tab.label}
                {tab.id !== "all" && count > 0 && (
                  <span className={[
                    "ml-1.5 font-mono text-[9px] px-1 py-0.5 rounded",
                    active
                      ? "text-qs-accent bg-qs-accent/10"
                      : "text-qs-text-dim bg-qs-text/5",
                  ].join(" ")}>
                    {count}
                  </span>
                )}
                {/* Barre active */}
                {active && (
                  <motion.div
                    layoutId="search-tab-indicator"
                    className="absolute bottom-0 left-0 right-0 h-0.5 bg-qs-accent"
                    style={{ boxShadow: "0 0 8px rgb(var(--qs-accent) / 0.6)" }}
                    transition={{ type: "spring", stiffness: 400, damping: 30 }}
                  />
                )}
              </button>
            );
          })}
        </div>
      )}

      {/* ── Corps scrollable ── */}
      <div className="flex-1 overflow-y-auto">
        <div className="p-7">

          {/* ── Historique ── */}
          {showHistory && !loading && (
            <div className="mb-8">
              <p className="font-condensed text-[10px] font-semibold text-qs-text-dim uppercase tracking-[0.18em] mb-4">
                Recherches récentes
              </p>
              <div className="space-y-0.5">
                {searchHistory.map((entry) => (
                  <div
                    key={entry.id + entry.entry_type}
                    onClick={() => openHistoryEntry(entry)}
                    className="flex items-center gap-3 p-2 rounded-lg group cursor-pointer hover:bg-qs-accent/5 border border-transparent hover:border-qs-accent/10 transition-colors"
                  >
                    <div className="w-10 h-10 rounded-md overflow-hidden flex-shrink-0 bg-qs-surface flex items-center justify-center text-qs-text-dim">
                      {entry.cover_url
                        ? <img src={entry.cover_url} alt="" className="w-full h-full object-cover" />
                        : <EntryTypeIcon type={entry.entry_type} />
                      }
                    </div>
                    <div className="flex-1 min-w-0">
                      <p className="font-sans text-sm font-medium text-qs-text truncate">{entry.title}</p>
                      <p className="font-sans text-xs text-qs-text-dim truncate">{entry.subtitle}</p>
                    </div>
                    <span className="font-condensed text-[9px] uppercase tracking-wider text-qs-text-dim px-1.5 py-0.5 rounded bg-qs-text/5 flex-shrink-0">
                      {entry.entry_type}
                    </span>
                    <button
                      onClick={(e) => { e.stopPropagation(); removeSearchHistoryEntry(entry.id); }}
                      className="opacity-0 group-hover:opacity-100 w-6 h-6 flex items-center justify-center rounded-full hover:bg-qs-text/10 text-qs-text-dim hover:text-qs-text transition flex-shrink-0"
                    >
                      <X className="w-3 h-3" />
                    </button>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* ── Spinner ── */}
          {loading && (
            <div className="flex justify-center py-16">
              <Spinner />
            </div>
          )}

          {/* ── Résultats ── */}
          {searchResults && !loading && (
            <AnimatePresence mode="wait">
              <motion.div
                key={activeTab}
                initial={{ opacity: 0, y: 6 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, y: -6 }}
                transition={{ duration: 0.15 }}
              >
                {/* ═══ VUE TOUT ═══ */}
                {activeTab === "all" && (
                  <div className="space-y-10">
                    {/* Titres */}
                    {(searchResults.tracks?.items?.length ?? 0) > 0 && (
                      <section>
                        <SectionHeading label="Titres" count={counts.tracks} onMore={() => setActiveTab("tracks")} />
                        <div className="space-y-0.5 mt-4">
                          {searchResults.tracks!.items.slice(0, 5).map((track) => (
                            <TrackRow
                              key={track.id}
                              track={track}
                              onPlayInAlbum={() => playTrackAndRecord(
                                track.id, track.title,
                                track.performer?.name ?? "",
                                track.album?.image?.thumbnail ?? undefined,
                              )}
                            />
                          ))}
                        </div>
                      </section>
                    )}

                    {/* Albums */}
                    {(searchResults.albums?.items?.length ?? 0) > 0 && (
                      <section>
                        <SectionHeading label="Albums" count={counts.albums} onMore={() => setActiveTab("albums")} />
                        <div className="grid grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-3 mt-4">
                          {searchResults.albums!.items.slice(0, 6).map((album) => (
                            <AlbumCard
                              key={album.id}
                              album={album}
                              onClick={() => openAlbum(album.id, album.title, album.artist?.name ?? "", album.image?.thumbnail ?? undefined)}
                            />
                          ))}
                        </div>
                      </section>
                    )}

                    {/* Artistes */}
                    {(searchResults.artists?.items?.length ?? 0) > 0 && (
                      <section>
                        <SectionHeading label="Artistes" count={counts.artists} onMore={() => setActiveTab("artists")} />
                        <div className="grid grid-cols-4 md:grid-cols-5 lg:grid-cols-6 xl:grid-cols-8 gap-3 mt-4">
                          {searchResults.artists!.items.slice(0, 8).map((artist) => (
                            <ArtistCard
                              key={artist.id}
                              artist={artist}
                              onClick={() => openArtist(artist.id, artist.name, artist.image?.large ?? undefined)}
                            />
                          ))}
                        </div>
                      </section>
                    )}

                    {/* Playlists */}
                    {(searchResults.playlists?.items?.length ?? 0) > 0 && (
                      <section>
                        <SectionHeading label="Playlists" count={counts.playlists} onMore={() => setActiveTab("playlists")} />
                        <div className="grid grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-3 mt-4">
                          {searchResults.playlists!.items.slice(0, 6).map((pl) => (
                            <PlaylistCard
                              key={pl.id}
                              playlist={pl}
                              onClick={() => openPlaylist(pl.id, pl.name, pl.images300?.[0])}
                            />
                          ))}
                        </div>
                      </section>
                    )}

                    {/* Aucun résultat */}
                    {counts.tracks === 0 && counts.albums === 0 && counts.artists === 0 && counts.playlists === 0 && (
                      <EmptyState label={`Aucun résultat pour « ${searchQuery} »`} />
                    )}
                  </div>
                )}

                {/* ═══ VUE TITRES ═══ */}
                {activeTab === "tracks" && (
                  <div>
                    {counts.tracks > 0 ? (
                      <div className="space-y-0.5">
                        {searchResults.tracks!.items.map((track) => (
                          <TrackRow
                            key={track.id}
                            track={track}
                            onPlayInAlbum={() => playTrackAndRecord(
                              track.id, track.title,
                              track.performer?.name ?? "",
                              track.album?.image?.thumbnail ?? undefined,
                            )}
                          />
                        ))}
                      </div>
                    ) : (
                      <EmptyState label="Aucun titre trouvé" />
                    )}
                  </div>
                )}

                {/* ═══ VUE ALBUMS ═══ */}
                {activeTab === "albums" && (
                  <div>
                    {counts.albums > 0 ? (
                      <div className="grid grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4">
                        {searchResults.albums!.items.map((album) => (
                          <AlbumCard
                            key={album.id}
                            album={album}
                            onClick={() => openAlbum(album.id, album.title, album.artist?.name ?? "", album.image?.thumbnail ?? undefined)}
                          />
                        ))}
                      </div>
                    ) : (
                      <EmptyState label="Aucun album trouvé" />
                    )}
                  </div>
                )}

                {/* ═══ VUE ARTISTES ═══ */}
                {activeTab === "artists" && (
                  <div>
                    {counts.artists > 0 ? (
                      <div className="grid grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4">
                        {searchResults.artists!.items.map((artist) => (
                          <ArtistCard
                            key={artist.id}
                            artist={artist}
                            onClick={() => openArtist(artist.id, artist.name, artist.image?.large ?? undefined)}
                          />
                        ))}
                      </div>
                    ) : (
                      <EmptyState label="Aucun artiste trouvé" />
                    )}
                  </div>
                )}

                {/* ═══ VUE PLAYLISTS ═══ */}
                {activeTab === "playlists" && (
                  <div>
                    {counts.playlists > 0 ? (
                      <div className="grid grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4">
                        {searchResults.playlists!.items.map((pl) => (
                          <PlaylistCard
                            key={pl.id}
                            playlist={pl}
                            onClick={() => openPlaylist(pl.id, pl.name, pl.images300?.[0])}
                          />
                        ))}
                      </div>
                    ) : (
                      <EmptyState label="Aucune playlist trouvée" />
                    )}
                  </div>
                )}
              </motion.div>
            </AnimatePresence>
          )}

          {/* ── Empty state initial ── */}
          {!searchResults && !loading && !showHistory && (
            <div className="flex flex-col items-center justify-center py-24 text-qs-text-dim">
              <SearchIcon className="w-10 h-10 mb-4 opacity-25" />
              <p className="font-sans text-base font-medium">Rechercher dans le catalogue Qobuz</p>
              <p className="font-condensed text-xs uppercase tracking-wider mt-1.5 opacity-60">
                Titres · Albums · Artistes · Playlists
              </p>
            </div>
          )}

        </div>
      </div>
    </div>
  );
}

// ── Sous-composants ────────────────────────────────────────────────────────
function SectionHeading({ label, count, onMore }: { label: string; count: number; onMore: () => void }) {
  return (
    <div className="flex items-center justify-between">
      <div className="flex items-center gap-2.5">
        <h3 className="font-condensed text-xs font-semibold text-qs-text-dim uppercase tracking-[0.18em]">
          {label}
        </h3>
        {count > 0 && (
          <span className="font-mono text-[9px] text-qs-text-dim bg-qs-text/5 px-1.5 py-0.5 rounded">
            {count}
          </span>
        )}
      </div>
      {count > 5 && (
        <button
          onClick={onMore}
          className="font-condensed text-[10px] uppercase tracking-wider text-qs-accent/70 hover:text-qs-accent transition-colors"
        >
          Voir tout →
        </button>
      )}
    </div>
  );
}

function EmptyState({ label }: { label: string }) {
  return (
    <div className="flex flex-col items-center justify-center py-16 text-qs-text-dim">
      <SearchIcon className="w-8 h-8 mb-3 opacity-20" />
      <p className="font-sans text-sm">{label}</p>
    </div>
  );
}
