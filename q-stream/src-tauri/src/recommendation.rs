use crate::models::*;
use reqwest::Client;
use std::collections::{HashMap, HashSet};

const LASTFM_API_URL: &str = "https://ws.audioscrobbler.com/2.0/";
const LASTFM_API_KEY: &str = "4d033e306826ffc7ddb8d9739ae8f459";

/// Smart Shuffle recommendation engine
/// Hybrid approach: Last.fm similarity + Qobuz catalog cross-referencing
pub struct RecommendationEngine {
    client: Client,
    lastfm_api_key: String,
}

impl RecommendationEngine {
    pub fn new(lastfm_api_key: Option<String>) -> Self {
        Self {
            client: Client::new(),
            lastfm_api_key: lastfm_api_key
                .filter(|k| !k.is_empty())
                .unwrap_or_else(|| LASTFM_API_KEY.to_string()),
        }
    }

    /// Get similar tracks from Last.fm for a given track
    pub async fn get_similar_tracks(
        &self,
        track_name: &str,
        artist_name: &str,
        limit: u32,
    ) -> Result<Vec<LastFmTrack>, String> {
        if self.lastfm_api_key.is_empty() {
            return Ok(Vec::new());
        }

        let resp = self
            .client
            .get(LASTFM_API_URL)
            .query(&[
                ("method", "track.getSimilar"),
                ("artist", artist_name),
                ("track", track_name),
                ("api_key", &self.lastfm_api_key),
                ("format", "json"),
                ("limit", &limit.to_string()),
            ])
            .send()
            .await
            .map_err(|e| format!("Last.fm request failed: {}", e))?;

        let data: LastFmSimilarResponse = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse Last.fm response: {}", e))?;

        Ok(data
            .similartracks
            .map(|st| st.track)
            .unwrap_or_default())
    }

    /// Fetch the global Last.fm top-tracks chart
    pub async fn get_chart_top_tracks(
        &self,
        limit: u32,
    ) -> Result<Vec<(String, String)>, String> {
        let resp = self
            .client
            .get(LASTFM_API_URL)
            .query(&[
                ("method", "chart.getTopTracks"),
                ("api_key", &self.lastfm_api_key),
                ("format", "json"),
                ("limit", &limit.to_string()),
            ])
            .send()
            .await
            .map_err(|e| format!("Last.fm chart request failed: {}", e))?;

        let data: LastFmChartResponse = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse Last.fm chart: {}", e))?;

        Ok(data
            .tracks
            .items
            .into_iter()
            .map(|t| (t.name, t.artist.name))
            .collect())
    }

    /// Cross-reference Last.fm chart tracks with the Qobuz catalog.
    /// Returns playable QobuzTracks ordered by chart popularity.
    pub async fn get_trending_via_qobuz(
        &self,
        qobuz: &crate::qobuz::QobuzClient,
        limit: u32,
    ) -> Result<Vec<QobuzTrack>, String> {
        let chart_tracks = self.get_chart_top_tracks(limit + 8).await?;

        // Parallel Qobuz searches
        let mut set = tokio::task::JoinSet::new();
        for (track_name, artist_name) in chart_tracks.into_iter().take((limit + 8) as usize) {
            let qobuz_clone = qobuz.clone();
            let query = format!("{} {}", track_name, artist_name);
            set.spawn(async move {
                qobuz_clone.search(&query, 1).await
            });
        }

        let mut results: Vec<QobuzTrack> = Vec::new();
        while let Some(res) = set.join_next().await {
            if let Ok(Ok(search_result)) = res {
                if let Some(tracks) = search_result.tracks {
                    if let Some(track) = tracks.items.into_iter().next() {
                        results.push(track);
                    }
                }
            }
            if results.len() >= limit as usize {
                break;
            }
        }

        Ok(results)
    }

    /// Fetch the user's top artists from Last.fm.
    /// `period` is one of: "overall" | "7day" | "1month" | "3month" | "6month" | "12month".
    pub async fn get_user_top_artists(
        &self,
        username: &str,
        period: &str,
        limit: u32,
    ) -> Result<Vec<String>, String> {
        let resp = self
            .client
            .get(LASTFM_API_URL)
            .query(&[
                ("method", "user.getTopArtists"),
                ("user", username),
                ("period", period),
                ("api_key", &self.lastfm_api_key),
                ("format", "json"),
                ("limit", &limit.to_string()),
            ])
            .send()
            .await
            .map_err(|e| format!("Last.fm top-artists request failed: {}", e))?;

        let data: LastFmTopArtistsResponse = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse Last.fm top artists: {}", e))?;

        Ok(data.topartists.items.into_iter().map(|a| a.name).collect())
    }

