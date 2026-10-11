use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{info, warn};

/// Maximum total memory size of font data buffers to load (default: 48 MB)
/// to maintain Vibecore's target 30-50 MB idle memory footprint.
pub const MAX_LOADED_FONT_BYTES: usize = 48 * 1024 * 1024;

/// Discovers candidate CJK font files across Linux, Windows, macOS,
/// as well as local application directories (`./fonts/` and `~/.config/vibecore/fonts/`).
pub fn discover_cjk_font_paths() -> Vec<PathBuf> {
    let mut candidate_paths = Vec::new();

    // 1. Check local application and user config font directories
    let local_fonts = PathBuf::from("fonts");
    if local_fonts.exists() {
        scan_dir_for_fonts(&local_fonts, &mut candidate_paths);
    }

    if let Some(proj_dirs) = directories::ProjectDirs::from("org", "vibecore", "vibecore") {
        let user_fonts = proj_dirs.config_dir().join("fonts");
        if user_fonts.exists() {
            scan_dir_for_fonts(&user_fonts, &mut candidate_paths);
        }
    }

    // 2. Platform-specific discovery
    #[cfg(target_os = "linux")]
    {
        discover_linux_cjk_fonts(&mut candidate_paths);
    }

    #[cfg(target_os = "windows")]
    {
        discover_windows_cjk_fonts(&mut candidate_paths);
    }

    #[cfg(target_os = "macos")]
    {
        discover_macos_cjk_fonts(&mut candidate_paths);
    }

    // Deduplicate paths while preserving order
    let mut seen = HashSet::new();
    let mut unique_paths = Vec::new();

    for path in candidate_paths {
        if let Ok(canonical) = fs::canonicalize(&path) {
            if seen.insert(canonical.clone()) {
                unique_paths.push(canonical);
            }
        } else if seen.insert(path.clone()) {
            unique_paths.push(path);
        }
    }

    unique_paths
}

fn scan_dir_for_fonts(dir: &Path, paths: &mut Vec<PathBuf>) {
    for entry in walkdir::WalkDir::new(dir)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let p = entry.path();
        if p.is_file() && is_font_file(p) {
            paths.push(p.to_path_buf());
        }
    }
}

