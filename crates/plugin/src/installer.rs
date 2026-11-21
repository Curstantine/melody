use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use tracing::{info, warn};
use wasmi::{Engine, Module};

use crate::manifest::{validate_extension_id, ExtensionManifest, ExtensionOrigin};
use crate::runtime::audit_module_imports;

const MAX_ARCHIVE_ENTRIES: usize = 256;
const MAX_ARCHIVE_TOTAL_BYTES: u64 = 32 * 1024 * 1024;
const MAX_WASM_MODULE_BYTES: u64 = 16 * 1024 * 1024;

/// A discovered extension directory on disk before runtime instantiation.
#[derive(Debug, Clone)]
pub struct DiscoveredExtension {
    pub manifest: ExtensionManifest,
    pub origin: ExtensionOrigin,
    pub dir: PathBuf,
    pub wasm_path: Option<PathBuf>,
    pub load_error: Option<String>,
}

/// Discovers extensions in `bundled_dir` and `user_dir`. User extensions with the same `id`
/// take precedence over bundled extensions.
pub fn discover_extensions(
    bundled_dir: Option<&Path>,
    user_dir: &Path,
) -> Vec<DiscoveredExtension> {
    let mut by_id: BTreeMap<String, DiscoveredExtension> = BTreeMap::new();

    if let Some(bundled) = bundled_dir {
        scan_extensions_dir(bundled, ExtensionOrigin::Bundled, &mut by_id);
    }
    scan_extensions_dir(user_dir, ExtensionOrigin::User, &mut by_id);

    let mut list: Vec<DiscoveredExtension> = by_id.into_values().collect();
    list.sort_by(|a, b| {
        a.manifest
            .name
            .to_ascii_lowercase()
            .cmp(&b.manifest.name.to_ascii_lowercase())
            .then_with(|| a.manifest.id.cmp(&b.manifest.id))
    });
    list
}

fn scan_extensions_dir(
    root: &Path,
    origin: ExtensionOrigin,
    out: &mut BTreeMap<String, DiscoveredExtension>,
) {
    if !root.is_dir() {
        return;
    }

    let Ok(entries) = fs::read_dir(root) else {
        warn!("Failed to read extensions directory {}", root.display());
        return;
    };

    for entry in entries.flatten() {
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        let folder_name = entry.file_name().to_string_lossy().to_string();
        if folder_name.starts_with('.') {
            continue;
        }

        let manifest_path = dir.join("extension.toml");
        if !manifest_path.is_file() {
            continue;
        }

        let discovered = load_discovered_dir(&dir, &folder_name, origin);
        out.insert(discovered.manifest.id.clone(), discovered);
    }
}

fn load_discovered_dir(
    dir: &Path,
    fallback_id: &str,
    origin: ExtensionOrigin,
) -> DiscoveredExtension {
    let manifest_path = dir.join("extension.toml");
    let content = match fs::read_to_string(&manifest_path) {
        Ok(c) => c,
        Err(e) => {
            return invalid_discovered(
                fallback_id,
                origin,
                dir,
                format!("Failed to read extension.toml: {e}"),
            );
        }
    };

    let manifest = match ExtensionManifest::from_toml_str(&content) {
        Ok(m) => m,
        Err(e) => {
            return invalid_discovered(fallback_id, origin, dir, e);
        }
    };

    match manifest.validate(dir) {
        Ok(wasm_path) => DiscoveredExtension {
            manifest,
            origin,
            dir: dir.to_path_buf(),
            wasm_path: Some(wasm_path),
            load_error: None,
        },
        Err(e) => DiscoveredExtension {
            manifest,
            origin,
            dir: dir.to_path_buf(),
            wasm_path: None,
            load_error: Some(e),
        },
    }
}

fn invalid_discovered(
    fallback_id: &str,
    origin: ExtensionOrigin,
    dir: &Path,
    error: String,
) -> DiscoveredExtension {
    DiscoveredExtension {
        manifest: ExtensionManifest {
            id: fallback_id.to_string(),
            name: fallback_id.to_string(),
            version: "?".to_string(),
            description: String::new(),
            authors: Vec::new(),
            repository: None,
            entry: "extension.wasm".to_string(),
            api_version: melody_extension_sdk::ABI_VERSION,
            permissions: Vec::new(),
            allowed_hosts: Vec::new(),
            settings: Vec::new(),
        },
        origin,
        dir: dir.to_path_buf(),
        wasm_path: None,
        load_error: Some(error),
    }
}

