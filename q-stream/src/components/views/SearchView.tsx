import { useState, useCallback, useEffect, useRef } from "react";
import { motion } from "framer-motion";
import { Search as SearchIcon, X } from "lucide-react";
import { useStore } from "../../store";
import * as api from "../../api";
import AlbumCard from "../cards/AlbumCard";
import TrackRow from "../cards/TrackRow";

export default function SearchView() {
  const {
    searchQuery,
    setSearchQuery,
    searchResults,
    setSearchResults,
    setView,
    setViewParam,
    setAlbumDetail,
  } = useStore();
  const [loading, setLoading] = useState(false);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const doSearch = useCallback(
    async (query: string) => {
      if (!query.trim()) {
        setSearchResults(null);
        return;
      }
      setLoading(true);
      try {
        const results = await api.search(query);
        setSearchResults(results);
      } catch (e) {
        console.error(e);
      }
      setLoading(false);
    },
    [setSearchResults]
  );

  // Debounced instant search — fires 350ms after user stops typing
  useEffect(() => {
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => {
      doSearch(searchQuery);
    }, 350);
    return () => {
      if (debounceRef.current) clearTimeout(debounceRef.current);
    };
  }, [searchQuery, doSearch]);

  const openAlbum = async (albumId: string) => {
    const album = await api.getAlbum(albumId);
    setAlbumDetail(album);
    setViewParam(albumId);
    setView("album");
  };

  return (
    <div className="p-8">
      {/* Search bar */}
      <div className="relative max-w-xl mb-8">
        <SearchIcon className="absolute left-4 top-1/2 -translate-y-1/2 w-5 h-5 text-qs-text-dim" />
        <input
          type="text"
          placeholder="Search tracks, albums, artists..."
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          autoFocus
          className="w-full pl-12 pr-10 py-3.5 bg-white/5 border border-white/10 rounded-2xl text-white placeholder:text-qs-text-dim focus:outline-none focus:border-qs-accent/50 transition text-sm"
        />
        {searchQuery && (
          <button
            onClick={() => {
              setSearchQuery("");
              setSearchResults(null);
            }}
            className="absolute right-3 top-1/2 -translate-y-1/2 text-qs-text-dim hover:text-white"
          >
            <X className="w-4 h-4" />
          </button>
        )}
      </div>

      {loading && (
        <div className="flex justify-center py-12">
          <motion.div
            animate={{ rotate: 360 }}
            transition={{ duration: 2, repeat: Infinity, ease: "linear" }}
            className="w-6 h-6 border-2 border-qs-accent border-t-transparent rounded-full"
          />
        </div>
      )}

      {searchResults && !loading && (
        <div className="space-y-8">
          {/* Tracks */}
          {searchResults.tracks?.items?.length ? (
            <section>
              <h3 className="text-lg font-semibold text-white mb-3">Tracks</h3>
              <div className="space-y-1">
                {searchResults.tracks.items.slice(0, 8).map((track) => (
                  <TrackRow key={track.id} track={track} />
                ))}
              </div>
            </section>
          ) : null}

          {/* Albums */}
          {searchResults.albums?.items?.length ? (
            <section>
              <h3 className="text-lg font-semibold text-white mb-3">Albums</h3>
              <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-4">
                {searchResults.albums.items.slice(0, 10).map((album) => (
                  <AlbumCard
                    key={album.id}
                    album={album}
                    onClick={() => openAlbum(album.id)}
                  />
                ))}
              </div>
            </section>
          ) : null}

          {/* Artists */}
          {searchResults.artists?.items?.length ? (
            <section>
              <h3 className="text-lg font-semibold text-white mb-3">
                Artists
              </h3>
              <div className="grid grid-cols-3 md:grid-cols-4 lg:grid-cols-6 gap-4">
                {searchResults.artists.items.slice(0, 6).map((artist) => (
                  <motion.button
                    key={artist.id}
                    whileHover={{ scale: 1.03 }}
                    onClick={async () => {
                      const a = await api.getArtist(artist.id);
                      useStore.getState().setArtistDetail(a);
                      setViewParam(artist.id.toString());
                      setView("artist");
                    }}
                    className="flex flex-col items-center gap-2 p-4 rounded-xl hover:bg-white/5 transition"
                  >
                    <div className="w-24 h-24 rounded-full overflow-hidden bg-qs-surface">
                      {artist.image?.large ? (
                        <img
                          src={artist.image.large}
                          alt={artist.name}
                          className="w-full h-full object-cover"
                        />
                      ) : (
                        <div className="w-full h-full flex items-center justify-center text-3xl">
                          🎤
                        </div>
                      )}
                    </div>
                    <p className="text-sm font-medium text-white truncate w-full text-center">
                      {artist.name}
                    </p>
                  </motion.button>
                ))}
              </div>
            </section>
          ) : null}
        </div>
      )}

      {!searchResults && !loading && (
        <div className="flex flex-col items-center justify-center py-20 text-qs-text-dim">
          <SearchIcon className="w-12 h-12 mb-4 opacity-30" />
          <p className="text-lg">Search the Qobuz catalog</p>
          <p className="text-sm mt-1">
            Find Hi-Res tracks, albums, and artists
          </p>
        </div>
      )}
    </div>
  );
}