    /// Fetch the top artists for a Last.fm tag (genre).
    pub async fn get_tag_top_artists(
        &self,
        tag: &str,
        limit: u32,
    ) -> Result<Vec<String>, String> {
        let resp = self
            .client
            .get(LASTFM_API_URL)
            .query(&[
                ("method", "tag.getTopArtists"),
                ("tag", tag),
                ("api_key", &self.lastfm_api_key),
                ("format", "json"),
                ("limit", &limit.to_string()),
            ])
            .send()
            .await
            .map_err(|e| format!("Last.fm tag.getTopArtists request failed: {}", e))?;

        let data: LastFmTopArtistsResponse = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse Last.fm tag top artists: {}", e))?;

        Ok(data.topartists.items.into_iter().map(|a| a.name).collect())
    }

    /// Return Qobuz albums for the user's top Last.fm artists, blending three
    /// time windows: 7-day (50 %), 1-month (30 %), 3-month (20 %).
    pub async fn get_personalized_albums(
        &self,
        lastfm_username: &str,
        qobuz: &crate::qobuz::QobuzClient,
        limit: u32,
    ) -> Result<Vec<QobuzAlbumSimple>, String> {
        // Fetch three temporal windows concurrently
        let (w7, w1m, w3m) = tokio::join!(
            self.get_user_top_artists(lastfm_username, "7day", 6),
            self.get_user_top_artists(lastfm_username, "1month", 5),
            self.get_user_top_artists(lastfm_username, "3month", 4),
        );

        let w7 = w7.unwrap_or_default();
        let w1m = w1m.unwrap_or_default();
        let w3m = w3m.unwrap_or_default();

        // Blend: 7-day gets priority, then 1-month, then 3-month (deduplicated)
        let mut seen_artists: HashSet<String> = HashSet::new();
        let mut blended: Vec<String> = Vec::new();
        for artist in w7.into_iter().take(5)
            .chain(w1m.into_iter().take(4))
            .chain(w3m.into_iter().take(3))
        {
            if seen_artists.insert(artist.to_lowercase()) {
                blended.push(artist);
            }
            if blended.len() >= 10 {
                break;
            }
        }

        if blended.is_empty() {
            return Ok(Vec::new());
        }

        let mut set = tokio::task::JoinSet::new();
        for artist_name in blended.into_iter().take(10) {
            let qobuz_clone = qobuz.clone();
            set.spawn(async move { qobuz_clone.search(&artist_name, 5).await });
        }

        let mut albums: Vec<QobuzAlbumSimple> = Vec::new();
        let mut seen_ids: HashSet<String> = HashSet::new();
        let mut artist_counts: HashMap<String, usize> = HashMap::new();

        while let Some(res) = set.join_next().await {
            if let Ok(Ok(search_result)) = res {
                if let Some(alb_list) = search_result.albums {
                    for alb in alb_list.items.into_iter() {
                        let artist_key = alb
                            .artist
                            .as_ref()
                            .map(|a| a.name.to_lowercase())
                            .unwrap_or_default();
                        let count = artist_counts.entry(artist_key).or_insert(0);
                        if *count < 2 && seen_ids.insert(alb.id.clone()) {
                            *count += 1;
                            albums.push(alb);
                        }
                        if albums.len() >= limit as usize {
                            break;
                        }
                    }
                }
            }
            if albums.len() >= limit as usize {
                break;
            }
        }

        Ok(albums)
    }

    /// Fetch recent tracks with timestamps and compute per-artist decay scores.
    /// Artists are returned sorted by their decay-weighted listen count (most
    /// recent listens get a higher score via exponential decay with ~7-day half-life).
    pub async fn get_recent_artists_with_decay(
        &self,
        username: &str,
        track_limit: u32,
    ) -> Result<Vec<String>, String> {
        let resp = self
            .client
            .get(LASTFM_API_URL)
            .query(&[
                ("method", "user.getRecentTracks"),
                ("user", username),
                ("api_key", &self.lastfm_api_key),
                ("format", "json"),
                ("limit", &track_limit.to_string()),
            ])
            .send()
            .await
            .map_err(|e| format!("Last.fm recentTracks request failed: {}", e))?;

        let data: LastFmRecentTracksResponse = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse Last.fm recentTracks: {}", e))?;

        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as f64;

        // Decay half-life ≈ 7 days → λ = ln(2)/7 ≈ 0.099
        const LAMBDA: f64 = 0.099;

        let mut artist_scores: HashMap<String, f64> = HashMap::new();
        // Keep one canonical (original-cased) name per artist
        let mut artist_canonical: HashMap<String, String> = HashMap::new();

        for track in data.recenttracks.items {
            let name = track.artist.name;
            if name.is_empty() {
                continue;
            }
            let days_ago = if let Some(d) = track.date {
                let uts: f64 = d.uts.parse().unwrap_or(now_secs);
                (now_secs - uts).max(0.0) / 86_400.0
            } else {
                0.0 // currently playing
            };
            let decay = (-LAMBDA * days_ago).exp();
            let key = name.to_lowercase();
            *artist_scores.entry(key.clone()).or_insert(0.0) += decay;
            artist_canonical.entry(key).or_insert(name);
        }

        let mut sorted: Vec<(String, f64)> = artist_scores.into_iter().collect();
        sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        Ok(sorted
            .into_iter()
            .take(10)
            .filter_map(|(key, _)| artist_canonical.remove(&key))
            .collect())
    }