struct StagingGuard {
    path: PathBuf,
}

impl StagingGuard {
    fn new(user_dir: &Path) -> Result<Self, String> {
        fs::create_dir_all(user_dir)
            .map_err(|e| format!("Failed to create extensions directory: {e}"))?;
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let path = user_dir.join(format!(".staging-{}-{nanos}", std::process::id()));
        fs::create_dir_all(&path)
            .map_err(|e| format!("Failed to create staging directory: {e}"))?;
        Ok(Self { path })
    }
}

impl Drop for StagingGuard {
    fn drop(&mut self) {
        if self.path.exists() {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

/// Installs an extension from a directory or a `.zip` / `.mext` archive into `user_dir`,
/// validating its manifest, compiling its WASM module, and auditing imports against
/// declared permissions before committing.
pub fn install_extension_from_path(
    engine: &Engine,
    source: &Path,
    user_dir: &Path,
) -> Result<ExtensionManifest, String> {
    if !source.exists() {
        return Err(format!(
            "Selected path '{}' does not exist",
            source.display()
        ));
    }

    let staging = StagingGuard::new(user_dir)?;

    let ext_root = if source.is_dir() {
        copy_extension_dir(source, &staging.path)?;
        staging.path.clone()
    } else if source.is_file() {
        extract_extension_archive(source, &staging.path)?;
        resolve_archive_root(&staging.path)?
    } else {
        return Err(format!(
            "Unsupported extension source '{}'",
            source.display()
        ));
    };

    let manifest_path = ext_root.join("extension.toml");
    let manifest_str = fs::read_to_string(&manifest_path)
        .map_err(|_| "Extension package does not contain extension.toml".to_string())?;
    let manifest = ExtensionManifest::from_toml_str(&manifest_str)?;
    let wasm_path = manifest.validate(&ext_root)?;

    let meta = fs::metadata(&wasm_path)
        .map_err(|e| format!("Failed to inspect '{}': {e}", wasm_path.display()))?;
    if meta.len() > MAX_WASM_MODULE_BYTES {
        return Err(format!(
            "Extension module '{}' exceeds 16 MB limit",
            manifest.entry
        ));
    }

    let wasm_bytes = fs::read(&wasm_path)
        .map_err(|e| format!("Failed to read '{}': {e}", wasm_path.display()))?;
    let module = Module::new(engine, &wasm_bytes)
        .map_err(|e| format!("Invalid WebAssembly module '{}': {e}", manifest.entry))?;
    audit_module_imports(&module, &manifest.permissions)?;

    let dest_dir = user_dir.join(&manifest.id);
    commit_staged_extension(&ext_root, &dest_dir)?;

    info!(
        "Installed extension '{}' ({}) to {}",
        manifest.name,
        manifest.id,
        dest_dir.display()
    );
    Ok(manifest)
}

fn commit_staged_extension(staged_root: &Path, dest_dir: &Path) -> Result<(), String> {
    let mut backup_path: Option<PathBuf> = None;
    if dest_dir.exists() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let backup = dest_dir.with_file_name(format!(".backup-{}-{nanos}", std::process::id()));
        fs::rename(dest_dir, &backup)
            .map_err(|e| format!("Failed to replace existing extension: {e}"))?;
        backup_path = Some(backup);
    }

    if let Err(err) = fs::rename(staged_root, dest_dir) {
        if let Some(ref backup) = backup_path {
            let _ = fs::rename(backup, dest_dir);
        }
        return Err(format!("Failed to install extension into directory: {err}"));
    }

    if let Some(backup) = backup_path {
        let _ = fs::remove_dir_all(backup);
    }
    Ok(())
}

fn copy_extension_dir(source: &Path, dest: &Path) -> Result<(), String> {
    if !source.join("extension.toml").is_file() {
        return Err("Selected directory does not contain extension.toml".to_string());
    }

    let mut file_count = 0usize;
    let mut total_bytes = 0u64;

    let walker = walkdir::WalkDir::new(source).into_iter().filter_entry(|e| {
        if e.depth() == 0 {
            return true;
        }
        let name = e.file_name().to_string_lossy();
        !matches!(name.as_ref(), "target" | ".git" | ".cargo")
    });

    for entry in walker {
        let entry = entry.map_err(|e| format!("Failed to walk extension directory: {e}"))?;
        let rel = entry
            .path()
            .strip_prefix(source)
            .map_err(|e| format!("Failed to compute relative path: {e}"))?;
        if rel.as_os_str().is_empty() {
            continue;
        }

        let file_type = entry.file_type();
        if file_type.is_symlink() {
            continue;
        }

        let target_path = dest.join(rel);
        if file_type.is_dir() {
            fs::create_dir_all(&target_path)
                .map_err(|e| format!("Failed to create directory: {e}"))?;
        } else if file_type.is_file() {
            file_count += 1;
            if file_count > MAX_ARCHIVE_ENTRIES {
                return Err("Extension directory contains too many files".to_string());
            }
            let len = entry
                .metadata()
                .map_err(|e| format!("Failed to read file metadata: {e}"))?
                .len();
            total_bytes = total_bytes.saturating_add(len);
            if total_bytes > MAX_ARCHIVE_TOTAL_BYTES {
                return Err("Extension directory exceeds 32 MB size limit".to_string());
            }
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create parent directory: {e}"))?;
            }
            fs::copy(entry.path(), &target_path)
                .map_err(|e| format!("Failed to copy file '{}': {e}", rel.display()))?;
        }
    }

    Ok(())
}

fn extract_extension_archive(source: &Path, dest: &Path) -> Result<(), String> {
    let file = File::open(source).map_err(|e| {
        format!(
            "Failed to open extension archive '{}': {e}",
            source.display()
        )
    })?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| format!("Invalid extension archive (.zip/.mext): {e}"))?;

