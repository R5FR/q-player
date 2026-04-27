// ── Q-Stream TypeScript Types ──

export interface QobuzTrack {
  id: number;
  title: string;
  duration: number;
  track_number: number;
  disk_number: number;
  explicit: boolean;
  hires_available: boolean;
  album?: QobuzAlbumSimple;
  performer?: QobuzArtistSimple;
  maximum_bit_depth?: number;
  maximum_sampling_rate?: number;
}

export interface QobuzAlbum {
  id: string;
  title: string;
  release_date_original?: string;
  artist?: QobuzArtistSimple;
  image?: QobuzImage;
  hires_available: boolean;
  duration?: number;
  tracks_count?: number;
  tracks?: QobuzTrackList;
  genre?: QobuzGenre;
  label?: QobuzLabel;
  maximum_bit_depth?: number;
  maximum_sampling_rate?: number;
}

export interface QobuzAlbumSimple {
  id: string;
  title: string;
  image?: QobuzImage;
  artist?: QobuzArtistSimple;
  release_date_original?: string;
  maximum_bit_depth?: number;
  maximum_sampling_rate?: number;
}

export interface QobuzArtistSimple {
  id: number;
  name: string;
  image?: QobuzImage;
}

export interface QobuzArtist {
  id: number;
  name: string;
  image?: QobuzImage;
  biography?: { content?: string };
  albums?: QobuzAlbumList;
  similar_artists?: QobuzArtistSimple[];
}

export interface QobuzImage {
  small?: string;
  thumbnail?: string;
  large?: string;
  back?: string;
}

export interface QobuzPlaylist {
  id: number;
  name: string;
  description?: string;
  images300?: string[];
  image_rectangle_mini?: string[];
  duration?: number;
  tracks_count?: number;
  tracks?: QobuzTrackList;
  owner?: { id?: number; name?: string };
  is_public: boolean;
}

export interface QobuzTrackList {
  items: QobuzTrack[];
  total?: number;
  offset?: number;
  limit?: number;
}

export interface QobuzAlbumList {
  items: QobuzAlbumSimple[];
  total?: number;
}

export interface QobuzPlaylistList {
  items: QobuzPlaylist[];
  total?: number;
}

export interface QobuzGenre {
  id?: number;
  name: string;
  slug?: string;
}

export interface QobuzLabel {
  id?: number;
  name: string;
}

export interface QobuzSearchResults {
  tracks?: QobuzTrackList;
  albums?: QobuzAlbumList;
  artists?: { items: QobuzArtistSimple[]; total?: number };
  playlists?: QobuzPlaylistList;
}

export interface QobuzFavorites {
  tracks?: QobuzTrackList;
  albums?: QobuzAlbumList;
  artists?: { items: QobuzArtistSimple[]; total?: number };
  playlists?: QobuzPlaylistList;
}

export interface UnifiedTrack {
  id: string;
  title: string;
  artist: string;
  album: string;
  duration_seconds: number;
  cover_url?: string;
  source: TrackSource;
  quality_label?: string;
  sample_rate?: number;
  bit_depth?: number;
}

export type TrackSource =
  | { Qobuz: { track_id: number } }
  | { Local: { file_path: string } };

export interface PlaybackState {
  is_playing: boolean;
  current_track?: UnifiedTrack;
  position_ms: number;
  duration_ms: number;
  volume: number;
  quality?: string;
  sample_rate?: number;
  bit_depth?: number;
  /** Actual sample rate the DAC stream was opened at (0 = unknown). */
  output_sample_rate: number;
}

export interface QueueState {
  tracks: UnifiedTrack[];
  current_index?: number;
}

export interface SessionInfo {
  logged_in: boolean;
  user_name?: string;
  subscription?: string;
}

export interface LocalTrack {
  file_path: string;
  title: string;
  artist: string;
  album: string;
  duration_seconds: number;
  track_number?: number;
  cover_data?: string;
  sample_rate?: number;
  bit_depth?: number;
  format: string;
}

export type ViewType =
  | "home"
  | "search"
  | "favorites"
  | "album"
  | "artist"
  | "playlist"
  | "queue"
  | "local"
  | "eq"
  | "settings";

export interface LastFmUserSession {
  session_key: string;
  user_name: string;
}

export interface ArtistEnrichment {
  genres: string[];
  bio?: string;
  mbid?: string;
}

// ── Persistent app data ──

export interface SearchHistoryEntry {
  id: string;
  title: string;
  subtitle: string;
  cover_url?: string;
  entry_type: "album" | "artist" | "track" | "playlist";
}

/** Matches the Rust PersistentAppData struct (snake_case for Tauri interop). */
export interface PersistentAppData {
  recently_played: UnifiedTrack[];
  dismissed_albums: string[];
  search_history: SearchHistoryEntry[];
}

// ── Equalizer ──

export interface EqBand {
  freq: number;   // Hz
  gain: number;   // dB, -12 to +12
  q: number;      // Q factor (bandwidth)
  label: string;  // Display name
}

// ── User Config (persisted to config.toml) ──

/** Backend EQ band format (no label). */
export interface EqBandParam {
  freq: number;
  gain_db: number;
  q: number;
}

/** Matches the Rust UserConfig struct. */
export interface UserConfig {
  volume: number;
  audio_device?: string;
  eq_enabled: boolean;
  eq_bands: EqBandParam[];
  eq_advanced: boolean;
  music_folder?: string;
}
