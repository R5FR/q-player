import { useRef } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { ChevronLeft, ChevronRight, Search as SearchIcon, X } from "lucide-react";
import { useStore } from "../store";
import HomeView from "./views/HomeView";
import SearchView from "./views/SearchView";
import FavoritesView from "./views/FavoritesView";
import AlbumView from "./views/AlbumView";
import ArtistView from "./views/ArtistView";
import PlaylistView from "./views/PlaylistView";
import QueueView from "./views/QueueView";
import LocalView from "./views/LocalView";

const viewComponents: Record<string, React.FC> = {
  home: HomeView,
  search: SearchView,
  favorites: FavoritesView,
  album: AlbumView,
  artist: ArtistView,
  playlist: PlaylistView,
  queue: QueueView,
  local: LocalView,
};

export default function MainContent() {
  const {
    currentView,
    navHistory,
    navHistoryIndex,
    goBack,
    goForward,
    setView,
    searchQuery,
    setSearchQuery,
    setSearchResults,
  } = useStore();

  const scrollRef = useRef<HTMLDivElement>(null);
  const canGoBack = navHistoryIndex > 0;
  const canGoForward = navHistoryIndex < navHistory.length - 1;

  const ViewComponent = viewComponents[currentView] || HomeView;

  return (
    <main className="flex-1 flex flex-col overflow-hidden">
      {/* ── Barre de navigation sticky (Spotify-like) ── */}
      <div className="flex items-center gap-3 px-4 py-2.5 flex-shrink-0 glass-heavy">
        {/* Flèches back / forward */}
        <div className="flex items-center gap-1">
          <button
            onClick={goBack}
            disabled={!canGoBack}
            className="w-8 h-8 rounded-full flex items-center justify-center transition
              disabled:opacity-25 disabled:cursor-not-allowed
              enabled:hover:bg-white/10 text-white"
            aria-label="Retour"
          >
            <ChevronLeft className="w-5 h-5" />
          </button>
          <button
            onClick={goForward}
            disabled={!canGoForward}
            className="w-8 h-8 rounded-full flex items-center justify-center transition
              disabled:opacity-25 disabled:cursor-not-allowed
              enabled:hover:bg-white/10 text-white"
            aria-label="Suivant"
          >
            <ChevronRight className="w-5 h-5" />
          </button>
        </div>

        {/* Barre de recherche centrée */}
        <div className="flex-1 max-w-sm relative">
          <SearchIcon className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-qs-text-dim pointer-events-none" />
          <input
            type="text"
            placeholder="Rechercher..."
            value={searchQuery}
            onFocus={() => { if (currentView !== "search") setView("search"); }}
            onChange={(e) => {
              setSearchQuery(e.target.value);
              if (currentView !== "search") setView("search");
            }}
            className="input-cyber w-full pl-9 pr-8 py-2 text-sm"
          />
          {searchQuery && (
            <button
              onClick={() => { setSearchQuery(""); setSearchResults(null); }}
              className="absolute right-2.5 top-1/2 -translate-y-1/2 text-qs-text-dim hover:text-white transition"
            >
              <X className="w-3.5 h-3.5" />
            </button>
          )}
        </div>
      </div>

      {/* ── Contenu scrollable ── */}
      <div ref={scrollRef} className="flex-1 overflow-y-auto overflow-x-hidden">
        <AnimatePresence mode="wait">
          <motion.div
            key={currentView}
            initial={{ opacity: 0, y: 8 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -8 }}
            transition={{ duration: 0.15 }}
            className="h-full"
          >
            <ViewComponent />
          </motion.div>
        </AnimatePresence>
      </div>
    </main>
  );
}