    if archive.len() > MAX_ARCHIVE_ENTRIES {
        return Err(format!(
            "Extension archive contains {} entries (limit is {MAX_ARCHIVE_ENTRIES})",
            archive.len()
        ));
    }

    let mut total_written = 0u64;
    for idx in 0..archive.len() {
        let entry = archive
            .by_index(idx)
            .map_err(|e| format!("Failed to read archive entry #{idx}: {e}"))?;

        let Some(rel_path) = entry.enclosed_name().map(|p| p.to_path_buf()) else {
            return Err("Archive contains unsafe path traversal entry".to_string());
        };

        let out_path = dest.join(&rel_path);
        if entry.is_dir() {
            fs::create_dir_all(&out_path)
                .map_err(|e| format!("Failed to create directory in archive: {e}"))?;
            continue;
        }

        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create parent directory: {e}"))?;
        }

        let remaining = MAX_ARCHIVE_TOTAL_BYTES.saturating_sub(total_written);
        let mut out_file = File::create(&out_path)
            .map_err(|e| format!("Failed to write '{}': {e}", rel_path.display()))?;

        // Read up to remaining + 1 bytes to catch spoofed zip header sizes safely.
        let mut limited = entry.take(remaining.saturating_add(1));
        let copied = std::io::copy(&mut limited, &mut out_file)
            .map_err(|e| format!("Failed to extract '{}': {e}", rel_path.display()))?;

        total_written = total_written.saturating_add(copied);
        if total_written > MAX_ARCHIVE_TOTAL_BYTES {
            return Err("Extension archive exceeds 32 MB uncompressed size limit".to_string());
        }
    }

    Ok(())
}

fn resolve_archive_root(staging_dir: &Path) -> Result<PathBuf, String> {
    if staging_dir.join("extension.toml").is_file() {
        return Ok(staging_dir.to_path_buf());
    }

    let mut subdirs = Vec::new();
    let entries = fs::read_dir(staging_dir)
        .map_err(|e| format!("Failed to inspect extracted archive: {e}"))?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name == "__MACOSX" || name.starts_with('.') {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            subdirs.push(path);
        }
    }

    if subdirs.len() == 1 && subdirs[0].join("extension.toml").is_file() {
        return Ok(subdirs.remove(0));
    }

    Err("Archive does not contain extension.toml at root or inside a single folder".to_string())
}

/// Removes a user-installed extension directory matching `id` from `user_dir`.
pub fn uninstall_extension(id: &str, user_dir: &Path) -> Result<(), String> {
    validate_extension_id(id)?;
    let dir = user_dir.join(id);
    if !dir.exists() {
        return Err(format!("User extension '{id}' is not installed"));
    }
    fs::remove_dir_all(&dir).map_err(|e| format!("Failed to remove extension '{id}': {e}"))?;
    info!("Uninstalled extension '{id}' from {}", dir.display());
    Ok(())
}
