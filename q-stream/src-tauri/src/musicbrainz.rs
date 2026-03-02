use crate::models::{ArtistEnrichment, MusicBrainzSearchResponse, WikipediaSummary};
use reqwest::Client;
use tracing::{debug, warn};

const MB_BASE: &str = "https://musicbrainz.org/ws/2";
const MB_USER_AGENT: &str =
    "q-stream/0.1 (https://github.com/q-stream/q-stream)";
const WIKI_API: &str = "https://en.wikipedia.org/api/rest_v1/page/summary";

pub struct MusicBrainzClient {
    client: Client,
}

impl MusicBrainzClient {
    pub fn new() -> Self {
        let client = Client::builder()
            .user_agent(MB_USER_AGENT)
            .build()
            .expect("Failed to build MusicBrainz HTTP client");
        Self { client }
    }

    /// Search MusicBrainz for an artist by name.
    /// Returns the best-match artist ID and its genre tags.
    pub async fn search_artist(
        &self,
        name: &str,
    ) -> Result<Option<(String, Vec<String>)>, String> {
        let url = format!("{}/artist", MB_BASE);

        let resp = self
            .client
            .get(&url)
            .query(&[
                ("query", name),
                ("limit", "1"),
                ("fmt", "json"),
            ])
            .send()
            .await
            .map_err(|e| format!("MusicBrainz search failed: {}", e))?;

        if !resp.status().is_success() {
            warn!("MusicBrainz search returned {}", resp.status());
            return Ok(None);
        }

        let data: MusicBrainzSearchResponse = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse MusicBrainz response: {}", e))?;

        let Some(artist) = data.artists.into_iter().next() else {
            return Ok(None);
        };

        debug!("MusicBrainz: found artist {} ({})", artist.name, artist.id);

        // Sort tags by count descending, take top 6
        let mut tags = artist.tags;
        tags.sort_unstable_by(|a, b| b.count.cmp(&a.count));
        let genres: Vec<String> = tags
            .into_iter()
            .take(6)
            .map(|t| normalise_genre(&t.name))
            .collect();

        Ok(Some((artist.id, genres)))
    }

    /// Fetch a short Wikipedia biography for an artist using the
    /// Wikipedia REST summary API (searches by artist name).
    pub async fn get_wikipedia_extract(&self, artist_name: &str) -> Option<String> {
        // Wikipedia titles use underscores; try the artist name as-is first.
        let slug = artist_name.replace(' ', "_");
        let url = format!("{}/{}", WIKI_API, urlencoding::encode(&slug));

        let resp = self.client.get(&url).send().await.ok()?;
        if !resp.status().is_success() {
            return None;
        }

        let summary: WikipediaSummary = resp.json().await.ok()?;
        let extract = summary.extract?;

        // Wikipedia returns a very long extract for disambiguation pages; cap it.
        if extract.len() > 1200 {
            // Truncate at last sentence boundary within the limit
            let truncated = &extract[..1200];
            let last_period = truncated.rfind(". ").map(|i| i + 1).unwrap_or(1200);
            Some(extract[..last_period].trim().to_string())
        } else {
            Some(extract)
        }
    }

    /// High-level: given an artist name, return an `ArtistEnrichment`
    /// containing genre tags and a Wikipedia biography excerpt.
    pub async fn enrich_artist(&self, artist_name: &str) -> ArtistEnrichment {
        let (mbid, genres) = match self.search_artist(artist_name).await {
            Ok(Some((id, tags))) => (Some(id), tags),
            Ok(None) => (None, Vec::new()),
            Err(e) => {
                warn!("MusicBrainz enrichment error for '{}': {}", artist_name, e);
                (None, Vec::new())
            }
        };

        let bio = self.get_wikipedia_extract(artist_name).await;

        ArtistEnrichment { genres, bio, mbid }
    }
}

/// Title-case a genre string ("alternative rock" → "Alternative Rock")
fn normalise_genre(tag: &str) -> String {
    tag.split_whitespace()
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    first.to_uppercase().collect::<String>() + chars.as_str()
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
