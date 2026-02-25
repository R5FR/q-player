/// Last.fm authentication and scrobbling client.
///
/// Auth flow:
///   1. `get_auth_token()` → get a short-lived token
///   2. Direct user to `auth_url(token)` (opens in browser)
///   3. `get_session(token)` → exchange for a permanent session key
///   4. Persist session key; use for `update_now_playing` / `scrobble`

use md5::{Digest, Md5};
use reqwest::Client;
use std::collections::BTreeMap;
use tracing::info;

use crate::models::LastFmUserSession;

const LASTFM_API_URL: &str = "https://ws.audioscrobbler.com/2.0/";
pub const LASTFM_API_KEY: &str = "4d033e306826ffc7ddb8d9739ae8f459";
pub const LASTFM_SHARED_SECRET: &str = "fce60c1b09b3c95c50fb1e920be3ef61";

pub struct LastFmClient {
    client: Client,
}

impl LastFmClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// Compute the API signature for a set of parameters.
    /// Algorithm: sort keys alphabetically → concat key+value pairs → append secret → MD5.
    fn sign(&self, params: &BTreeMap<&str, String>) -> String {
        let mut s = String::new();
        for (k, v) in params {
            s.push_str(k);
            s.push_str(v);
        }
        s.push_str(LASTFM_SHARED_SECRET);
        let hash = Md5::digest(s.as_bytes());
        format!("{:x}", hash)
    }

    // ── Auth ──

    /// Step 1: Get a short-lived auth token.
    pub async fn get_auth_token(&self) -> Result<String, String> {
        let mut params: BTreeMap<&str, String> = BTreeMap::new();
        params.insert("api_key", LASTFM_API_KEY.to_string());
        params.insert("method", "auth.getToken".to_string());
        let sig = self.sign(&params);

        let resp: serde_json::Value = self
            .client
            .get(LASTFM_API_URL)
            .query(&[
                ("method", "auth.getToken"),
                ("api_key", LASTFM_API_KEY),
                ("api_sig", &sig),
                ("format", "json"),
            ])
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())?;

        resp["token"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| format!("No token in Last.fm response: {}", resp))
    }

    /// Step 2: Build the URL the user must visit to grant access.
    pub fn auth_url(&self, token: &str) -> String {
        format!(
            "https://www.last.fm/api/auth/?api_key={}&token={}",
            LASTFM_API_KEY, token
        )
    }

    /// Step 3: Exchange the authorised token for a permanent session key.
    pub async fn get_session(&self, token: &str) -> Result<LastFmUserSession, String> {
        let mut params: BTreeMap<&str, String> = BTreeMap::new();
        params.insert("api_key", LASTFM_API_KEY.to_string());
        params.insert("method", "auth.getSession".to_string());
        params.insert("token", token.to_string());
        let sig = self.sign(&params);

        let resp: serde_json::Value = self
            .client
            .get(LASTFM_API_URL)
            .query(&[
                ("method", "auth.getSession"),
                ("api_key", LASTFM_API_KEY),
                ("token", token),
                ("api_sig", &sig),
                ("format", "json"),
            ])
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())?;

        if let Some(err) = resp["error"].as_i64() {
            return Err(format!(
                "Last.fm auth error {}: {}",
                err,
                resp["message"].as_str().unwrap_or("unknown")
            ));
        }

        let key = resp["session"]["key"]
            .as_str()
            .ok_or("No session key in response")?
            .to_string();
        let name = resp["session"]["name"]
            .as_str()
            .unwrap_or("Unknown")
            .to_string();

        Ok(LastFmUserSession {
            session_key: key,
            user_name: name,
        })
    }

    // ── Scrobbling ──

    /// Notify Last.fm that a track has just started playing.
    pub async fn update_now_playing(
        &self,
        session_key: &str,
        track: &str,
        artist: &str,
        duration_secs: u32,
    ) -> Result<(), String> {
        let dur = duration_secs.to_string();
        let mut params: BTreeMap<&str, String> = BTreeMap::new();
        params.insert("api_key", LASTFM_API_KEY.to_string());
        params.insert("artist", artist.to_string());
        params.insert("duration", dur.clone());
        params.insert("method", "track.updateNowPlaying".to_string());
        params.insert("sk", session_key.to_string());
        params.insert("track", track.to_string());
        let sig = self.sign(&params);

        self.client
            .post(LASTFM_API_URL)
            .form(&[
                ("method", "track.updateNowPlaying"),
                ("api_key", LASTFM_API_KEY),
                ("sk", session_key),
                ("artist", artist),
                ("track", track),
                ("duration", &dur),
                ("api_sig", &sig),
                ("format", "json"),
            ])
            .send()
            .await
            .map_err(|e| e.to_string())?;

        info!("Last.fm now playing: {} – {}", artist, track);
        Ok(())
    }

    /// Submit a completed scrobble to Last.fm.
    /// Should be called once the track has been listened to for ≥50% of its duration or ≥4 min.
    pub async fn scrobble(
        &self,
        session_key: &str,
        track: &str,
        artist: &str,
        timestamp_unix: i64,
        duration_secs: u32,
    ) -> Result<(), String> {
        let ts = timestamp_unix.to_string();
        let dur = duration_secs.to_string();

        // Last.fm uses indexed arrays for batch scrobbles; we always send a single one.
        let mut params: BTreeMap<&str, String> = BTreeMap::new();
        params.insert("api_key", LASTFM_API_KEY.to_string());
        params.insert("artist[0]", artist.to_string());
        params.insert("duration[0]", dur.clone());
        params.insert("method", "track.scrobble".to_string());
        params.insert("sk", session_key.to_string());
        params.insert("timestamp[0]", ts.clone());
        params.insert("track[0]", track.to_string());
        let sig = self.sign(&params);

        self.client
            .post(LASTFM_API_URL)
            .form(&[
                ("method", "track.scrobble"),
                ("api_key", LASTFM_API_KEY),
                ("sk", session_key),
                ("artist[0]", artist),
                ("track[0]", track),
                ("timestamp[0]", &ts),
                ("duration[0]", &dur),
                ("api_sig", &sig),
                ("format", "json"),
            ])
            .send()
            .await
            .map_err(|e| e.to_string())?;

        info!("Last.fm scrobbled: {} – {}", artist, track);
        Ok(())
    }
}
