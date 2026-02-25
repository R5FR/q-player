import { invoke } from "@tauri-apps/api/core";
import type {
  PlaybackState,
  QobuzAlbum,
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

export async function clearQueue(): Promise<void> {
  return invoke("clear_queue");
}

export async function smartShuffle(lastfmApiKey?: string): Promise<QueueState> {
  return invoke("smart_shuffle", { lastfmApiKey: lastfmApiKey ?? null });
}

// ── Recommendations ──

export async function getTrendingTracks(): Promise<QobuzTrack[]> {
  return invoke("get_trending_tracks");
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
