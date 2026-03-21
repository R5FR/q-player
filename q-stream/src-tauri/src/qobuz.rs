use crate::models::*;
use base64::Engine;
use md5::{Digest, Md5};
use regex::Regex;
use reqwest::Client;
use thiserror::Error;
use tracing::{debug, info, warn};
use serde_json::Value as JsonValue;

const QOBUZ_BASE_URL: &str = "https://www.qobuz.com/api.json/0.2";
const QOBUZ_PLAY_URL: &str = "https://play.qobuz.com/login";

#[derive(Error, Debug)]
pub enum QobuzError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Failed to parse response: {0}")]
    Parse(String),
    #[error("Authentication failed: {0}")]
    Auth(String),
    #[error("Secret extraction failed: {0}")]
    Secret(String),
    #[error("API error: {0}")]
    Api(String),
}

#[derive(Debug, Clone)]
pub struct QobuzClient {
    client: Client,
    app_id: String,
    active_secret: String,
    user_auth_token: String,
    user_id: i64,
    user_name: String,
    subscription_label: Option<String>,
}

impl QobuzClient {
    /// Authenticate with Qobuz: extract app secrets, login, find active secret
    pub async fn login(email: &str, password: &str) -> Result<Self, QobuzError> {
        let client = Client::builder()
            .cookie_store(true)
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .build()?;

        // Step 1: Extract app_id and secrets from bundle.js
        info!("Extracting Qobuz app credentials...");
        let (app_id, secrets) = Self::extract_secrets(&client).await?;
        info!("Found app_id: {}, {} candidate secrets", app_id, secrets.len());

        // Step 2: Login
        let login_url = format!("{}/user/login", QOBUZ_BASE_URL);
        let resp = client
            .get(&login_url)
            .query(&[("email", email), ("password", password), ("app_id", &app_id)])
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(QobuzError::Auth(format!(
                "Login failed with status {}",
                resp.status()
            )));
        }

        let login_data: QobuzLoginResponse = resp
            .json()
            .await
            .map_err(|e| QobuzError::Auth(format!("Failed to parse login response: {}", e)))?;

        let user_auth_token = login_data.user_auth_token;
        let user_id = login_data.user.id;
        let user_name = login_data
            .user
            .display_name
            .unwrap_or_else(|| "User".to_string());
        let subscription_label = login_data
            .user
            .credential
            .and_then(|c| c.label);

        info!("Logged in as: {} (id: {})", user_name, user_id);

        // Step 3: Find active secret by testing track/getFileUrl
        let active_secret =
            Self::find_active_secret(&client, &app_id, &user_auth_token, &secrets).await?;

        info!("Active secret found successfully");