    /// Kept for backward compatibility (used by smart_shuffle).
    #[allow(dead_code)]
    pub async fn get_recent_artists(
        &self,
        username: &str,
        track_limit: u32,
    ) -> Result<Vec<String>, String> {
        self.get_recent_artists_with_decay(username, track_limit).await
    }

    /// Return Qobuz albums matching the user's recently played artists on Last.fm,
    /// using decay-weighted ranking so very recent listens influence results more.
    pub async fn get_recent_playback_albums(
        &self,
        lastfm_username: &str,
        qobuz: &crate::qobuz::QobuzClient,
        limit: usize,
    ) -> Result<Vec<QobuzAlbumSimple>, String> {
        let recent_artists = self
            .get_recent_artists_with_decay(lastfm_username, 50)
            .await?;

        if recent_artists.is_empty() {
            return Ok(Vec::new());
        }

        let mut set = tokio::task::JoinSet::new();
        for artist_name in recent_artists.into_iter().take(10) {
            let qobuz_clone = qobuz.clone();
            set.spawn(async move { qobuz_clone.search(&artist_name, 5).await });
        }

        let mut albums: Vec<QobuzAlbumSimple> = Vec::new();
        let mut seen_ids: HashSet<String> = HashSet::new();
        let mut artist_counts: HashMap<String, usize> = HashMap::new();

        while let Some(res) = set.join_next().await {
            if let Ok(Ok(search_result)) = res {
                if let Some(alb_list) = search_result.albums {
                    for alb in alb_list.items.into_iter() {
                        let artist_key = alb
                            .artist
                            .as_ref()
                            .map(|a| a.name.to_lowercase())
                            .unwrap_or_default();
                        let count = artist_counts.entry(artist_key).or_insert(0);
                        if *count < 2 && seen_ids.insert(alb.id.clone()) {
                            *count += 1;
                            albums.push(alb);
                        }
                        if albums.len() >= limit {
                            break;
                        }
                    }
                }
            }
            if albums.len() >= limit {
                break;
            }
        }

        Ok(albums)
    }

    /// Discovery recommendations based on the user's Qobuz library.
    /// Extracts artists from favorite albums + user playlists, then searches
    /// Qobuz for albums by those artists that are NOT already in the library.
    /// At most 2 albums per artist to ensure variety.
    pub async fn get_library_discovery(
        &self,
        qobuz: &crate::qobuz::QobuzClient,
        limit: usize,
    ) -> Result<Vec<QobuzAlbumSimple>, String> {
        // 1. Fetch the user's library (favorites + playlists)
        let favorites = qobuz.get_favorites().await.map_err(|e| e.to_string())?;

        // Collect owned album IDs to exclude from results
        let mut owned_ids: HashSet<String> = HashSet::new();
        if let Some(ref albs) = favorites.albums {
            for a in &albs.items {
                owned_ids.insert(a.id.clone());
            }
        }

        // 2. Extract unique artist names from favourite albums + playlist tracks
        let mut artist_names: HashSet<String> = HashSet::new();
        if let Some(ref albs) = favorites.albums {
            for a in &albs.items {
                if let Some(ref art) = a.artist {
                    artist_names.insert(art.name.clone());
                }
            }
        }
        if let Some(ref pls) = favorites.playlists {
            for pl in &pls.items {
                if let Some(ref tl) = pl.tracks {
                    for t in &tl.items {
                        if let Some(ref perf) = t.performer {
                            artist_names.insert(perf.name.clone());
                        }
                    }
                }
            }
        }

        if artist_names.is_empty() {
            return Ok(Vec::new());
        }

        // 3. Parallel Qobuz search for each artist
        let artists: Vec<String> = artist_names.into_iter().take(12).collect();
        let mut set = tokio::task::JoinSet::new();
        for artist_name in artists {
            let qobuz_clone = qobuz.clone();
            set.spawn(async move { qobuz_clone.search(&artist_name, 8).await });
        }

        // 4. Collect albums NOT already in the library, max 2 per artist
        let mut albums: Vec<QobuzAlbumSimple> = Vec::new();
        let mut seen_ids = owned_ids; // reuse to also deduplicate within results
        let mut artist_counts: HashMap<String, usize> = HashMap::new();

        while let Some(res) = set.join_next().await {
            if let Ok(Ok(search_result)) = res {
                if let Some(alb_list) = search_result.albums {
                    for alb in alb_list.items.into_iter() {
                        let artist_key = alb
                            .artist
                            .as_ref()
                            .map(|a| a.name.to_lowercase())
                            .unwrap_or_default();
                        let count = artist_counts.entry(artist_key).or_insert(0);
                        if *count < 2 && seen_ids.insert(alb.id.clone()) {
                            *count += 1;
                            albums.push(alb);
                        }
                        if albums.len() >= limit {
                            break;
                        }
                    }
                }
            }
            if albums.len() >= limit {
                break;
            }
        }

        Ok(albums)
    }

