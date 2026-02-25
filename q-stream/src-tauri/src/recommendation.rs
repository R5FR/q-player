use crate::models::*;
use reqwest::Client;

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
