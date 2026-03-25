import { invoke } from "@tauri-apps/api/core";
import type {
  PlaybackState,
  QobuzAlbum,
  QobuzAlbumSimple,
  QobuzArtist,
  QobuzAlbumList,
  QobuzFavorites,
  QobuzPlaylist,
  QobuzPlaylistList,
  QobuzSearchResults,
  QobuzTrack,
  QueueState,
  SessionInfo,
  UnifiedTrack,
  LocalTrack,
  LastFmUserSession,
  ArtistEnrichment,
  PersistentAppData,
  EqBand,
} from "./types";

// ── Auth ──

export async function login(email: string, password: string): Promise<SessionInfo> {
  return invoke("login", { email, password });
}

export async function logout(): Promise<void> {
  return invoke("logout");
}

export async function getSession(): Promise<SessionInfo> {
  return invoke("get_session");
}

export async function restoreSession(): Promise<SessionInfo> {
  return invoke("restore_session");
}

// ── Playback ──

export async function playTrack(trackId: number): Promise<PlaybackState> {
  return invoke("play_track", { trackId });
}

export async function pause(): Promise<void> {
  return invoke("pause");
}

export async function resume(): Promise<void> {
  return invoke("resume");
}

export async function stop(): Promise<void> {
  return invoke("stop");
}

export async function seek(positionMs: number): Promise<void> {
  return invoke("seek", { positionMs });
}

export async function setVolume(volume: number): Promise<void> {
  return invoke("set_volume", { volume });
}

export async function getPlaybackState(): Promise<PlaybackState> {
  return invoke("get_playback_state");
}

export async function nextTrack(): Promise<PlaybackState | null> {
  return invoke("next_track");
}

export async function previousTrack(): Promise<PlaybackState | null> {
  return invoke("previous_track");
}

export async function playFromQueue(idx: number): Promise<PlaybackState> {
  return invoke("play_from_queue", { idx });
}

// ── Browse ──

export async function search(query: string): Promise<QobuzSearchResults> {
  return invoke("search", { query });
}

export async function getAlbum(albumId: string): Promise<QobuzAlbum> {
  return invoke("get_album", { albumId });
}

export async function getArtist(artistId: number): Promise<QobuzArtist> {
  return invoke("get_artist", { artistId });
}

export async function getPlaylist(playlistId: number): Promise<QobuzPlaylist> {
  return invoke("get_playlist", { playlistId });
}

export async function getFeaturedAlbums(genreId?: string): Promise<QobuzAlbumList> {
  return invoke("get_featured_albums", { genreId: genreId ?? null });
}

export async function getFeaturedPlaylists(): Promise<QobuzPlaylistList> {
  return invoke("get_featured_playlists");
}

export async function getGenres(): Promise<{ items: Array<{ id?: number; name: string }> }> {
  return invoke("get_genres");
}

// ── Favorites ──

export async function getFavorites(): Promise<QobuzFavorites> {
  return invoke("get_favorites");
}

export async function addFavorite(itemType: string, itemId: string): Promise<void> {
  return invoke("add_favorite", { itemType, itemId });
}

export async function removeFavorite(itemType: string, itemId: string): Promise<void> {
  return invoke("remove_favorite", { itemType, itemId });
}

// ── Queue ──

export async function getQueue(): Promise<QueueState> {
  return invoke("get_queue");
}

export async function addToQueue(track: UnifiedTrack): Promise<QueueState> {
  return invoke("add_to_queue", { track });
}

export async function addTracksToQueue(tracks: UnifiedTrack[]): Promise<QueueState> {
  return invoke("add_tracks_to_queue", { tracks });
}

export async function clearQueue(): Promise<void> {
  return invoke("clear_queue");
}

export async function playNext(track: UnifiedTrack): Promise<QueueState> {
  return invoke("play_next", { track });
}

export async function removeFromQueue(index: number): Promise<QueueState> {
  return invoke("remove_from_queue", { index });
}

export async function smartShuffle(lastfmApiKey?: string): Promise<QueueState> {
  return invoke("smart_shuffle", { lastfmApiKey: lastfmApiKey ?? null });
}

export async function enqueueSimilar(trackTitle: string, trackArtist: string): Promise<number> {
  return invoke("enqueue_similar", { trackTitle, trackArtist });
}

// ── Recommendations ──

export async function getTrendingTracks(): Promise<QobuzTrack[]> {
  return invoke("get_trending_tracks");
}

export async function getPersonalizedRecommendations(lastfmUsername: string): Promise<QobuzAlbumSimple[]> {
  return invoke("get_personalized_recommendations", { lastfmUsername });
}

/** Albums matching recently played artists (Last.fm user.getRecentTracks → Qobuz). */
export async function getRecentPlaybackRecs(lastfmUsername: string): Promise<QobuzAlbumSimple[]> {
  return invoke("get_recent_playback_recommendations", { lastfmUsername });
}

/** Discovery albums based on artists in the user's Qobuz library (favorites + playlists). */
export async function getLibraryDiscovery(): Promise<QobuzAlbumSimple[]> {
  return invoke("get_library_discovery");
}