#[cfg(target_os = "linux")]
fn discover_linux_cjk_fonts(paths: &mut Vec<PathBuf>) {
    let initial_count = paths.len();

    // Query fontconfig using fc-match for pan-CJK families and specific language fallbacks
    let fc_queries = [
        vec!["Noto Sans CJK JP", "-f", "%{file}###%{family}###%{lang}\n"],
        vec!["Noto Sans CJK SC", "-f", "%{file}###%{family}###%{lang}\n"],
        vec!["Noto Sans CJK TC", "-f", "%{file}###%{family}###%{lang}\n"],
        vec!["Noto Sans CJK KR", "-f", "%{file}###%{family}###%{lang}\n"],
        vec!["Noto Sans CJK HK", "-f", "%{file}###%{family}###%{lang}\n"],
        vec!["IBM Plex Sans JP", "-f", "%{file}###%{family}###%{lang}\n"],
        vec!["IBM Plex Sans SC", "-f", "%{file}###%{family}###%{lang}\n"],
        vec!["IBM Plex Sans TC", "-f", "%{file}###%{family}###%{lang}\n"],
        vec!["IBM Plex Sans KR", "-f", "%{file}###%{family}###%{lang}\n"],
        vec!["WenQuanYi Zen Hei", "-f", "%{file}###%{family}###%{lang}\n"],
        vec![
            "WenQuanYi Micro Hei",
            "-f",
            "%{file}###%{family}###%{lang}\n",
        ],
        vec![
            "Droid Sans Fallback",
            "-f",
            "%{file}###%{family}###%{lang}\n",
        ],
        vec![":lang=ja", "-f", "%{file}###%{family}###%{lang}\n"],
        vec![":lang=zh-cn", "-f", "%{file}###%{family}###%{lang}\n"],
        vec![":lang=zh-tw", "-f", "%{file}###%{family}###%{lang}\n"],
        vec![":lang=ko", "-f", "%{file}###%{family}###%{lang}\n"],
    ];

    for args in fc_queries {
        if let Ok(output) = Command::new("fc-match").args(&args).output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    let parts: Vec<&str> = line.split("###").collect();
                    if parts.is_empty() {
                        continue;
                    }
                    let file_path = parts[0].trim();
                    if file_path.is_empty() {
                        continue;
                    }

                    // Verify that the font actually supports CJK languages or is an explicit CJK font
                    let is_target_or_cjk = if parts.len() >= 2 && parts[1].contains("CJK") {
                        true
                    } else if parts.len() >= 3 {
                        let lang = parts[2];
                        lang.contains("ja")
                            || lang.contains("zh")
                            || lang.contains("ko")
                            || lang.contains("cmn")
                            || lang.contains("yue")
                    } else {
                        true
                    };

                    if is_target_or_cjk {
                        let path = PathBuf::from(file_path);
                        if path.exists() && is_font_file(&path) {
                            paths.push(path);
                        }
                    }
                }
            }
        }
    }

    // Fallback: If fontconfig query returned nothing new, scan standard font directories
    if paths.len() == initial_count {
        info!("Fontconfig returned no CJK fonts; falling back to directory scanning");
        let font_dirs = [
            "/etc/profiles/per-user",
            "/run/current-system/sw/share/fonts",
            "/run/current-system/sw/share/X11/fonts",
            "/usr/share/fonts",
            "/usr/local/share/fonts",
            "~/.local/share/fonts",
            "~/.fonts",
        ];

        let cjk_keywords = [
            "cjk",
            "noto-cjk",
            "notosanscjk",
            "ibmplexsanssc",
            "ibmplexsansjp",
            "ibmplexsanskr",
            "ibmplexsanstc",
            "wqy",
            "zenhei",
            "microhei",
            "sourcehancjk",
            "droidfallback",
        ];

        for dir_str in font_dirs {
            let expanded = if dir_str.starts_with("~/") {
                if let Ok(home) = std::env::var("HOME") {
                    PathBuf::from(dir_str.replacen("~", &home, 1))
                } else {
                    PathBuf::from(dir_str)
                }
            } else {
                PathBuf::from(dir_str)
            };

            if expanded.exists() {
                scan_dir_recursive_matching(&expanded, &cjk_keywords, paths, 4);
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn discover_windows_cjk_fonts(paths: &mut Vec<PathBuf>) {
    let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".to_string());
    let fonts_dir = PathBuf::from(windir).join("Fonts");

    let windows_fonts = [
        "msyh.ttc",     // Microsoft YaHei (Simplified Chinese)
        "msyhl.ttc",    // Microsoft YaHei Light
        "msyhbd.ttc",   // Microsoft YaHei Bold
        "msjh.ttc",     // Microsoft JhengHei (Traditional Chinese)
        "msjhl.ttc",    // Microsoft JhengHei Light
        "meiryo.ttc",   // Meiryo (Japanese)
        "msgothic.ttc", // MS Gothic (Japanese)
        "YuGothM.ttc",  // Yu Gothic (Japanese)
        "YuGothR.ttc",
        "malgun.ttf",   // Malgun Gothic (Korean)
        "malgunbd.ttf",
        "simsun.ttc",   // SimSun
        "mingliu.ttc",  // MingLiU
        "gulim.ttc",    // Gulim (Korean)
        "batang.ttc",   // Batang (Korean)
        "IBMPlexSansJP-Regular.ttf",
        "IBMPlexSansJP-Regular.otf",
        "IBMPlexSansSC-Regular.ttf",
        "IBMPlexSansSC-Regular.otf",
        "IBMPlexSansTC-Regular.ttf",
        "IBMPlexSansTC-Regular.otf",
        "IBMPlexSansKR-Regular.ttf",
        "IBMPlexSansKR-Regular.otf",
    ];

    for font_name in windows_fonts {
        let p = fonts_dir.join(font_name);
        if p.exists() {
            paths.push(p);
        }
    }
}

#[cfg(target_os = "macos")]
fn discover_macos_cjk_fonts(paths: &mut Vec<PathBuf>) {
    let macos_dirs = [
        "/System/Library/Fonts",
        "/System/Library/Fonts/Supplemental",
        "/Library/Fonts",
    ];

    let macos_fonts = [
        "PingFang.ttc",
        "Hiragino Sans GB.ttc",
        "AppleSDGothicNeo.ttc",
        "Arial Unicode.ttf",
        "STHeiti Medium.ttc",
        "Songti.ttc",
        "IBMPlexSansJP-Regular.otf",
        "IBMPlexSansJP-Regular.ttf",
        "IBMPlexSansSC-Regular.otf",
        "IBMPlexSansSC-Regular.ttf",
        "IBMPlexSansTC-Regular.otf",
        "IBMPlexSansTC-Regular.ttf",
        "IBMPlexSansKR-Regular.otf",
        "IBMPlexSansKR-Regular.ttf",
    ];

    for dir_str in macos_dirs {
        let dir = PathBuf::from(dir_str);
        if dir.exists() {
            for font_name in macos_fonts {
                let p = dir.join(font_name);
                if p.exists() {
                    paths.push(p);
                }
            }
        }
    }
}

fn scan_dir_recursive_matching(
    dir: &Path,
    keywords: &[&str],
    paths: &mut Vec<PathBuf>,
    max_depth: usize,
) {
    for entry in walkdir::WalkDir::new(dir)
        .max_depth(max_depth)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let p = entry.path();
        if entry.file_type().is_file() && is_font_file(p) {
            let filename_lower = p
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_lowercase();

            let matches_keyword = keywords.iter().any(|kw| filename_lower.contains(kw));
            if matches_keyword {
                paths.push(p.to_path_buf());
            }
        }
    }
}

fn is_font_file(path: &Path) -> bool {
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        matches!(ext.to_lowercase().as_str(), "ttf" | "otf" | "ttc" | "otc")
    } else {
        false
    }
}

/// Extracts all font family names contained in a font file using `ttf-parser`.
/// Works accurately for both single font files (.ttf, .otf) and font collections (.ttc, .otc).
pub fn extract_font_families(data: &[u8]) -> Vec<String> {
    let mut families = Vec::new();

    let count = ttf_parser::fonts_in_collection(data).unwrap_or(1);
    for index in 0..count {
        if let Ok(face) = ttf_parser::Face::parse(data, index) {
            let mut matched_family = None;
            for name in face.names() {
                // Name ID 1 is Font Family Name, Name ID 16 is Typographic Family Name
                if (name.name_id == 1 || name.name_id == 16) && name.is_unicode() {
                    if let Some(text) = name.to_string() {
                        let trimmed = text.trim();
                        if !trimmed.is_empty() {
                            matched_family = Some(trimmed.to_string());
                            break;
                        }
                    }
                }
            }
            if let Some(family) = matched_family {
                if !families.contains(&family) {
                    families.push(family);
                }
            }
        }
    }

    families
}

/// Queries the primary font family name for a given font file path.
/// First attempts parsing the font file headers directly via `ttf-parser`.
/// Falls back to `fc-query` on Linux and well-known filename heuristics.
pub fn query_font_family(path: &Path) -> Option<String> {
    // 1. Try reading the first few kilobytes or full file to parse the header
    if let Ok(file) = fs::File::open(path) {
        use std::io::Read;
        // Most TTF/OTF font name tables reside within the first 64KB
        let mut handle = file;
        let mut buffer = Vec::new();
        if handle.read_to_end(&mut buffer).is_ok() {
            let families = extract_font_families(&buffer);
            if let Some(first) = families.into_iter().next() {
                return Some(first);
            }
        }
    }

    // 2. Linux fc-query fallback
    #[cfg(target_os = "linux")]
    {
        if let Ok(output) = Command::new("fc-query")
            .arg(path)
            .args(["-f", "%{family}\n"])
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Some(first_line) = stdout.lines().next() {
                    let first_family = first_line.split(',').next().unwrap_or("").trim();
                    if !first_family.is_empty() {
                        return Some(first_family.to_string());
                    }
                }
            }
        }
    }

    // 3. Heuristic fallback based on well-known font file names
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_lowercase();

    if file_name.contains("ibmplexsansjp") {
        Some("IBM Plex Sans JP".to_string())
    } else if file_name.contains("ibmplexsanssc") {
        Some("IBM Plex Sans SC".to_string())
    } else if file_name.contains("ibmplexsanstc") {
        Some("IBM Plex Sans TC".to_string())
    } else if file_name.contains("ibmplexsanskr") {
        Some("IBM Plex Sans KR".to_string())
    } else if file_name.contains("ibmplexsans") || file_name.contains("ibm-plex-sans") {
        Some("IBM Plex Sans".to_string())
    } else if file_name.contains("notosanscjk") || file_name.contains("noto-cjk") {
        Some("Noto Sans CJK JP".to_string())
    } else if file_name.contains("msyh") {
        Some("Microsoft YaHei".to_string())
    } else if file_name.contains("msjh") {
        Some("Microsoft JhengHei".to_string())
    } else if file_name.contains("meiryo") {
        Some("Meiryo".to_string())
    } else if file_name.contains("msgothic") {
        Some("MS Gothic".to_string())
    } else if file_name.contains("yugoth") {
        Some("Yu Gothic".to_string())
    } else if file_name.contains("malgun") {
        Some("Malgun Gothic".to_string())
    } else if file_name.contains("pingfang") {
        Some("PingFang SC".to_string())
    } else if file_name.contains("hiragino") {
        Some("Hiragino Sans GB".to_string())
    } else {
        None
    }
}

