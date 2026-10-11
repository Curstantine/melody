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
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() {
                if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                    let ext_lower = ext.to_lowercase();
                    if matches!(ext_lower.as_str(), "ttf" | "otf" | "ttc" | "otc") {
                        paths.push(p);
                    }
                }
            }
        }
    }
}

#[cfg(target_os = "linux")]
fn discover_linux_cjk_fonts(paths: &mut Vec<PathBuf>) {
    // A. Query fontconfig using fc-match for CJK languages and families
    let fc_queries = [
        vec![":lang=zh", "-f", "%{file}\n"],
        vec![":lang=ja", "-f", "%{file}\n"],
        vec![":lang=ko", "-f", "%{file}\n"],
        vec!["Noto Sans CJK SC", "-f", "%{file}\n"],
        vec!["Noto Sans CJK TC", "-f", "%{file}\n"],
        vec!["Noto Sans CJK JP", "-f", "%{file}\n"],
        vec!["Noto Sans CJK KR", "-f", "%{file}\n"],
        vec!["IBM Plex Sans SC", "-f", "%{file}\n"],
        vec!["IBM Plex Sans TC", "-f", "%{file}\n"],
        vec!["IBM Plex Sans JP", "-f", "%{file}\n"],
        vec!["IBM Plex Sans KR", "-f", "%{file}\n"],
        vec!["WenQuanYi Zen Hei", "-f", "%{file}\n"],
        vec!["WenQuanYi Micro Hei", "-f", "%{file}\n"],
        vec!["Droid Sans Fallback", "-f", "%{file}\n"],
    ];

    for args in fc_queries {
        if let Ok(output) = Command::new("fc-match").args(&args).output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        let path = PathBuf::from(trimmed);
                        if path.exists() && is_font_file(&path) {
                            paths.push(path);
                        }
                    }
                }
            }
        }
    }

    // B. Scan standard font directories for CJK patterns
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
        "cjk", "noto-cjk", "ibmplexsanssc", "ibmplexsansjp", "ibmplexsanskr", "ibmplexsanstc",
        "wqy", "zenhei", "microhei", "sourcehancjk", "droidfallback",
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

#[cfg(target_os = "windows")]
fn discover_windows_cjk_fonts(paths: &mut Vec<PathBuf>) {
    let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".to_string());
    let fonts_dir = PathBuf::from(windir).join("Fonts");

    let windows_cjk = [
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
    ];

    for font_name in windows_cjk {
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

    let macos_cjk = [
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
            for font_name in macos_cjk {
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
    if max_depth == 0 {
        return;
    }

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                scan_dir_recursive_matching(&p, keywords, paths, max_depth - 1);
            } else if p.is_file() && is_font_file(&p) {
                let filename_lower = p
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_lowercase();

                let matches_keyword = keywords.iter().any(|kw| filename_lower.contains(kw));
                if matches_keyword {
                    paths.push(p);
                }
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

/// Loads all discovered CJK font files as raw byte buffers.
/// Returns a vector of font byte vectors ready to be fed to Iced.
pub fn load_cjk_font_data() -> Vec<Vec<u8>> {
    let font_paths = discover_cjk_font_paths();
    let mut fonts = Vec::new();

    info!("Discovered {} candidate CJK font paths", font_paths.len());

    for path in font_paths {
        match fs::read(&path) {
            Ok(bytes) => {
                info!(
                    "Loaded CJK font: {:?} ({:.2} MB)",
                    path.file_name().unwrap_or_default(),
                    bytes.len() as f64 / 1_048_576.0
                );
                fonts.push(bytes);
            }
            Err(e) => {
                warn!("Failed to read font file {:?}: {}", path, e);
            }
        }
    }

    fonts
}
