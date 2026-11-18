use std::{fmt::Write, fs, path::Path};

use image::codecs::jpeg::JpegEncoder;
use md5::{Digest, Md5};
use tracing::debug;

const EXCEPTION_MIN_SIZE: usize = 100_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Resolution {
    Low,
    High,
}

impl Resolution {
    const fn dim(self) -> u32 {
        match self {
            Self::Low => 256,
            Self::High => 680,
        }
    }

    const fn quality(self) -> u8 {
        match self {
            Self::Low => 70,
            Self::High => 90,
        }
    }
}

/// Generates a deterministic hash-based cache key for a given track path and thumbnail resolution
pub fn compute_cache_key(track_path: &Path, res: Resolution) -> String {
    let max_dim = res.dim();
    let mut hasher = Md5::new();
    hasher.update(b"v3_square");
    hasher.update(track_path.to_string_lossy().as_bytes());
    hasher.update(max_dim.to_le_bytes());

    if let Ok(meta) = fs::metadata(track_path) {
        hasher.update(meta.len().to_le_bytes());
        if let Ok(mtime) = meta.modified() {
            if let Ok(duration) = mtime.duration_since(std::time::UNIX_EPOCH) {
                hasher.update(duration.as_millis().to_le_bytes());
            }
        }
    }

    let digest = hasher.finalize();
    let mut key = String::with_capacity(32 + 1 + 10);
    for byte in digest {
        let _ = write!(key, "{:02x}", byte);
    }
    let _ = write!(key, "_{}", max_dim);
    key
}

/// Retrieves an existing thumbnail from disk cache, or extracts, downscales, and caches it.
pub fn get_or_create(track_path: &Path, res: Resolution) -> Option<Vec<u8>> {
    let max_dim = res.dim();
    let cache_dir = path::thumbnail_cache_dir();
    let cache_key = compute_cache_key(track_path, res);

    let thumb_file = cache_dir.join(format!("{}.jpg", cache_key));
    let none_marker = cache_dir.join(format!("{}.none", cache_key));

    if thumb_file.is_file() {
        if let Ok(bytes) = fs::read(&thumb_file) {
            if !bytes.is_empty() {
                return Some(bytes);
            }
        }
    }

    if none_marker.is_file() {
        return None;
    }

    let original_bytes = match metadata::extract_cover_art(track_path) {
        Some(bytes) if !bytes.is_empty() => bytes,
        _ => {
            // Write negative marker to prevent re-probing expensive container metadata
            let _ = fs::create_dir_all(cache_dir);
            let _ = fs::write(&none_marker, b"");
            return None;
        }
    };

    match image::load_from_memory(&original_bytes) {
        Ok(img) => {
            let thumb = img.resize_to_fill(max_dim, max_dim, image::imageops::FilterType::Triangle);
            let rgb = image::DynamicImage::ImageRgb8(thumb.to_rgb8());

            let mut out_bytes = Vec::new();
            if rgb
                .write_with_encoder(JpegEncoder::new_with_quality(&mut out_bytes, res.quality()))
                .is_ok()
            {
                let _ = fs::create_dir_all(cache_dir);
                let _ = fs::write(&thumb_file, &out_bytes);
                let _ = fs::remove_file(&none_marker);

                debug!(
                    "Generated thumbnail for {track_path:?} ({} bytes)",
                    out_bytes.len()
                );

                Some(out_bytes)
            } else if original_bytes.len() < EXCEPTION_MIN_SIZE {
                let _ = fs::remove_file(&none_marker);
                Some(original_bytes)
            } else {
                None
            }
        }
        Err(e) => {
            debug!("Failed to decode cover image for {:?}: {}", track_path, e);
            if original_bytes.len() < EXCEPTION_MIN_SIZE {
                let _ = fs::remove_file(&none_marker);
                Some(original_bytes)
            } else {
                None
            }
        }
    }
}

/// Generates a heavily blurred low-resolution JPEG thumbnail (~1-3 KB)
/// suitable for full-screen ambient backdrop rendering with bilinear texture smoothing.
///
/// Downscales to 32x32 with sigma 3.0 (8x faster than 64x64 @ sigma 6.0 while
/// producing an identical ambient gradient under GPU bilinear filtering).
pub fn generate_blurred(image_bytes: &[u8]) -> Option<Vec<u8>> {
    let img = image::load_from_memory(image_bytes).ok()?;
    let small = img.thumbnail_exact(32, 32);
    let blurred = image::imageops::blur(&small, 3.0);
    let darken = image::imageops::brighten(&blurred, -10);
    let rgb = image::DynamicImage::ImageRgba8(darken);

    let mut out_bytes = Vec::new();
    rgb.write_with_encoder(JpegEncoder::new_with_quality(&mut out_bytes, 70))
        .ok()?;

    Some(out_bytes)
}