/// Discovered CJK font bundle with raw byte data and optional primary family name.
pub struct CjkFontBundle {
    pub font_data: Vec<Vec<u8>>,
    pub primary_family: Option<String>,
}

/// Loads discovered CJK font files as raw byte buffers while strictly respecting
/// memory constraints (capped at `MAX_LOADED_FONT_BYTES`).
///
/// Discovered fonts are registered as fallback glyph sources in iced/cosmic-text.
/// Rather than forcing a specific Latin font, `primary_family` is set to `None` by default
/// so standard UI rendering uses clean system typography while CJK characters seamlessly fall back.
pub fn load_cjk_fonts() -> CjkFontBundle {
    let font_paths = discover_cjk_font_paths();
    let mut font_data = Vec::new();
    let mut total_bytes = 0usize;
    let mut discovered_families = Vec::new();

    info!("Discovered {} candidate font paths", font_paths.len());

    for path in font_paths {
        if total_bytes >= MAX_LOADED_FONT_BYTES {
            info!(
                "Reached font memory budget ({:.2} MB loaded), skipping remaining candidates",
                total_bytes as f64 / 1_048_576.0
            );
            break;
        }

        match fs::read(&path) {
            Ok(bytes) => {
                let size = bytes.len();
                // Check if loading this file would drastically exceed budget
                if total_bytes > 0 && total_bytes + size > MAX_LOADED_FONT_BYTES * 3 / 2 {
                    info!(
                        "Skipping large font {:?} ({:.2} MB) to protect memory budget",
                        path.file_name().unwrap_or_default(),
                        size as f64 / 1_048_576.0
                    );
                    continue;
                }

                let families = extract_font_families(&bytes);
                for fam in families {
                    if !discovered_families.contains(&fam) {
                        discovered_families.push(fam);
                    }
                }

                total_bytes += size;
                info!(
                    "Loaded font: {:?} ({:.2} MB) [Total: {:.2} MB]",
                    path.file_name().unwrap_or_default(),
                    size as f64 / 1_048_576.0,
                    total_bytes as f64 / 1_048_576.0
                );
                font_data.push(bytes);
            }
            Err(e) => {
                warn!("Failed to read font file {:?}: {}", path, e);
            }
        }
    }

    info!(
        "Font loading complete: {} buffers, {:.2} MB total memory",
        font_data.len(),
        total_bytes as f64 / 1_048_576.0
    );

    // Keep primary_family as None by default to let iced use system default typography,
    // while all loaded font buffers serve as cosmic-text glyph fallbacks.
    CjkFontBundle {
        font_data,
        primary_family: None,
    }
}

/// Loads all discovered CJK font files as raw byte buffers.
/// Preserved for backward compatibility and tests.
#[allow(dead_code)]
pub fn load_cjk_font_data() -> Vec<Vec<u8>> {
    load_cjk_fonts().font_data
}
