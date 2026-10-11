use serde::{Deserialize, Serialize};

/// Dominant and accent color palette extracted from an album cover art image.
/// Colors are stored as normalized RGBA floats `[r, g, b, a]` in range `0.0..=1.0`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ExtractedColors {
    /// Most prominent vibrant color in the artwork
    pub primary: [f32; 4],
    /// Secondary contrasting or supporting tone
    pub secondary: [f32; 4],
    /// Third accent/highlight tone
    pub accent: [f32; 4],
    /// Deep ambient backdrop tone (ideal for canvas backgrounds and dark cards)
    pub background: [f32; 4],
}

impl Default for ExtractedColors {
    fn default() -> Self {
        Self {
            primary: [0.48, 0.38, 0.94, 1.0],    // Vibrant electric indigo
            secondary: [0.92, 0.30, 0.62, 1.0],  // Vivid magenta
            accent: [0.20, 0.82, 0.88, 1.0],     // Luminous cyan
            background: [0.08, 0.08, 0.13, 1.0], // Deep ambient navy/charcoal
        }
    }
}

#[derive(Clone, Copy)]
struct ColorBucket {
    r_sum: u64,
    g_sum: u64,
    b_sum: u64,
    count: usize,
}

/// Extracts a 4-color palette (primary, secondary, accent, background) from image bytes.
/// If decoding fails or no image is present, returns `ExtractedColors::default()`.
pub fn extract_palette(image_bytes: &[u8]) -> ExtractedColors {
    if image_bytes.is_empty() {
        return ExtractedColors::default();
    }

    let img = match image::load_from_memory(image_bytes) {
        Ok(img) => img,
        Err(_) => return ExtractedColors::default(),
    };

    // Downsample to 48x48 for fast analysis (<1ms)
    let thumb = img.thumbnail_exact(48, 48).to_rgb8();

    // Quantize into 8x8x8 = 512 color buckets
    let mut buckets = vec![
        ColorBucket {
            r_sum: 0,
            g_sum: 0,
            b_sum: 0,
            count: 0
        };
        512
    ];

    for pixel in thumb.pixels() {
        let r = pixel[0] as usize;
        let g = pixel[1] as usize;
        let b = pixel[2] as usize;

        let r_bin = (r >> 5).min(7);
        let g_bin = (g >> 5).min(7);
        let b_bin = (b >> 5).min(7);
        let idx = (r_bin << 6) | (g_bin << 3) | b_bin;

        buckets[idx].r_sum += r as u64;
        buckets[idx].g_sum += g as u64;
        buckets[idx].b_sum += b as u64;
        buckets[idx].count += 1;
    }

    // Collect non-empty buckets and compute average RGB & score
    struct Candidate {
        rgb: [f32; 3],
        score: f32,
        brightness: f32,
    }

    let mut candidates: Vec<Candidate> = buckets
        .into_iter()
        .filter(|b| b.count > 0)
        .map(|b| {
            let r = (b.r_sum as f32 / b.count as f32) / 255.0;
            let g = (b.g_sum as f32 / b.count as f32) / 255.0;
            let b_val = (b.b_sum as f32 / b.count as f32) / 255.0;

            let max_c = r.max(g).max(b_val);
            let min_c = r.min(g).min(b_val);
            let saturation = if max_c > 0.001 {
                (max_c - min_c) / max_c
            } else {
                0.0
            };
            let brightness = 0.299 * r + 0.587 * g + 0.114 * b_val;

            // Score boosts saturation and moderate brightness
            let mut score = b.count as f32 * (saturation * 2.5 + 0.5);
            // Penalize near-pure white or near-pure black for primary accent picking
            if brightness > 0.90 || brightness < 0.10 {
                score *= 0.25;
            }

            Candidate {
                rgb: [r, g, b_val],
                score,
                brightness,
            }
        })
        .collect();

    if candidates.is_empty() {
        return ExtractedColors::default();
    }

    candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    let color_distance = |a: [f32; 3], b: [f32; 3]| -> f32 {
        let dr = a[0] - b[0];
        let dg = a[1] - b[1];
        let db = a[2] - b[2];
        (dr * dr + dg * dg + db * db).sqrt()
    };

    let primary_rgb = candidates[0].rgb;

    // Pick secondary: distinct from primary
    let secondary_rgb = candidates
        .iter()
        .skip(1)
        .find(|c| color_distance(c.rgb, primary_rgb) > 0.25)
        .map(|c| c.rgb)
        .unwrap_or_else(|| {
            // Shift primary hue/saturation if no distinct cluster found
            [
                (primary_rgb[1] * 0.8 + 0.2).min(1.0),
                (primary_rgb[2] * 0.8 + 0.2).min(1.0),
                (primary_rgb[0] * 0.8 + 0.2).min(1.0),
            ]
        });

    // Pick accent: distinct from both
    let accent_rgb = candidates
        .iter()
        .skip(1)
        .find(|c| {
            color_distance(c.rgb, primary_rgb) > 0.25 && color_distance(c.rgb, secondary_rgb) > 0.25
        })
        .map(|c| c.rgb)
        .unwrap_or_else(|| {
            [
                (secondary_rgb[2] * 0.7 + 0.3).min(1.0),
                (secondary_rgb[0] * 0.7 + 0.3).min(1.0),
                (secondary_rgb[1] * 0.7 + 0.3).min(1.0),
            ]
        });

    // Ambient background: deep, rich, low-lightness backdrop
    // Find darkest candidate or darken primary
    let bg_candidate = candidates
        .iter()
        .min_by(|a, b| a.brightness.partial_cmp(&b.brightness).unwrap_or(std::cmp::Ordering::Equal));

    let background_rgb = match bg_candidate {
        Some(bg) if bg.brightness <= 0.20 && bg.brightness >= 0.04 => [
            bg.rgb[0] * 0.85,
            bg.rgb[1] * 0.85,
            bg.rgb[2] * 0.85,
        ],
        _ => [
            (primary_rgb[0] * 0.15 + 0.05).min(0.20),
            (primary_rgb[1] * 0.15 + 0.05).min(0.20),
            (primary_rgb[2] * 0.15 + 0.07).min(0.22),
        ],
    };

    ExtractedColors {
        primary: [primary_rgb[0], primary_rgb[1], primary_rgb[2], 1.0],
        secondary: [secondary_rgb[0], secondary_rgb[1], secondary_rgb[2], 1.0],
        accent: [accent_rgb[0], accent_rgb[1], accent_rgb[2], 1.0],
        background: [background_rgb[0], background_rgb[1], background_rgb[2], 1.0],
    }
}
