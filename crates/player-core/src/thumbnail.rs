use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::Cursor;
use std::path::{Path, PathBuf};

use directories::ProjectDirs;
use image::ImageOutputFormat;
use tracing::debug;

use crate::metadata::extract_cover_art;

/// Default max dimension for cached album thumbnails
pub const DEFAULT_THUMBNAIL_SIZE: u32 = 200;

/// Returns the platform-specific directory where cover thumbnails are cached on disk
pub fn thumbnail_cache_dir() -> PathBuf {
    if let Some(proj) = ProjectDirs::from("org", "vibecore", "vibecore") {
        proj.cache_dir().join("thumbnails")
    } else {
        PathBuf::from(".cache/thumbnails")
    }
}

/// Generates a deterministic hash-based cache key for a given track path and thumbnail dimension
pub fn compute_thumbnail_cache_key(track_path: &Path, max_dim: u32) -> String {
    let mut hasher = DefaultHasher::new();
    "v2_square".hash(&mut hasher);
    track_path.to_string_lossy().hash(&mut hasher);
    max_dim.hash(&mut hasher);

    if let Ok(meta) = fs::metadata(track_path) {
        meta.len().hash(&mut hasher);
        if let Ok(mtime) = meta.modified() {
            mtime.hash(&mut hasher);
        }
    }

    format!("{:016x}_{}", hasher.finish(), max_dim)
}

/// Retrieves an existing thumbnail from disk cache, or extracts, downscales, and caches it.
/// Returns small JPEG bytes (~8-15 KB) suitable for smooth GPU texture rendering.
pub fn get_or_create_thumbnail(track_path: &Path, max_dim: u32) -> Option<Vec<u8>> {
    let cache_dir = thumbnail_cache_dir();
    let cache_key = compute_thumbnail_cache_key(track_path, max_dim);

    let thumb_file = cache_dir.join(format!("{}.jpg", cache_key));
    let none_marker = cache_dir.join(format!("{}.none", cache_key));

    // 1. Fast hit: cached thumbnail already exists on disk
    if thumb_file.is_file() {
        if let Ok(bytes) = fs::read(&thumb_file) {
            if !bytes.is_empty() {
                return Some(bytes);
            }
        }
    }

    // Fast negative hit: we already checked this file previously and it has no cover art
    if none_marker.is_file() {
        return None;
    }

    // 2. Extract original cover art bytes from audio container or folder
    let original_bytes = match extract_cover_art(track_path) {
        Some(bytes) if !bytes.is_empty() => bytes,
        _ => {
            // Write negative marker to prevent re-probing expensive container metadata
            let _ = fs::create_dir_all(&cache_dir);
            let _ = fs::write(&none_marker, b"");
            return None;
        }
    };

    // 3. Decode image and downscale to thumbnail dimensions
    match image::load_from_memory(&original_bytes) {
        Ok(img) => {
            // Resize to fill square dimensions and center-crop so the thumbnail is always strictly square (1:1),
            // avoiding non-square aspect ratio overflow that triggers separate clipping layers in Iced.
            let thumb = img.resize_to_fill(max_dim, max_dim, image::imageops::FilterType::Triangle);
            let rgb = image::DynamicImage::ImageRgb8(thumb.to_rgb8());

            let mut out_bytes = Vec::new();
            if rgb
                .write_to(
                    &mut Cursor::new(&mut out_bytes),
                    ImageOutputFormat::Jpeg(80),
                )
                .is_ok()
            {
                let _ = fs::create_dir_all(&cache_dir);
                let _ = fs::write(&thumb_file, &out_bytes);
                debug!(
                    "Generated thumbnail for {:?} ({} bytes)",
                    track_path,
                    out_bytes.len()
                );
                Some(out_bytes)
            } else {
                // If encoding failed, fallback to original if reasonably small
                if original_bytes.len() < 300_000 {
                    Some(original_bytes)
                } else {
                    None
                }
            }
        }
        Err(e) => {
            debug!("Failed to decode cover image for {:?}: {}", track_path, e);
            if original_bytes.len() < 300_000 {
                Some(original_bytes)
            } else {
                None
            }
        }
    }
}

/// Disk cache summary statistics for cover thumbnails
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CacheStats {
    pub file_count: usize,
    pub total_bytes: u64,
}

/// Calculates the number of thumbnail cache files and their total footprint in bytes
pub fn get_thumbnail_cache_stats() -> CacheStats {
    let dir = thumbnail_cache_dir();
    let mut file_count = 0;
    let mut total_bytes = 0;

    if dir.is_dir() {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                if let Ok(meta) = entry.metadata() {
                    if meta.is_file() {
                        file_count += 1;
                        total_bytes += meta.len();
                    }
                }
            }
        }
    }

    CacheStats {
        file_count,
        total_bytes,
    }
}

/// Deletes all cached thumbnail files and negative lookup markers from disk
pub fn clear_thumbnail_cache() -> std::io::Result<usize> {
    let dir = thumbnail_cache_dir();
    let mut removed = 0;

    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                if fs::remove_file(entry.path()).is_ok() {
                    removed += 1;
                }
            }
        }
    }

    Ok(removed)
}
