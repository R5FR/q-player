import { create } from "zustand";
import type {
  PlaybackState,
  ViewType,
  QobuzSearchResults,
  QobuzAlbum,
  QobuzArtist,
  QobuzPlaylist,
  QobuzFavorites,
  QueueState,
  SessionInfo,
  LocalTrack,
  LastFmUserSession,
  UnifiedTrack,
} from "./types";

interface NavEntry {
  view: ViewType;
  param: string | null;
  albumDetail: QobuzAlbum | null;
  artistDetail: QobuzArtist | null;
  playlistDetail: QobuzPlaylist | null;
}

interface AppStore {
  // Session
  session: SessionInfo;
  setSession: (s: SessionInfo) => void;

  // Navigation
  currentView: ViewType;
  setView: (v: ViewType) => void;
  viewParam: string | null;
  setViewParam: (p: string | null) => void;
  navHistory: NavEntry[];
  navHistoryIndex: number;
  goBack: () => void;
  goForward: () => void;

  // Playback
  playback: PlaybackState;
  setPlayback: (p: PlaybackState) => void;

  // Playback modes
  shuffle: boolean;
  setShuffle: (s: boolean) => void;
  repeatMode: 'off' | 'one' | 'all';
  cycleRepeat: () => void;

  // Search
  searchResults: QobuzSearchResults | null;
  setSearchResults: (r: QobuzSearchResults | null) => void;
  searchQuery: string;
  setSearchQuery: (q: string) => void;

  // Detail views
  albumDetail: QobuzAlbum | null;
  setAlbumDetail: (a: QobuzAlbum | null) => void;
  artistDetail: QobuzArtist | null;
  setArtistDetail: (a: QobuzArtist | null) => void;
  playlistDetail: QobuzPlaylist | null;
  setPlaylistDetail: (p: QobuzPlaylist | null) => void;

  // Favorites
  favorites: QobuzFavorites | null;
  setFavorites: (f: QobuzFavorites | null) => void;

  // Queue
  queue: QueueState;
  setQueue: (q: QueueState) => void;

  // Local library
  localTracks: LocalTrack[];
  setLocalTracks: (t: LocalTrack[]) => void;

  // Recently played
  recentlyPlayed: UnifiedTrack[];
  addRecentlyPlayed: (track: UnifiedTrack) => void;

  // Dismissed recommendation albums
  dismissedAlbums: string[];
  dismissAlbum: (id: string) => void;

  // UI
  dominantColor: [number, number, number];
  setDominantColor: (c: [number, number, number]) => void;
  isLoading: boolean;
  setLoading: (l: boolean) => void;
  error: string | null;
  setError: (e: string | null) => void;

  // Last.fm
  lastfmUser: LastFmUserSession | null;
  setLastfmUser: (u: LastFmUserSession | null) => void;
}

export const useStore = create<AppStore>((set) => ({
  // Session
  session: { logged_in: false },
  setSession: (session) => set({ session }),

  // Navigation
  currentView: "home",
  setView: (view) => set((state) => {
    const entry: NavEntry = {
      view,
      param: state.viewParam,
      albumDetail: state.albumDetail,
      artistDetail: state.artistDetail,
      playlistDetail: state.playlistDetail,
    };
    const truncated = state.navHistory.slice(0, state.navHistoryIndex + 1);
    const newHist = [...truncated, entry];
    return { currentView: view, navHistory: newHist, navHistoryIndex: newHist.length - 1 };
  }),
  viewParam: null,
  setViewParam: (viewParam) => set({ viewParam }),
  navHistory: [{ view: "home" as ViewType, param: null, albumDetail: null, artistDetail: null, playlistDetail: null }],
  navHistoryIndex: 0,
  goBack: () => set((state) => {
    if (state.navHistoryIndex <= 0) return {};
    const newIndex = state.navHistoryIndex - 1;
    const entry = state.navHistory[newIndex];
    return {
      currentView: entry.view,
      viewParam: entry.param,
      navHistoryIndex: newIndex,
      albumDetail: entry.albumDetail,
      artistDetail: entry.artistDetail,
      playlistDetail: entry.playlistDetail,
    };
  }),
  goForward: () => set((state) => {
    if (state.navHistoryIndex >= state.navHistory.length - 1) return {};
    const newIndex = state.navHistoryIndex + 1;
    const entry = state.navHistory[newIndex];
    return {
      currentView: entry.view,
      viewParam: entry.param,
      navHistoryIndex: newIndex,
      albumDetail: entry.albumDetail,
      artistDetail: entry.artistDetail,
      playlistDetail: entry.playlistDetail,
    };
  }),

  // Playback
  playback: {
    is_playing: false,
    position_ms: 0,
    duration_ms: 0,
    volume: 0.7,
  },
  setPlayback: (playback) => set({ playback }),

  // Playback modes
  shuffle: false,
  setShuffle: (shuffle) => set({ shuffle }),
  repeatMode: 'off' as const,
  cycleRepeat: () => set((state) => ({
    repeatMode: state.repeatMode === 'off' ? 'all' : state.repeatMode === 'all' ? 'one' : 'off',
  })),

  // Search
  searchResults: null,
  setSearchResults: (searchResults) => set({ searchResults }),
  searchQuery: "",
  setSearchQuery: (searchQuery) => set({ searchQuery }),

  // Detail views
  albumDetail: null,
  setAlbumDetail: (albumDetail) => set({ albumDetail }),
  artistDetail: null,
  setArtistDetail: (artistDetail) => set({ artistDetail }),
  playlistDetail: null,
  setPlaylistDetail: (playlistDetail) => set({ playlistDetail }),

  // Favorites
  favorites: null,
  setFavorites: (favorites) => set({ favorites }),

  // Queue
  queue: { tracks: [] },
  setQueue: (queue) => set({ queue }),

  // Local library
  localTracks: [],
  setLocalTracks: (localTracks) => set({ localTracks }),

  // Recently played
  recentlyPlayed: [],
  addRecentlyPlayed: (track) => set((state) => {
    const filtered = state.recentlyPlayed.filter((t) => t.id !== track.id);
    return { recentlyPlayed: [track, ...filtered].slice(0, 6) };
  }),

  // Dismissed recommendation albums
  dismissedAlbums: [],
  dismissAlbum: (id) => set((state) => ({
    dismissedAlbums: state.dismissedAlbums.includes(id)
      ? state.dismissedAlbums
      : [...state.dismissedAlbums, id],
  })),

  // UI
  dominantColor: [18, 18, 24],
  setDominantColor: (dominantColor) => set({ dominantColor }),
  isLoading: false,
  setLoading: (isLoading) => set({ isLoading }),
  error: null,
  setError: (error) => set({ error }),

  // Last.fm
  lastfmUser: null,
  setLastfmUser: (lastfmUser) => set({ lastfmUser }),
}));