    /// Discover albums by artists the user has favorited but hasn't saved to their library.
    /// Useful for surfacing new releases / B-sides from known artists.
    pub async fn get_unknown_albums_by_known_artists(
        &self,
        qobuz: &crate::qobuz::QobuzClient,
        limit: usize,
    ) -> Result<Vec<QobuzAlbumSimple>, String> {
        let favorites = qobuz.get_favorites().await.map_err(|e| e.to_string())?;

        // Albums already owned
        let mut owned_ids: HashSet<String> = HashSet::new();
        if let Some(ref albs) = favorites.albums {
            for a in &albs.items {
                owned_ids.insert(a.id.clone());
            }
        }

        // Favorite artists
        let fav_artists: Vec<QobuzArtistSimple> = favorites
            .artists
            .map(|a| a.items)
            .unwrap_or_default();

        if fav_artists.is_empty() {
            return Ok(Vec::new());
        }

        let mut set = tokio::task::JoinSet::new();
        for artist in fav_artists.into_iter().take(12) {
            let qobuz_clone = qobuz.clone();
            set.spawn(async move { qobuz_clone.search(&artist.name, 8).await });
        }

        let mut albums: Vec<QobuzAlbumSimple> = Vec::new();
        let mut seen_ids = owned_ids;
        let mut artist_counts: HashMap<String, usize> = HashMap::new();

        while let Some(res) = set.join_next().await {
            if let Ok(Ok(search_result)) = res {
                if let Some(alb_list) = search_result.albums {
                    for alb in alb_list.items.into_iter() {
                        let artist_key = alb
                            .artist
                            .as_ref()
                            .map(|a| a.name.to_lowercase())
                            .unwrap_or_default();
                        let count = artist_counts.entry(artist_key).or_insert(0);
                        if *count < 3 && seen_ids.insert(alb.id.clone()) {
                            *count += 1;
                            albums.push(alb);
                        }
                        if albums.len() >= limit {
                            break;
                        }
                    }
                }
            }
            if albums.len() >= limit {
                break;
            }
        }

        Ok(albums)
    }

