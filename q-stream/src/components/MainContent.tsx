import { motion, AnimatePresence } from "framer-motion";
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
  const { currentView } = useStore();
  const ViewComponent = viewComponents[currentView] || HomeView;

  return (
    <main className="flex-1 overflow-y-auto overflow-x-hidden">
      <AnimatePresence mode="wait">
        <motion.div
          key={currentView}
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          exit={{ opacity: 0, y: -10 }}
          transition={{ duration: 0.2 }}
          className="h-full"
        >
          <ViewComponent />
        </motion.div>
      </AnimatePresence>
    </main>
  );
}
