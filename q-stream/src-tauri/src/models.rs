use serde::{Deserialize, Serialize};

// ── Qobuz API Models ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QobuzTrack {
    pub id: i64,
    pub title: String,
    pub duration: i32,
    pub track_number: i32,
    #[serde(default)]
    pub disk_number: i32,
    #[serde(default)]
    pub explicit: bool,
    #[serde(default)]
    pub hires_available: bool,
    pub album: Option<QobuzAlbumSimple>,
    pub performer: Option<QobuzArtistSimple>,
    #[serde(default)]
    pub maximum_bit_depth: Option<i32>,
    #[serde(default)]
    pub maximum_sampling_rate: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QobuzAlbum {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub release_date_original: Option<String>,
    pub artist: Option<QobuzArtistSimple>,
    pub image: Option<QobuzImage>,
    #[serde(default)]
    pub hires_available: bool,
    #[serde(default)]
    pub duration: Option<i32>,
    #[serde(default)]
    pub tracks_count: Option<i32>,
    pub tracks: Option<QobuzTrackList>,
    #[serde(default)]
    pub genre: Option<QobuzGenre>,
    #[serde(default)]
    pub label: Option<QobuzLabel>,
    #[serde(default)]
    pub maximum_bit_depth: Option<i32>,
    #[serde(default)]
    pub maximum_sampling_rate: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QobuzAlbumSimple {
    pub id: String,
    pub title: String,
    pub image: Option<QobuzImage>,
    pub artist: Option<QobuzArtistSimple>,
    #[serde(default)]
    pub release_date_original: Option<String>,
    #[serde(default)]
    pub maximum_bit_depth: Option<i32>,
    #[serde(default)]
    pub maximum_sampling_rate: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QobuzArtistSimple {
    pub id: i64,
    pub name: String,
    pub image: Option<QobuzImage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QobuzArtist {
    pub id: i64,
    pub name: String,
    pub image: Option<QobuzImage>,
    #[serde(default)]
    pub biography: Option<QobuzBiography>,
    pub albums: Option<QobuzAlbumList>,
    #[serde(default)]
    pub similar_artists: Option<Vec<QobuzArtistSimple>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QobuzBiography {
    pub content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QobuzImage {
    pub small: Option<String>,
    pub thumbnail: Option<String>,
    pub large: Option<String>,
    pub back: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QobuzPlaylist {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub image_rectangle_mini: Option<Vec<String>>,
    pub images300: Option<Vec<String>>,
    #[serde(default)]
    pub duration: Option<i32>,
    #[serde(default)]
    pub tracks_count: Option<i32>,
    pub tracks: Option<QobuzTrackList>,
    pub owner: Option<QobuzPlaylistOwner>,
    #[serde(default)]
    pub is_public: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QobuzPlaylistOwner {
    pub id: Option<i64>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QobuzTrackList {
    pub items: Vec<QobuzTrack>,
    #[serde(default)]
    pub total: Option<i32>,
    #[serde(default)]
    pub offset: Option<i32>,
    #[serde(default)]
    pub limit: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QobuzAlbumList {
    pub items: Vec<QobuzAlbumSimple>,
    #[serde(default)]
    pub total: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QobuzPlaylistList {
    pub items: Vec<QobuzPlaylist>,
    #[serde(default)]
    pub total: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QobuzGenre {
    pub id: Option<i64>,
    pub name: String,
    pub slug: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QobuzLabel {
    pub id: Option<i64>,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QobuzSearchResults {
    pub tracks: Option<QobuzTrackList>,
    pub albums: Option<QobuzAlbumList>,
    pub artists: Option<QobuzArtistList>,
    pub playlists: Option<QobuzPlaylistList>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QobuzArtistList {
    pub items: Vec<QobuzArtistSimple>,
    #[serde(default)]
    pub total: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackFileUrl {
    pub track_id: i64,
    pub duration: Option<i32>,
    pub url: String,
    pub format_id: i32,
    pub mime_type: String,
    pub sampling_rate: f64,
    pub bit_depth: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QobuzLoginResponse {
    pub user_auth_token: String,
    pub user: QobuzUser,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QobuzUser {
    pub id: i64,
    pub login: Option<String>,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub credential: Option<QobuzCredential>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QobuzCredential {
    pub label: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QobuzFeatured {
    pub albums: Option<QobuzAlbumList>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QobuzGenreList {
    pub items: Vec<QobuzGenre>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QobuzFavorites {
    #[serde(default)]
    pub tracks: Option<QobuzTrackList>,
    #[serde(default)]
    pub albums: Option<QobuzAlbumList>,
    #[serde(default)]
    pub artists: Option<QobuzArtistList>,
    #[serde(default)]
    pub playlists: Option<QobuzPlaylistList>,
}

// Wrapper for playlist/getUserPlaylists response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QobuzUserPlaylistsResponse {
    pub playlists: QobuzPlaylistList,
}

// ── Audio Quality ──

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AudioQuality {
    Mp3 = 5,
    FlacLossless = 6,
    FlacHiRes96 = 7,
    FlacHiRes192 = 27,
}

impl AudioQuality {
    pub fn format_id(&self) -> i32 {
        *self as i32
    }

    pub fn label(&self) -> &str {
        match self {
            AudioQuality::Mp3 => "MP3 320kbps",
            AudioQuality::FlacLossless => "FLAC 16-bit/44.1kHz",
            AudioQuality::FlacHiRes96 => "FLAC 24-bit/96kHz",
            AudioQuality::FlacHiRes192 => "FLAC 24-bit/192kHz",
        }
    }

    /// Returns ordered list from highest to lowest quality
    pub fn priority_list() -> Vec<AudioQuality> {
        vec![
            AudioQuality::FlacHiRes192,
            AudioQuality::FlacHiRes96,
            AudioQuality::FlacLossless,
            AudioQuality::Mp3,
        ]
    }
}

// ── Playback State ──

/// Playback state of a remote Qobuz Connect renderer (populated while Q-Stream is inactive/cast).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectRemoteState {
    pub is_playing: bool,
    /// Position in ms at the time of the last RestoreState update.
    pub position_ms: u64,
    pub duration_ms: u64,
    /// Unix timestamp (ms) when position_ms was captured, used to interpolate.
    pub last_updated_at_ms: u64,
    pub track: Option<UnifiedTrack>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackState {
    pub is_playing: bool,
    pub current_track: Option<UnifiedTrack>,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub volume: f32,
    pub quality: Option<String>,
    pub sample_rate: Option<f64>,
    pub bit_depth: Option<i32>,
}

// ── Unified Track (Qobuz + Local) ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedTrack {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_seconds: i32,
    pub cover_url: Option<String>,
    pub source: TrackSource,
    pub quality_label: Option<String>,
    pub sample_rate: Option<f64>,
    pub bit_depth: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrackSource {
    Qobuz { track_id: i64 },
    Local { file_path: String },
}

// ── Queue ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueState {
    pub tracks: Vec<UnifiedTrack>,
    pub current_index: Option<usize>,
}

// ── Local Library ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalTrack {
    pub file_path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_seconds: i32,
    pub track_number: Option<i32>,
    pub cover_data: Option<String>, // base64 encoded
    pub sample_rate: Option<u32>,
    pub bit_depth: Option<u16>,
    pub format: String,
}

// ── Session ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub logged_in: bool,
    pub user_name: Option<String>,
    pub subscription: Option<String>,
}

/// Persistent Last.fm user session (stored in `~/.config/q-stream/lastfm.json`)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LastFmUserSession {
    pub session_key: String,
    pub user_name: String,
}

/// Persisted to disk for session restore on next launch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedSession {
    pub app_id: String,
    pub active_secret: String,
    pub user_auth_token: String,
    pub user_id: i64,
    pub user_name: String,
    pub subscription_label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LastFmChartResponse {
    pub tracks: LastFmChartTracks,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LastFmChartTracks {
    #[serde(rename = "track")]
    pub items: Vec<LastFmChartTrack>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LastFmChartTrack {
    pub name: String,
    pub artist: LastFmArtist,
    #[serde(default)]
    pub playcount: Option<String>,
}

// ── Last.fm models ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LastFmSimilarResponse {
    pub similartracks: Option<LastFmSimilarTracks>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LastFmSimilarTracks {
    pub track: Vec<LastFmTrack>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LastFmTrack {
    pub name: String,
    pub artist: LastFmArtist,
    #[serde(rename = "match")]
    pub match_score: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LastFmArtist {
    pub name: String,
}

// ── Last.fm user history models ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LastFmTopArtistsResponse {
    pub topartists: LastFmTopArtistsContainer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LastFmTopArtistsContainer {
    #[serde(rename = "artist")]
    pub items: Vec<LastFmTopArtist>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LastFmTopArtist {
    pub name: String,
}

// ── Last.fm recent tracks (user.getRecentTracks) ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LastFmRecentTracksResponse {
    pub recenttracks: LastFmRecentTracksContainer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LastFmRecentTracksContainer {
    #[serde(rename = "track")]
    pub items: Vec<LastFmRecentTrack>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LastFmRecentTrack {
    pub name: String,
    pub artist: LastFmRecentTrackArtist,
    /// Present for past tracks; absent for the currently-playing track.
    #[serde(default)]
    pub date: Option<LastFmDate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LastFmDate {
    /// Unix timestamp as a string (e.g. "1741000000")
    pub uts: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LastFmRecentTrackArtist {
    // Last.fm uses "#text" for the artist name in recent tracks
    #[serde(rename = "#text")]
    pub name: String,
}

// ── MusicBrainz / Wikipedia enrichment ──

/// Returned by the `get_artist_enrichment` command to the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtistEnrichment {
    pub genres: Vec<String>,
    pub bio: Option<String>,
    pub mbid: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicBrainzSearchResponse {
    pub artists: Vec<MusicBrainzArtist>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicBrainzArtist {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub tags: Vec<MusicBrainzTag>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicBrainzTag {
    pub count: u32,
    pub name: String,
}

/// Response from Wikipedia REST API summary endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikipediaSummary {
    pub extract: Option<String>,
}

// ── Persistent App Data (saved to ~/.config/q-stream/app_data.json) ──

/// A single entry in the browsing / search history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHistoryEntry {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub cover_url: Option<String>,
    /// One of: "album" | "artist" | "track" | "playlist"
    pub entry_type: String,
}

// ── Qobuz Connect ──

/// A Qobuz Connect renderer visible on the network (reported by the server).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectRenderer {
    pub renderer_id: u64,
    pub name: String,
    pub model: String,
    pub is_active: bool,
}

/// All data persisted across sessions.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PersistentAppData {
    #[serde(default)]
    pub recently_played: Vec<UnifiedTrack>,
    #[serde(default)]
    pub dismissed_albums: Vec<String>,
    #[serde(default)]
    pub search_history: Vec<SearchHistoryEntry>,
}
