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
} from "./types";

interface AppStore {
  // Session
  session: SessionInfo;
  setSession: (s: SessionInfo) => void;

  // Navigation
  currentView: ViewType;
  setView: (v: ViewType) => void;
  viewParam: string | null;
  setViewParam: (p: string | null) => void;

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
  setView: (currentView) => set({ currentView }),
  viewParam: null,
  setViewParam: (viewParam) => set({ viewParam }),

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
