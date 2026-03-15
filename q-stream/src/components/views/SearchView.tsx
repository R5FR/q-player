import { useState, useCallback, useEffect, useRef } from "react";
import { motion } from "framer-motion";
import { Search as SearchIcon, X, Disc3, Mic2, Music, ListMusic } from "lucide-react";
import { useStore } from "../../store";
import * as api from "../../api";
import type { SearchHistoryEntry } from "../../types";
import AlbumCard from "../cards/AlbumCard";
import TrackRow from "../cards/TrackRow";

const EntryTypeIcon = ({ type }: { type: string }) => {
  switch (type) {
    case "album": return <Disc3 className="w-4 h-4" />;
    case "artist": return <Mic2 className="w-4 h-4" />;
    case "track": return <Music className="w-4 h-4" />;
    case "playlist": return <ListMusic className="w-4 h-4" />;
    default: return <SearchIcon className="w-4 h-4" />;
  }
};

export default function SearchView() {
  const {
    searchQuery,
    searchResults,
    setSearchResults,
    setView,
    setViewParam,
    setAlbumDetail,
    setArtistDetail,
    setPlaylistDetail,
    searchHistory,
    addSearchHistoryEntry,
    removeSearchHistoryEntry,
    setPlayback,
  } = useStore();
  const [loading, setLoading] = useState(false);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const doSearch = useCallback(
    async (query: string) => {
      if (!query.trim()) { setSearchResults(null); return; }
      setLoading(true);
      try { setSearchResults(await api.search(query)); } catch (e) { console.error(e); }
      setLoading(false);
    },
    [setSearchResults]
  );

  useEffect(() => {
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => doSearch(searchQuery), 350);
    return () => { if (debounceRef.current) clearTimeout(debounceRef.current); };
  }, [searchQuery, doSearch]);

  // ── Navigation helpers (record to history) ──

  const openAlbum = async (albumId: string, title: string, artist: string, coverUrl?: string) => {
    const album = await api.getAlbum(albumId);
    setAlbumDetail(album);
    setViewParam(albumId);
    setView("album");
    addSearchHistoryEntry({ id: albumId, title, subtitle: artist, cover_url: coverUrl, entry_type: "album" });
  };

  const openArtist = async (artistId: number, name: string, coverUrl?: string) => {
    const a = await api.getArtist(artistId);
    setArtistDetail(a);
    setViewParam(artistId.toString());
    setView("artist");
    addSearchHistoryEntry({ id: artistId.toString(), title: name, subtitle: "Artiste", cover_url: coverUrl, entry_type: "artist" });
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
        case "album": {
          const album = await api.getAlbum(entry.id);
          setAlbumDetail(album); setViewParam(entry.id); setView("album");
          break;
        }
        case "artist": {
          const a = await api.getArtist(parseInt(entry.id));
          setArtistDetail(a); setViewParam(entry.id); setView("artist");
          break;
        }
        case "playlist": {
          const pl = await api.getPlaylist(parseInt(entry.id));
          setPlaylistDetail(pl); setViewParam(entry.id); setView("playlist");
          break;
        }
        case "track": {
          const state = await api.playTrack(parseInt(entry.id));
          setPlayback(state);
          break;
        }
      }
      addSearchHistoryEntry(entry); // bubble to top
    } catch (e) { console.error(e); }
  };

  const showHistory = !searchQuery.trim() && searchHistory.length > 0;

  return (
    <div className="p-8">
      {/* ── Recherches récentes ── */}
      {showHistory && !loading && (
        <div className="mb-8">
          <h2 className="text-base font-semibold text-white mb-4">Recherches récentes</h2>
          <div className="space-y-0.5">
            {searchHistory.map((entry) => (
              <motion.div
                key={entry.id + entry.entry_type}
                whileHover={{ backgroundColor: "rgba(0,212,255,0.04)" }}
                className="flex items-center gap-3 p-2 rounded-lg group cursor-pointer"
                onClick={() => openHistoryEntry(entry)}
              >
                <div className="w-10 h-10 rounded-md overflow-hidden flex-shrink-0 bg-qs-surface flex items-center justify-center text-qs-text-dim">
                  {entry.cover_url
                    ? <img src={entry.cover_url} alt="" className="w-full h-full object-cover" />
                    : <EntryTypeIcon type={entry.entry_type} />
                  }
                </div>
                <div className="flex-1 min-w-0">
                  <p className="text-sm font-medium text-white truncate">{entry.title}</p>
                  <p className="text-xs text-qs-text-dim truncate">{entry.subtitle}</p>
                </div>
                <span className="text-[10px] text-qs-text-dim px-1.5 py-0.5 rounded bg-white/5 flex-shrink-0 capitalize">
                  {entry.entry_type}
                </span>
                <button
                  onClick={(e) => { e.stopPropagation(); removeSearchHistoryEntry(entry.id); }}
                  className="opacity-0 group-hover:opacity-100 w-6 h-6 flex items-center justify-center rounded-full hover:bg-white/10 text-qs-text-dim hover:text-white transition flex-shrink-0"
                >
                  <X className="w-3 h-3" />
                </button>
              </motion.div>
            ))}
          </div>
        </div>
      )}

      {/* ── Spinner ── */}
      {loading && (
        <div className="flex justify-center py-12">
          <motion.div
            animate={{ rotate: 360 }}
            transition={{ duration: 2, repeat: Infinity, ease: "linear" }}
            className="w-6 h-6 border-2 border-qs-accent border-t-transparent rounded-full"
          />
        </div>
      )}

      {/* ── Résultats de recherche ── */}
      {searchResults && !loading && (
        <div className="space-y-8">
          {searchResults.tracks?.items?.length ? (
            <section>
              <h3 className="text-lg font-semibold text-white mb-3">Titres</h3>
              <div className="space-y-1">
                {searchResults.tracks.items.slice(0, 8).map((track) => (
                  <TrackRow
                    key={track.id}
                    track={track}
                    onPlayInAlbum={() => playTrackAndRecord(
                      track.id,
                      track.title,
                      track.performer?.name ?? "",
                      track.album?.image?.thumbnail ?? undefined,
                    )}
                  />
                ))}
              </div>
            </section>
          ) : null}

          {searchResults.albums?.items?.length ? (
            <section>
              <h3 className="text-lg font-semibold text-white mb-3">Albums</h3>
              <div className="grid grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4">
                {searchResults.albums.items.slice(0, 12).map((album) => (
                  <AlbumCard
                    key={album.id}
                    album={album}
                    onClick={() => openAlbum(
                      album.id,
                      album.title,
                      album.artist?.name ?? "",
                      album.image?.thumbnail ?? undefined,
                    )}
                  />
                ))}
              </div>
            </section>
          ) : null}

          {searchResults.artists?.items?.length ? (
            <section>
              <h3 className="text-lg font-semibold text-white mb-3">Artistes</h3>
              <div className="grid grid-cols-3 md:grid-cols-4 lg:grid-cols-6 gap-4">
                {searchResults.artists.items.slice(0, 6).map((artist) => (
                  <motion.button
                    key={artist.id}
                    whileHover={{ scale: 1.03 }}
                    onClick={() => openArtist(artist.id, artist.name, artist.image?.large ?? undefined)}
                    className="flex flex-col items-center gap-2 p-4 rounded-xl hover:bg-white/5 transition"
                  >
                    <div className="w-24 h-24 rounded-full overflow-hidden bg-qs-surface">
                      {artist.image?.large
                        ? <img src={artist.image.large} alt={artist.name} className="w-full h-full object-cover" />
                        : <div className="w-full h-full flex items-center justify-center text-3xl">🎤</div>
                      }
                    </div>
                    <p className="text-sm font-medium text-white truncate w-full text-center">{artist.name}</p>
                  </motion.button>
                ))}
              </div>
            </section>
          ) : null}
        </div>
      )}

      {/* ── Empty state ── */}
      {!searchResults && !loading && !showHistory && (
        <div className="flex flex-col items-center justify-center py-20 text-qs-text-dim">
          <SearchIcon className="w-12 h-12 mb-4 opacity-30" />
          <p className="text-lg">Rechercher dans le catalogue Qobuz</p>
          <p className="text-sm mt-1">Titres, albums et artistes en Hi-Res</p>
        </div>
      )}
    </div>
  );
}