    /// Genre-based exploration:
    /// 1. Get the user's top Last.fm artists (6 months).
    /// 2. Fetch MusicBrainz genre tags for each artist (in parallel).
    /// 3. Pick the 3 most common genres.
    /// 4. For each genre, get Last.fm's top artists for that tag.
    /// 5. Search Qobuz for albums by those artists.
    pub async fn get_genre_exploration(
        &self,
        lastfm_username: &str,
        qobuz: &crate::qobuz::QobuzClient,
        limit: usize,
    ) -> Result<Vec<QobuzAlbumSimple>, String> {
        // Step 1: user's top artists
        let top_artists = self
            .get_user_top_artists(lastfm_username, "6month", 5)
            .await
            .unwrap_or_default();

        if top_artists.is_empty() {
            return Ok(Vec::new());
        }

        // Step 2: MusicBrainz genres for each top artist (parallel)
        let mut mb_set: tokio::task::JoinSet<Vec<String>> = tokio::task::JoinSet::new();
        for artist_name in top_artists.iter().take(5) {
            let artist_name = artist_name.clone();
            mb_set.spawn(async move {
                let mb = crate::musicbrainz::MusicBrainzClient::new();
                mb.enrich_artist(&artist_name).await.genres
            });
        }

        let mut genre_counts: HashMap<String, usize> = HashMap::new();
        while let Some(Ok(genres)) = mb_set.join_next().await {
            for genre in genres {
                *genre_counts.entry(genre.to_lowercase()).or_insert(0) += 1;
            }
        }

        // Step 3: top 3 genres
        let mut genres_sorted: Vec<(String, usize)> = genre_counts.into_iter().collect();
        genres_sorted.sort_by(|a, b| b.1.cmp(&a.1));
        let top_genres: Vec<String> = genres_sorted
            .into_iter()
            .take(3)
            .map(|(g, _)| g)
            .collect();

        if top_genres.is_empty() {
            return Ok(Vec::new());
        }

        // Step 4: Last.fm tag.getTopArtists for each genre (sequential, only 3 calls)
        let mut genre_artists: Vec<String> = Vec::new();
        let mut seen_artists: HashSet<String> = HashSet::new();
        // Exclude artists the user already listens to a lot
        for already in top_artists.iter() {
            seen_artists.insert(already.to_lowercase());
        }
        for genre in &top_genres {
            if let Ok(artists) = self.get_tag_top_artists(genre, 6).await {
                for a in artists {
                    if seen_artists.insert(a.to_lowercase()) {
                        genre_artists.push(a);
                    }
                }
            }
        }

        if genre_artists.is_empty() {
            return Ok(Vec::new());
        }

        // Step 5: Parallel Qobuz search
        let mut set = tokio::task::JoinSet::new();
        for artist in genre_artists.into_iter().take(12) {
            let qobuz_clone = qobuz.clone();
            set.spawn(async move { qobuz_clone.search(&artist, 5).await });
        }

        let mut albums: Vec<QobuzAlbumSimple> = Vec::new();
        let mut seen_ids: HashSet<String> = HashSet::new();
        let mut artist_counts: HashMap<String, usize> = HashMap::new();

        while let Some(res) = set.join_next().await {
            if let Ok(Ok(search_result)) = res {
                if let Some(alb_list) = search_result.albums {
                    for alb in alb_list.items.into_iter() {
                        let artist_key = alb
                            .artist
                            .as_ref()
                            .map(|a| a.name.to_lowercase())
                            .unwrap_or_default();
                        let count = artist_counts.entry(artist_key).or_insert(0);
                        if *count < 2 && seen_ids.insert(alb.id.clone()) {
                            *count += 1;
                            albums.push(alb);
                        }
                        if albums.len() >= limit {
                            break;
                        }
                    }
                }
            }
            if albums.len() >= limit {
                break;
            }
        }

        Ok(albums)
    }

    /// Weighted shuffle algorithm:
    /// - Avoids repeating the same artist consecutively
    /// - Favors highest bitrate available
    /// - Uses similarity scores for weighting
    pub fn weighted_shuffle(
        tracks: &[UnifiedTrack],
        recent_artists: &[String],
    ) -> Vec<UnifiedTrack> {
        if tracks.is_empty() {
            return Vec::new();
        }

        let mut scored: Vec<(f64, UnifiedTrack)> = tracks
            .iter()
            .map(|track| {
                let mut score: f64 = 1.0;

                // Penalize recently played artists
                let artist_lower = track.artist.to_lowercase();
                for (i, recent) in recent_artists.iter().enumerate() {
                    if recent.to_lowercase() == artist_lower {
                        // More recent = higher penalty
                        let recency = 1.0 - (i as f64 / recent_artists.len().max(1) as f64);
                        score *= 0.2 + (0.8 * (1.0 - recency));
                    }
                }

                // Favor higher quality
                if let Some(sr) = track.sample_rate {
                    if sr >= 192.0 {
                        score *= 1.5;
                    } else if sr >= 96.0 {
                        score *= 1.3;
                    } else if sr >= 44.1 {
                        score *= 1.1;
                    }
                }

                // Add randomness
                score *= 0.5 + (rand_f64() * 0.5);

                (score, track.clone())
            })
            .collect();

        // Sort by score descending
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        // Final pass: break up consecutive same-artist runs
        let mut result: Vec<UnifiedTrack> = scored.into_iter().map(|(_, t)| t).collect();
        let len = result.len();
        if len > 2 {
            for i in 1..len - 1 {
                if result[i].artist == result[i - 1].artist && result[i].artist != result[i + 1].artist
                {
                    result.swap(i, i + 1);
                }
            }
        }

        result
    }
}

/// Simple pseudo-random f64 in [0, 1)
fn rand_f64() -> f64 {
    use std::time::SystemTime;
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    (nanos as f64 % 1000.0) / 1000.0
}
