use crate::models::*;
use lofty::prelude::*;
use lofty::probe::Probe;
use std::path::Path;
use tracing::{debug, info, warn};
use walkdir::WalkDir;

const SUPPORTED_EXTENSIONS: &[&str] = &["flac", "mp3", "m4a", "aac", "ogg", "wav", "aiff", "wma"];

/// Scan a directory for local music files and extract metadata
pub fn scan_directory(dir: &Path) -> Vec<LocalTrack> {
    let mut tracks = Vec::new();

    for entry in WalkDir::new(dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();

        if !SUPPORTED_EXTENSIONS.contains(&ext.as_str()) {
            continue;
        }

        match read_track_metadata(path) {
            Ok(track) => {
                debug!("Found local track: {} - {}", track.artist, track.title);
                tracks.push(track);
            }
            Err(e) => {
                warn!("Failed to read metadata for {:?}: {}", path, e);
            }
        }
    }

    info!("Scanned {} tracks from {:?}", tracks.len(), dir);
    tracks
}

/// Read metadata from a single audio file
fn read_track_metadata(path: &Path) -> Result<LocalTrack, String> {
    let tagged_file = Probe::open(path)
        .map_err(|e| format!("Failed to open: {}", e))?
        .read()
        .map_err(|e| format!("Failed to read tags: {}", e))?;

    let properties = tagged_file.properties();
    let duration_seconds = properties.duration().as_secs() as i32;
    let sample_rate = properties.sample_rate();
    let bit_depth = properties.bit_depth().map(|b| b as u16);

    let tag = tagged_file.primary_tag().or_else(|| tagged_file.first_tag());

    let title = tag
        .and_then(|t| t.title().map(|s| s.to_string()))
        .unwrap_or_else(|| {
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Unknown")
                .to_string()
        });

    let artist = tag
        .and_then(|t| t.artist().map(|s| s.to_string()))
        .unwrap_or_else(|| "Unknown Artist".to_string());

    let album = tag
        .and_then(|t| t.album().map(|s| s.to_string()))
        .unwrap_or_else(|| "Unknown Album".to_string());

    let track_number = tag.and_then(|t| t.track().map(|n| n as i32));

    // Extract cover art as base64
    let cover_data = tag.and_then(|t| {
        t.pictures().first().map(|pic| {
            use base64::Engine;
            let mime = match pic.mime_type() {
                Some(lofty::picture::MimeType::Jpeg) => "image/jpeg",
                Some(lofty::picture::MimeType::Png) => "image/png",
                _ => "image/jpeg",
            };
            let b64 = base64::engine::general_purpose::STANDARD.encode(pic.data());
            format!("data:{};base64,{}", mime, b64)
        })
    });

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("unknown")
        .to_uppercase();

    Ok(LocalTrack {
        file_path: path.to_string_lossy().to_string(),
        title,
        artist,
        album,
        duration_seconds,
        track_number,
        cover_data,
        sample_rate,
        bit_depth,
        format: ext,
    })
}

/// Convert a LocalTrack to a UnifiedTrack
pub fn local_to_unified(local: &LocalTrack) -> UnifiedTrack {
    let quality_label = match (&local.sample_rate, &local.bit_depth) {
        (Some(sr), Some(bd)) => Some(format!("{}-bit/{}kHz {}", bd, *sr as f64 / 1000.0, local.format)),
        _ => Some(local.format.clone()),
    };

    UnifiedTrack {
        id: format!("local_{}", uuid::Uuid::new_v4()),
        title: local.title.clone(),
        artist: local.artist.clone(),
        album: local.album.clone(),
        duration_seconds: local.duration_seconds,
        cover_url: local.cover_data.clone(),
        source: TrackSource::Local {
            file_path: local.file_path.clone(),
        },
        quality_label,
        sample_rate: local.sample_rate.map(|s| s as f64),
        bit_depth: local.bit_depth.map(|b| b as i32),
    }
}