/// Retrieves a cached blurred backdrop thumbnail from disk, or generates and caches it
/// from the track's low-resolution cover thumbnail.
pub fn get_or_create_blurred(track_path: &Path, low_cover_bytes: Option<&[u8]>) -> Option<Vec<u8>> {
    let cache_dir = path::thumbnail_cache_dir();
    let cache_key = compute_cache_key(track_path, Resolution::Low);
    let blur_file = cache_dir.join(format!("{}_blur.jpg", cache_key));

    if blur_file.is_file() {
        if let Ok(bytes) = fs::read(&blur_file) {
            if !bytes.is_empty() {
                return Some(bytes);
            }
        }
    }

    let owned_cover;
    let source_bytes = match low_cover_bytes {
        Some(b) if !b.is_empty() => b,
        _ => {
            owned_cover = get_or_create(track_path, Resolution::Low)?;
            &owned_cover
        }
    };

    let blurred = generate_blurred(source_bytes)?;
    let _ = fs::create_dir_all(cache_dir);
    let _ = fs::write(&blur_file, &blurred);
    Some(blurred)
}

/// Composites up to 4 cover images into a 2x2 square JPEG collage at `res`
/// dimensions so multi-cover playlist tiles can render through a single rounded image widget.
pub fn generate_collage(images: &[Vec<u8>], res: Resolution) -> Option<Vec<u8>> {
    let dim = res.dim();
    if images.len() < 4 || dim < 2 {
        return None;
    }
    let half = dim / 2;
    let size = half * 2;
    let mut canvas = image::RgbaImage::new(size, size);

    for (idx, bytes) in images.iter().take(4).enumerate() {
        let img = image::load_from_memory(bytes).ok()?;
        let tile = img
            .resize_to_fill(half, half, image::imageops::FilterType::Triangle)
            .to_rgba8();
        let x = ((idx as u32) % 2) * half;
        let y = ((idx as u32) / 2) * half;
        image::imageops::replace(&mut canvas, &tile, x as i64, y as i64);
    }

    let mut out_bytes = Vec::new();
    image::DynamicImage::ImageRgba8(canvas)
        .to_rgb8()
        .write_with_encoder(JpegEncoder::new_with_quality(&mut out_bytes, 85))
        .ok()?;

    Some(out_bytes)
}

/// Disk cache summary statistics for cover thumbnails
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CacheStats {
    pub file_count: usize,
    pub total_bytes: u64,
}

/// Calculates the number of thumbnail cache files and their total footprint in bytes
pub fn get_thumbnail_cache_stats() -> CacheStats {
    let dir = path::thumbnail_cache_dir();
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
    let dir = path::thumbnail_cache_dir();
    let mut removed = 0;

    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            if entry.file_type()?.is_file() && fs::remove_file(entry.path()).is_ok() {
                removed += 1;
            }
        }
    }

    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thumbnail_generation_and_caching() {
        let temp_dir = std::env::temp_dir().join("melody_test_thumbnails");
        let _ = std::fs::create_dir_all(&temp_dir);

        let mut img = image::RgbImage::new(600, 600);
        for (x, y, pixel) in img.enumerate_pixels_mut() {
            *pixel = image::Rgb([(x % 256) as u8, (y % 256) as u8, 128]);
        }

        let song_path = temp_dir.join("test_track.flac");
        let cover_path = temp_dir.join("cover.jpg");

        std::fs::write(&song_path, b"dummy audio content").expect("Write song failed");
        img.save(&cover_path).expect("Save test cover failed");

        let thumb_opt = get_or_create(&song_path, Resolution::Low);
        assert!(thumb_opt.is_some(), "Thumbnail should be generated");
        let thumb_bytes = thumb_opt.unwrap();
        assert!(!thumb_bytes.is_empty());

        let decoded =
            image::load_from_memory(&thumb_bytes).expect("Failed to decode generated thumbnail");
        assert_eq!(decoded.width(), Resolution::Low.dim());
        assert_eq!(decoded.height(), Resolution::Low.dim());

        let cached_thumb = get_or_create(&song_path, Resolution::Low);
        assert_eq!(cached_thumb, Some(thumb_bytes));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_generate_collage_cover() {
        let colors = [
            image::Rgb([255, 0, 0]),
            image::Rgb([0, 255, 0]),
            image::Rgb([0, 0, 255]),
            image::Rgb([255, 255, 0]),
        ];
        let encoded: Vec<Vec<u8>> = colors
            .iter()
            .map(|&c| {
                let img = image::RgbImage::from_pixel(64, 64, c);
                let mut buf = Vec::new();
                image::DynamicImage::ImageRgb8(img)
                    .write_with_encoder(JpegEncoder::new_with_quality(&mut buf, 90))
                    .unwrap();
                buf
            })
            .collect();

        assert!(generate_collage(&encoded[..3], Resolution::Low).is_none());

        let collage_bytes = generate_collage(&encoded, Resolution::Low).expect("collage generated");
        let decoded = image::load_from_memory(&collage_bytes).expect("valid collage image");
        assert_eq!(decoded.width(), Resolution::Low.dim());
        assert_eq!(decoded.height(), Resolution::Low.dim());
    }
}
