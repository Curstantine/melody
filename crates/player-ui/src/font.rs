use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{info, warn};

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

    // A. Primary: Query fontconfig using fc-match for IBM Plex Sans, IBM Plex CJK variants, and fallbacks
    let fc_queries = [
        vec!["IBM Plex Sans", "-f", "%{file}###%{family}###%{lang}\n"],
        vec!["IBM Plex Sans JP", "-f", "%{file}###%{family}###%{lang}\n"],
        vec!["IBM Plex Sans SC", "-f", "%{file}###%{family}###%{lang}\n"],
        vec!["IBM Plex Sans TC", "-f", "%{file}###%{family}###%{lang}\n"],
        vec!["IBM Plex Sans KR", "-f", "%{file}###%{family}###%{lang}\n"],
        vec!["Noto Sans CJK JP", "-f", "%{file}###%{family}###%{lang}\n"],
        vec!["Noto Sans CJK SC", "-f", "%{file}###%{family}###%{lang}\n"],
        vec!["Noto Sans CJK TC", "-f", "%{file}###%{family}###%{lang}\n"],
        vec!["Noto Sans CJK KR", "-f", "%{file}###%{family}###%{lang}\n"],
        vec!["Noto Sans CJK HK", "-f", "%{file}###%{family}###%{lang}\n"],
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

                    // Accept IBM Plex Sans directly, or verify that the font actually supports CJK languages
                    let is_target_or_cjk = if parts.len() >= 2 && parts[1].starts_with("IBM Plex Sans") {
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

    // B. Fallback: If fontconfig query returned nothing new, scan standard font directories
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
            "ibmplexsans",
            "ibm-plex-sans",
            "cjk",
            "noto-cjk",
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
        // IBM Plex Sans and related CJK fonts
        "IBMPlexSans-Regular.ttf",
        "IBMPlexSans-Regular.otf",
        "IBMPlexSansJP-Regular.ttf",
        "IBMPlexSansJP-Regular.otf",
        "IBMPlexSansSC-Regular.ttf",
        "IBMPlexSansSC-Regular.otf",
        "IBMPlexSansTC-Regular.ttf",
        "IBMPlexSansTC-Regular.otf",
        "IBMPlexSansKR-Regular.ttf",
        "IBMPlexSansKR-Regular.otf",
        // Windows system CJK fonts fallback
        "msyh.ttc",     // Microsoft YaHei (Simplified Chinese)
        "msyhl.ttc",    // Microsoft YaHei Light
        "msyhbd.ttc",   // Microsoft YaHei Bold
        "msjh.ttc",     // Microsoft JhengHei (Traditional Chinese)
        "msjhl.ttc",    // Microsoft JhengHei Light
        "meiryo.ttc",   // Meiryo (Japanese)
        "msgothic.ttc", // MS Gothic (Japanese)
        "YuGothM.ttc",  // Yu Gothic (Japanese)
        "YuGothR.ttc",
        "malgun.ttf", // Malgun Gothic (Korean)
        "malgunbd.ttf",
        "simsun.ttc",  // SimSun
        "mingliu.ttc", // MingLiU
        "gulim.ttc",   // Gulim (Korean)
        "batang.ttc",  // Batang (Korean)
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
        // IBM Plex Sans and related CJK fonts
        "IBMPlexSans-Regular.otf",
        "IBMPlexSans-Regular.ttf",
        "IBMPlexSansJP-Regular.otf",
        "IBMPlexSansJP-Regular.ttf",
        "IBMPlexSansSC-Regular.otf",
        "IBMPlexSansSC-Regular.ttf",
        "IBMPlexSansTC-Regular.otf",
        "IBMPlexSansTC-Regular.ttf",
        "IBMPlexSansKR-Regular.otf",
        "IBMPlexSansKR-Regular.ttf",
        // macOS system CJK fonts fallback
        "PingFang.ttc",
        "Hiragino Sans GB.ttc",
        "AppleSDGothicNeo.ttc",
        "Arial Unicode.ttf",
        "STHeiti Medium.ttc",
        "Songti.ttc",
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

/// Queries the primary font family name for a given font file path via fc-query or filename heuristic.
pub fn query_font_family(path: &Path) -> Option<String> {
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

    // Heuristic fallback based on well-known font file names
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

/// Loads all discovered CJK font files as raw byte buffers and determines
/// the primary font family name to use as default font (prioritizing IBM Plex Sans).
pub fn load_cjk_fonts() -> CjkFontBundle {
    let font_paths = discover_cjk_font_paths();
    let mut font_data = Vec::new();
    let mut discovered_families = Vec::new();

    info!("Discovered {} candidate font paths", font_paths.len());

    for path in font_paths {
        if let Some(family) = query_font_family(&path) {
            discovered_families.push(family);
        }

        match fs::read(&path) {
            Ok(bytes) => {
                info!(
                    "Loaded font: {:?} ({:.2} MB)",
                    path.file_name().unwrap_or_default(),
                    bytes.len() as f64 / 1_048_576.0
                );
                font_data.push(bytes);
            }
            Err(e) => {
                warn!("Failed to read font file {:?}: {}", path, e);
            }
        }
    }

    // Prioritize IBM Plex Sans as the default font family, followed by IBM Plex Sans CJK variants,
    // and finally any first discovered CJK font.
    let primary_family = if discovered_families.iter().any(|f| f == "IBM Plex Sans") {
        Some("IBM Plex Sans".to_string())
    } else if let Some(plex_cjk) = discovered_families
        .iter()
        .find(|f| f.starts_with("IBM Plex Sans"))
    {
        Some(plex_cjk.clone())
    } else {
        discovered_families.into_iter().next()
    };

    if let Some(ref fam) = primary_family {
        info!("Selected primary default font family: {}", fam);
    }

    CjkFontBundle {
        font_data,
        primary_family,
    }
}

/// Loads all discovered CJK font files as raw byte buffers.
/// Preserved for backward compatibility and tests.
#[allow(dead_code)]
pub fn load_cjk_font_data() -> Vec<Vec<u8>> {
    load_cjk_fonts().font_data
}