        Ok(Self {
            client,
            app_id,
            active_secret,
            user_auth_token,
            user_id,
            user_name,
            subscription_label,
        })
    }

    /// Extract app_id and secrets from Qobuz web player bundle
    async fn extract_secrets(client: &Client) -> Result<(String, Vec<String>), QobuzError> {
        // Fetch the login page
        let html = client
            .get(QOBUZ_PLAY_URL)
            .send()
            .await?
            .text()
            .await?;

        // Find bundle.js URL
        let bundle_re = Regex::new(r#"<script src="(/resources/[^"]+/bundle\.js)"#)
            .map_err(|e| QobuzError::Secret(e.to_string()))?;

        let bundle_path = bundle_re
            .captures(&html)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .ok_or_else(|| QobuzError::Secret("bundle.js not found in HTML".into()))?;

        let bundle_url = format!("https://play.qobuz.com{}", bundle_path);
        debug!("Fetching bundle from: {}", bundle_url);

        let bundle = client.get(&bundle_url).send().await?.text().await?;

        // Extract app_id
        let app_id_re = Regex::new(r#"production:\{api:\{appId:"(\d{9,})",appSecret:"#)
            .map_err(|e| QobuzError::Secret(e.to_string()))?;

        let app_id = app_id_re
            .captures(&bundle)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .ok_or_else(|| QobuzError::Secret("app_id not found in bundle".into()))?;

        // Extract seeds & info/extras pairs to construct secrets
        let seed_re = Regex::new(
            r#"[a-z]\.initialSeed\("(?P<seed>[\w=]+)",window\.utimezone\.(?P<timezone>[a-z]+)\)"#,
        )
        .map_err(|e| QobuzError::Secret(e.to_string()))?;

        // Collect all (timezone, seed) pairs
        let mut timezone_seeds: Vec<(String, String)> = Vec::new();
        for cap in seed_re.captures_iter(&bundle) {
            let seed = cap["seed"].to_string();
            let timezone = cap["timezone"].to_string();
            timezone_seeds.push((timezone, seed));
        }

        let mut secrets = Vec::new();

        // For each seed timezone, build a specific regex to find matching info/extras
        for (timezone, seed) in &timezone_seeds {
            // Capitalize first letter to match timezone city name in bundle
            let tz_capitalized = {
                let mut chars = timezone.chars();
                match chars.next() {
                    Some(c) => format!("{}{}", c.to_uppercase(), chars.as_str()),
                    None => continue,
                }
            };

            let info_pattern = format!(
                r#"name:"\w+/{}([a-z]?)",info:"(?P<info>[\w=]+)",extras:"(?P<extras>[\w=]+)""#,
                regex::escape(&tz_capitalized)
            );

            let info_re = match Regex::new(&info_pattern) {
                Ok(re) => re,
                Err(_) => continue,
            };

            for cap in info_re.captures_iter(&bundle) {
                let info = &cap["info"];
                let extras = &cap["extras"];

                // Construct the combined string and decode
                let combined = format!("{}{}{}", seed, info, extras);
                // Remove trailing 44 chars and base64 decode
                if combined.len() > 44 {
                    let trimmed = &combined[..combined.len() - 44];
                    // Try URL_SAFE base64 (with and without padding)
                    let decoded = base64::engine::general_purpose::URL_SAFE
                        .decode(trimmed)
                        .or_else(|_| {
                            base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(trimmed)
                        })
                        .or_else(|_| {
                            base64::engine::general_purpose::STANDARD.decode(trimmed)
                        });

                    if let Ok(bytes) = decoded {
                        if let Ok(secret) = String::from_utf8(bytes) {
                            debug!("Decoded secret for timezone {}: {}...", timezone, &secret[..secret.len().min(8)]);
                            secrets.push(secret);
                        }
                    }
                }
            }
        }

        // Fallback: try a simpler regex for secrets
        if secrets.is_empty() {
            let simple_re = Regex::new(r#"appSecret:"([a-f0-9]{32})""#)
                .map_err(|e| QobuzError::Secret(e.to_string()))?;
            for cap in simple_re.captures_iter(&bundle) {
                secrets.push(cap[1].to_string());
            }
        }

        if secrets.is_empty() {
            return Err(QobuzError::Secret(
                "No secrets could be extracted from bundle".into(),
            ));
        }

        Ok((app_id, secrets))
    }

    /// Test each secret to find the one that works
    async fn find_active_secret(
        client: &Client,
        app_id: &str,
        user_auth_token: &str,
        secrets: &[String],
    ) -> Result<String, QobuzError> {
        let test_track_id: i64 = 64868955;
        let test_format_id = 5; // MP3 - universally available

        for secret in secrets {
            let timestamp = chrono::Utc::now().timestamp();
            let sig_input = format!(
                "trackgetFileUrlformat_id{}intentstreamtrack_id{}{}{}",
                test_format_id, test_track_id, timestamp, secret
            );

            let mut hasher = Md5::new();
            hasher.update(sig_input.as_bytes());
            let sig = format!("{:x}", hasher.finalize());

            let url = format!("{}/track/getFileUrl", QOBUZ_BASE_URL);
            let resp = client
                .get(&url)
                .header("X-App-Id", app_id)
                .header("X-User-Auth-Token", user_auth_token)
                .query(&[
                    ("request_ts", &timestamp.to_string()),
                    ("request_sig", &sig),
                    ("track_id", &test_track_id.to_string()),
                    ("format_id", &test_format_id.to_string()),
                    ("intent", &"stream".to_string()),
                ])
                .send()
                .await;

            match resp {
                Ok(r) if r.status().is_success() => {
                    debug!("Found active secret");
                    return Ok(secret.clone());
                }
                Ok(r) => {
                    debug!("Secret test failed with status {}", r.status());
                }
                Err(e) => {
                    warn!("Secret test request error: {}", e);
                }
            }
        }

        Err(QobuzError::Secret(
            "None of the extracted secrets are valid".into(),
        ))
    }

    /// Generate MD5 signature for track/getFileUrl
    fn sign_track_request(&self, track_id: i64, format_id: i32, timestamp: i64) -> String {
        let sig_input = format!(
            "trackgetFileUrlformat_id{}intentstreamtrack_id{}{}{}",
            format_id, track_id, timestamp, self.active_secret
        );
        let mut hasher = Md5::new();
        hasher.update(sig_input.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Get streaming URL for a track (tries highest quality first)
    pub async fn get_track_url(&self, track_id: i64) -> Result<TrackFileUrl, QobuzError> {
        let qualities = AudioQuality::priority_list();

        for quality in &qualities {
            match self.get_track_url_with_quality(track_id, quality.format_id()).await {
                Ok(url) => return Ok(url),
                Err(e) => {
                    debug!(
                        "Quality {} unavailable for track {}: {}",
                        quality.label(),
                        track_id,
                        e
                    );
                }
            }
        }

        Err(QobuzError::Api(format!(
            "No streaming URL available for track {}",
            track_id
        )))
    }

    async fn get_track_url_with_quality(
        &self,
        track_id: i64,
        format_id: i32,
    ) -> Result<TrackFileUrl, QobuzError> {
        let timestamp = chrono::Utc::now().timestamp();
        let sig = self.sign_track_request(track_id, format_id, timestamp);

        let url = format!("{}/track/getFileUrl", QOBUZ_BASE_URL);
        let resp = self
            .client
            .get(&url)
            .header("X-App-Id", &self.app_id)
            .header("X-User-Auth-Token", &self.user_auth_token)
            .query(&[
                ("request_ts", &timestamp.to_string()),
                ("request_sig", &sig),
                ("track_id", &track_id.to_string()),
                ("format_id", &format_id.to_string()),
                ("intent", &"stream".to_string()),
            ])
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(QobuzError::Api(format!(
                "getFileUrl returned {}",
                resp.status()
            )));
        }

        let track_url: TrackFileUrl = resp
            .json()
            .await
            .map_err(|e| QobuzError::Parse(e.to_string()))?;

        Ok(track_url)
    }

    /// Fetch track audio data. Checks the disk cache first; on a miss downloads to memory,
    /// starts the audio immediately, and saves to disk in a background thread for future plays.
    pub async fn fetch_track_bytes(&self, track_url: &TrackFileUrl) -> Result<Vec<u8>, QobuzError> {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("q-stream")
            .join("audio_cache");

        let ext = match track_url.mime_type.as_str() {
            "audio/flac" => "flac",
            "audio/mpeg" => "mp3",
            "audio/mp4" => "m4a",
            _ => "flac",
        };

        let filename = format!("{}_{}.{}", track_url.track_id, track_url.format_id, ext);
        let file_path = cache_dir.join(&filename);

        // Cache hit — read from disk
        if file_path.exists() {
            debug!("Track {} loaded from cache", track_url.track_id);
            return std::fs::read(&file_path)
                .map_err(|e| QobuzError::Api(format!("Failed to read cache file: {}", e)));
        }

        // Download entirely to memory first, then play without blocking on disk I/O
        info!("Streaming track {} into memory", track_url.track_id);
        let bytes = self.client.get(&track_url.url).send().await?.bytes().await?;
        let data = bytes.to_vec();

        // Save to disk cache in background so next play is instant
        let data_clone = data.clone();
        let partial_path = cache_dir.join(format!("{}.partial", filename));
        std::thread::spawn(move || {
            let _ = std::fs::create_dir_all(&cache_dir);
            if std::fs::write(&partial_path, &data_clone).is_ok() {
                let _ = std::fs::rename(&partial_path, &file_path);
                debug!("Track cached to disk for future plays");
            }
        });

        Ok(data)
    }

    // ── Browse / Search ──

    async fn api_get<T: serde::de::DeserializeOwned>(
        &self,
        endpoint: &str,
        params: &[(&str, &str)],
    ) -> Result<T, QobuzError> {
        let url = format!("{}/{}", QOBUZ_BASE_URL, endpoint);
        let resp = self
            .client
            .get(&url)
            .header("X-App-Id", &self.app_id)
            .header("X-User-Auth-Token", &self.user_auth_token)
            .query(params)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(QobuzError::Api(format!(
                "{} returned {}: {}",
                endpoint, status, text
            )));
        }

        resp.json()
            .await
            .map_err(|e| QobuzError::Parse(format!("{}: {}", endpoint, e)))
    }

    pub async fn search(&self, query: &str, limit: i32) -> Result<QobuzSearchResults, QobuzError> {
        self.api_get(
            "catalog/search",
            &[("query", query), ("limit", &limit.to_string())],
        )
        .await
    }

    pub async fn get_album(&self, album_id: &str) -> Result<QobuzAlbum, QobuzError> {
        self.api_get("album/get", &[("album_id", album_id)]).await
    }

    pub async fn get_track(&self, track_id: i64) -> Result<QobuzTrack, QobuzError> {
        self.api_get("track/get", &[("track_id", &track_id.to_string())]).await
    }

    pub async fn get_artist(&self, artist_id: i64) -> Result<QobuzArtist, QobuzError> {
        self.api_get(
            "artist/get",
            &[
                ("artist_id", &artist_id.to_string()),
                ("extra", "albums"),
                ("limit", "50"),
            ],
        )
        .await
    }

    pub async fn get_playlist(&self, playlist_id: i64) -> Result<QobuzPlaylist, QobuzError> {
        self.api_get(
            "playlist/get",
            &[
                ("playlist_id", &playlist_id.to_string()),
                ("extra", "tracks"),
                ("limit", "500"),
            ],
        )
        .await
    }

    pub async fn get_featured_albums(
        &self,
        genre_id: Option<&str>,
    ) -> Result<QobuzAlbumList, QobuzError> {
        let mut params = vec![("type", "new-releases"), ("limit", "50")];
        if let Some(gid) = genre_id {
            params.push(("genre_id", gid));
        }
        self.api_get("album/getFeatured", &params).await
    }

    pub async fn get_featured_playlists(&self) -> Result<QobuzPlaylistList, QobuzError> {
        self.api_get(
            "playlist/getFeatured",
            &[("type", "editor-picks"), ("limit", "50")],
        )
        .await
    }

    pub async fn get_genres(&self) -> Result<QobuzGenreList, QobuzError> {
        self.api_get("genre/list", &[]).await
    }

    // ── Favorites & User Library ──

    pub async fn get_user_playlists(&self) -> Result<QobuzPlaylistList, QobuzError> {
        let resp: QobuzUserPlaylistsResponse = self
            .api_get(
                "playlist/getUserPlaylists",
                &[
                    ("limit", "500"),
                    ("offset", "0"),
                    ("user_id", &self.user_id.to_string()),
                ],
            )
            .await?;
        Ok(resp.playlists)
    }

    pub async fn get_favorites(&self) -> Result<QobuzFavorites, QobuzError> {
        // Fetch liked tracks/albums/artists and user playlists concurrently.
        // Note: only pass "limit" — the reference client does NOT pass a "type" filter.
        let (fav_result, playlists_result) = tokio::join!(
            self.api_get::<QobuzFavorites>(
                "favorite/getUserFavorites",
                &[("limit", "500")],
            ),
            self.get_user_playlists()
        );

        match fav_result {
            Err(e) => {
                warn!("get_favorites failed: {}", e);
                Err(e)
            }
            Ok(mut fav) => {
                // Merge user playlists (soft-fail: don't break favorites if playlists endpoint errors)
                match playlists_result {
                    Ok(playlists) => fav.playlists = Some(playlists),
                    Err(e) => warn!("get_user_playlists failed (non-fatal): {}", e),
                }
                Ok(fav)
            }
        }
    }

    async fn api_post(&self, endpoint: &str, params: &[(&str, &str)]) -> Result<(), QobuzError> {
        let url = format!("{}/{}", QOBUZ_BASE_URL, endpoint);
        let resp = self
            .client
            .post(&url)
            .header("X-App-Id", &self.app_id)
            .header("X-User-Auth-Token", &self.user_auth_token)
            .form(params)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            return Err(QobuzError::Api(format!(
                "{} returned {}",
                endpoint, status
            )));
        }

        Ok(())
    }

    pub async fn add_favorite(&self, item_type: &str, item_id: &str) -> Result<(), QobuzError> {
        let key = match item_type {
            "track" => "track_ids",
            "album" => "album_ids",
            "artist" => "artist_ids",
            _ => return Err(QobuzError::Api("Invalid favorite type".into())),
        };
        self.api_post("favorite/create", &[(key, item_id)]).await
    }

    pub async fn remove_favorite(&self, item_type: &str, item_id: &str) -> Result<(), QobuzError> {
        let key = match item_type {
            "track" => "track_ids",
            "album" => "album_ids",
            "artist" => "artist_ids",
            _ => return Err(QobuzError::Api("Invalid favorite type".into())),
        };
        self.api_post("favorite/delete", &[(key, item_id)]).await
    }

    // ── Session persistence ──

    /// Restore a client from previously saved credentials (skips login flow)
    pub fn from_saved(saved: &crate::models::SavedSession) -> Self {
        let client = Client::builder()
            .cookie_store(true)
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .build()
            .expect("Failed to build HTTP client");
        Self {
            client,
            app_id: saved.app_id.clone(),
            active_secret: saved.active_secret.clone(),
            user_auth_token: saved.user_auth_token.clone(),
            user_id: saved.user_id,
            user_name: saved.user_name.clone(),
            subscription_label: saved.subscription_label.clone(),
        }
    }

    /// Serialize key fields for on-disk persistence
    pub fn to_saved(&self) -> crate::models::SavedSession {
        crate::models::SavedSession {
            app_id: self.app_id.clone(),
            active_secret: self.active_secret.clone(),
            user_auth_token: self.user_auth_token.clone(),
            user_id: self.user_id,
            user_name: self.user_name.clone(),
            subscription_label: self.subscription_label.clone(),
        }
    }

    // ── Session info ──

    pub fn session_info(&self) -> SessionInfo {
        SessionInfo {
            logged_in: true,
            user_name: Some(self.user_name.clone()),
            subscription: self.subscription_label.clone(),
        }
    }

    pub fn user_auth_token(&self) -> &str {
        &self.user_auth_token
    }

    pub fn app_id(&self) -> &str {
        &self.app_id
    }

    /// Fetch a short-lived JWT for proactive Qobuz Connect WebSocket connection.
    /// Returns `(ws_endpoint, ws_jwt)`.
    pub async fn get_connect_jwt(&self) -> Result<(String, String), QobuzError> {
        let url = format!("{}/qws/createToken", QOBUZ_BASE_URL);
        let resp = self
            .client
            .post(&url)
            .header("X-App-Id", &self.app_id)
            .header("X-User-Auth-Token", &self.user_auth_token)
            .form(&[
                ("jwt", "jwt_qws"),
                ("user_auth_token_needed", "true"),
                ("strong_auth_needed", "true"),
            ])
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(QobuzError::Api(format!(
                "createToken failed: {}",
                resp.status()
            )));
        }

        let data: JsonValue = resp
            .json()
            .await
            .map_err(|e| QobuzError::Parse(format!("createToken parse error: {e}")))?;

        let jwt_payload = data
            .get("jwt_qws")
            .ok_or_else(|| QobuzError::Parse("missing jwt_qws".into()))?;

        let endpoint = jwt_payload
            .get("endpoint")
            .and_then(|v| v.as_str())
            .unwrap_or("wss://play.qobuz.com/ws")
            .to_string();

        let jwt = jwt_payload
            .get("jwt")
            .and_then(|v| v.as_str())
            .ok_or_else(|| QobuzError::Parse("missing jwt".into()))?
            .to_string();

        Ok((endpoint, jwt))
    }
}