/** The user's own Qobuz playlists. */
export async function getUserPlaylists(): Promise<QobuzPlaylist[]> {
  return invoke("get_user_playlists");
}

/** MusicBrainz genres + Wikipedia excerpt for an artist. */
export async function getArtistEnrichment(artistName: string): Promise<ArtistEnrichment> {
  return invoke("get_artist_enrichment", { artistName });
}

/** Albums by the user's favorite Qobuz artists not yet saved in their library. */
export async function getUnknownAlbumsByKnownArtists(): Promise<QobuzAlbumSimple[]> {
  return invoke("get_unknown_albums_by_known_artists");
}

/** Genre-exploration recommendations derived from the user's Last.fm listening history. */
export async function getGenreExploration(lastfmUsername: string): Promise<QobuzAlbumSimple[]> {
  return invoke("get_genre_exploration", { lastfmUsername });
}

// ── Local Library ──

export async function importFolder(folderPath: string): Promise<LocalTrack[]> {
  return invoke("import_folder", { folderPath });
}

export async function getLocalTracks(): Promise<LocalTrack[]> {
  return invoke("get_local_tracks");
}

export async function playLocalTrack(filePath: string): Promise<PlaybackState> {
  return invoke("play_local_track", { filePath });
}

// ── UI ──

export async function extractDominantColor(imageUrl: string): Promise<[number, number, number]> {
  return invoke("extract_dominant_color", { imageUrl });
}

// ── Last.fm ──

/** Step 1: returns the Last.fm authorisation URL to open in the browser. */
export async function lastfmStartAuth(): Promise<string> {
  return invoke("lastfm_start_auth");
}

/** Step 2: exchange the pending token for a session key (user must have authorised first). */
export async function lastfmCompleteAuth(): Promise<LastFmUserSession> {
  return invoke("lastfm_complete_auth");
}

export async function lastfmGetSession(): Promise<LastFmUserSession | null> {
  return invoke("lastfm_get_session");
}

export async function lastfmDisconnect(): Promise<void> {
  return invoke("lastfm_disconnect");
}

export async function lastfmNowPlaying(
  track: string,
  artist: string,
  durationSecs: number,
): Promise<void> {
  return invoke("lastfm_now_playing", { track, artist, durationSecs });
}

export async function lastfmScrobble(
  track: string,
  artist: string,
  durationSecs: number,
): Promise<void> {
  return invoke("lastfm_scrobble", { track, artist, durationSecs });
}

// ── Equalizer ──

export async function setEq(bands: EqBand[], enabled: boolean): Promise<void> {
  // Map frontend `gain` → Rust `gain_db`
  const backendBands = bands.map((b) => ({ freq: b.freq, gain_db: b.gain, q: b.q }));
  return invoke("set_eq", { bands: backendBands, enabled });
}

export async function getEqState(): Promise<{ enabled: boolean; bands: EqBand[] }> {
  // Map Rust `gain_db` → frontend `gain`
  const raw: { enabled: boolean; bands: Array<{ freq: number; gain_db: number; q: number }> } =
    await invoke("get_eq_state");
  return {
    enabled: raw.enabled,
    bands: raw.bands.map((b) => ({ freq: b.freq, gain: b.gain_db, q: b.q, label: "" })),
  };
}

// ── Spectrum ──

export async function getSpectrum(): Promise<number[]> {
  return invoke("get_spectrum");
}

// ── Audio Devices ──

export async function getAudioDevices(): Promise<string[]> {
  return invoke("get_audio_devices");
}

export async function setAudioDevice(deviceName: string | null): Promise<void> {
  return invoke("set_audio_device", { deviceName });
}

// ── Qobuz Connect ──

export async function scanConnectDevices(): Promise<void> {
  return invoke("scan_connect_devices");
}

export async function startQobuzConnect(): Promise<void> {
  return invoke("start_qobuz_connect");
}

export async function stopQobuzConnect(): Promise<void> {
  return invoke("stop_qobuz_connect");
}

export async function getConnectStatus(): Promise<boolean> {
  return invoke("get_connect_status");
}

export interface ConnectRenderer {
  renderer_id: number;
  name: string;
  model: string;
  is_active: boolean;
}

export async function getConnectRenderers(): Promise<ConnectRenderer[]> {
  return invoke("get_connect_renderers");
}

export async function castToRenderer(rendererId: number): Promise<void> {
  return invoke("cast_to_renderer", { rendererId });
}

/** Control the active remote renderer (play/pause/seek/next/prev) after casting. */
export async function controlRendererPlayback(
  action: "play" | "pause" | "seek" | "next" | "prev",
  positionMs?: number,
): Promise<void> {
  return invoke("control_renderer_playback", { action, positionMs: positionMs ?? null });
}

// ── Persistence ──

/** Load recently played, dismissed albums and search history from disk. */
export async function loadAppData(): Promise<PersistentAppData> {
  return invoke("load_app_data");
}

/** Persist recently played, dismissed albums and search history to disk. */
export async function saveAppData(data: PersistentAppData): Promise<void> {
  return invoke("save_app_data", { data });
}
