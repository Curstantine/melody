use std::collections::HashMap;
use std::io::{self, Read, Write};

const MAX_CANDIDATES: usize = 64;
const MAX_OPEN_HANDLES: usize = 8;
const MAX_READ_BYTES: usize = 1024 * 1024;
const MAX_WRITE_BYTES: usize = 1024 * 1024;
const IPC_IO_TIMEOUT_MS: u32 = 3_000;

#[cfg(windows)]
pub struct IpcStream {
    handle: windows::Win32::Foundation::HANDLE,
}

#[cfg(windows)]
unsafe impl Send for IpcStream {}

#[cfg(windows)]
impl Drop for IpcStream {
    fn drop(&mut self) {
        unsafe {
            let _ = windows::Win32::Foundation::CloseHandle(self.handle);
        }
    }
}

#[cfg(unix)]
pub struct IpcStream(std::os::unix::net::UnixStream);

#[cfg(not(any(windows, unix)))]
pub struct IpcStream;

impl IpcStream {
    fn connect_named(name: &str) -> Result<Self, String> {
        #[cfg(windows)]
        {
            use windows::core::PCWSTR;
            use windows::Win32::Foundation::{ERROR_PIPE_BUSY, GENERIC_READ, GENERIC_WRITE};
            use windows::Win32::Storage::FileSystem::{
                CreateFileW, FILE_FLAG_OVERLAPPED, FILE_SHARE_NONE, OPEN_EXISTING,
            };
            use windows::Win32::System::Pipes::WaitNamedPipeW;

            let pipe_path = format!(r"\\.\pipe\{name}");
            let wide: Vec<u16> = pipe_path.encode_utf16().chain(std::iter::once(0)).collect();

            let open_pipe = || unsafe {
                CreateFileW(
                    PCWSTR(wide.as_ptr()),
                    GENERIC_READ.0 | GENERIC_WRITE.0,
                    FILE_SHARE_NONE,
                    None,
                    OPEN_EXISTING,
                    FILE_FLAG_OVERLAPPED,
                    None,
                )
            };

            let handle = match open_pipe() {
                Ok(h) => h,
                Err(err) if err.code() == ERROR_PIPE_BUSY.to_hresult() => {
                    let waited = unsafe { WaitNamedPipeW(PCWSTR(wide.as_ptr()), 500) };
                    if waited.as_bool() {
                        open_pipe()
                            .map_err(|e| format!("Failed to open named pipe '{pipe_path}': {e}"))?
                    } else {
                        return Err(format!("Named pipe '{pipe_path}' is busy"));
                    }
                }
                Err(err) => {
                    return Err(format!("Failed to open named pipe '{pipe_path}': {err}"));
                }
            };

            Ok(Self { handle })
        }

        #[cfg(unix)]
        {
            use std::os::unix::net::UnixStream;
            use std::path::PathBuf;
            use std::time::Duration;

            let mut candidate_dirs: Vec<PathBuf> = Vec::new();
            let mut push_unique = |p: PathBuf| {
                if !candidate_dirs.contains(&p) {
                    candidate_dirs.push(p);
                }
            };

            if let Ok(xdg) = std::env::var("XDG_RUNTIME_DIR") {
                if !xdg.is_empty() {
                    push_unique(PathBuf::from(xdg));
                }
            }
            for var in ["TMPDIR", "TMP", "TEMP"] {
                if let Ok(dir) = std::env::var(var) {
                    if !dir.is_empty() {
                        push_unique(PathBuf::from(dir));
                    }
                }
            }
            push_unique(PathBuf::from("/tmp"));

            for dir in candidate_dirs {
                let sock_path = dir.join(name);
                if let Ok(stream) = UnixStream::connect(&sock_path) {
                    let timeout = Some(Duration::from_millis(u64::from(IPC_IO_TIMEOUT_MS)));
                    let _ = stream.set_read_timeout(timeout);
                    let _ = stream.set_write_timeout(timeout);
                    return Ok(Self(stream));
                }
            }
            Err(format!("Could not connect to Unix socket '{name}'"))
        }

        #[cfg(not(any(windows, unix)))]
        {
            let _ = name;
            Err("Local IPC is not supported on this platform".to_string())
        }
    }
}

#[cfg(windows)]
fn wait_overlapped(
    handle: windows::Win32::Foundation::HANDLE,
    overlapped: &mut windows::Win32::System::IO::OVERLAPPED,
    event: windows::Win32::Foundation::HANDLE,
) -> io::Result<usize> {
    use windows::Win32::Foundation::WAIT_OBJECT_0;
    use windows::Win32::System::Threading::WaitForSingleObject;
    use windows::Win32::System::IO::{CancelIoEx, GetOverlappedResult};

    let wait_res = unsafe { WaitForSingleObject(event, IPC_IO_TIMEOUT_MS) };
    let mut transferred = 0u32;
    if wait_res == WAIT_OBJECT_0 {
        unsafe { GetOverlappedResult(handle, overlapped, &mut transferred, false) }
            .map_err(|e| io::Error::other(e.to_string()))?;
        Ok(transferred as usize)
    } else {
        unsafe {
            let _ = CancelIoEx(handle, Some(overlapped));
            let _ = GetOverlappedResult(handle, overlapped, &mut transferred, true);
        }
        Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "IPC operation timed out",
        ))
    }
}

impl Read for IpcStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        #[cfg(windows)]
        {
            use windows::Win32::Foundation::{CloseHandle, ERROR_IO_PENDING};
            use windows::Win32::Storage::FileSystem::ReadFile;
            use windows::Win32::System::Threading::CreateEventW;
            use windows::Win32::System::IO::OVERLAPPED;

            if buf.is_empty() {
                return Ok(0);
            }
            let event = unsafe { CreateEventW(None, true, false, None) }
                .map_err(|e| io::Error::other(e.to_string()))?;
            let mut overlapped = OVERLAPPED {
                hEvent: event,
                ..Default::default()
            };
            let mut bytes_read = 0u32;
            let res = unsafe {
                ReadFile(
                    self.handle,
                    Some(buf),
                    Some(&mut bytes_read),
                    Some(&mut overlapped),
                )
            };
            let out = match res {
                Ok(()) => Ok(bytes_read as usize),
                Err(e) if e.code() == ERROR_IO_PENDING.to_hresult() => {
                    wait_overlapped(self.handle, &mut overlapped, event)
                }
                Err(e) => Err(io::Error::other(e.to_string())),
            };
            unsafe {
                let _ = CloseHandle(event);
            }
            out
        }
        #[cfg(unix)]
        {
            self.0.read(buf)
        }
        #[cfg(not(any(windows, unix)))]
        {
            let _ = buf;
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Local IPC unsupported",
            ))
        }
    }
}

impl Write for IpcStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        #[cfg(windows)]
        {
            use windows::Win32::Foundation::{CloseHandle, ERROR_IO_PENDING};
            use windows::Win32::Storage::FileSystem::WriteFile;
            use windows::Win32::System::Threading::CreateEventW;
            use windows::Win32::System::IO::OVERLAPPED;

            if buf.is_empty() {
                return Ok(0);
            }
            let event = unsafe { CreateEventW(None, true, false, None) }
                .map_err(|e| io::Error::other(e.to_string()))?;
            let mut overlapped = OVERLAPPED {
                hEvent: event,
                ..Default::default()
            };
            let mut bytes_written = 0u32;
            let res = unsafe {
                WriteFile(
                    self.handle,
                    Some(buf),
                    Some(&mut bytes_written),
                    Some(&mut overlapped),
                )
            };
            let out = match res {
                Ok(()) => Ok(bytes_written as usize),
                Err(e) if e.code() == ERROR_IO_PENDING.to_hresult() => {
                    wait_overlapped(self.handle, &mut overlapped, event)
                }
                Err(e) => Err(io::Error::other(e.to_string())),
            };
            unsafe {
                let _ = CloseHandle(event);
            }
            out
        }
        #[cfg(unix)]
        {
            self.0.write(buf)
        }
        #[cfg(not(any(windows, unix)))]
        {
            let _ = buf;
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Local IPC unsupported",
            ))
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        #[cfg(windows)]
        {
            Ok(())
        }
        #[cfg(unix)]
        {
            self.0.flush()
        }
        #[cfg(not(any(windows, unix)))]
        {
            Ok(())
        }
    }
}

/// Validates that an IPC endpoint name is a safe relative pipe/socket name (no traversal or absolute paths).
pub fn validate_ipc_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name.len() > 128 {
        return Err("IPC endpoint name must be between 1 and 128 characters".to_string());
    }
    if name.starts_with('/') || name.ends_with('/') || name.contains("..") {
        return Err(format!(
            "IPC endpoint name '{name}' contains disallowed path sequences"
        ));
    }
    for segment in name.split('/') {
        if segment.is_empty() || segment == "." || segment == ".." {
            return Err(format!(
                "IPC endpoint name '{name}' contains invalid path segment"
            ));
        }
        if !segment
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
        {
            return Err(format!(
                "IPC endpoint name '{name}' may only contain ASCII letters, digits, '.', '_', '-', and '/'"
            ));
        }
    }
    Ok(())
}

/// Validates an `allowed_ipc` pattern from `extension.toml`.
pub fn validate_ipc_pattern(pattern: &str) -> Result<(), String> {
    let pattern = pattern.trim();
    if pattern.is_empty() || pattern.len() > 128 {
        return Err("allowed_ipc pattern must be between 1 and 128 characters".to_string());
    }
    if pattern == "*" {
        return Err("allowed_ipc pattern '*' is too permissive".to_string());
    }
    if pattern.matches('*').count() > 1 {
        return Err(format!(
            "allowed_ipc pattern '{pattern}' may contain at most one '*' wildcard"
        ));
    }
    if pattern.starts_with('/') || pattern.ends_with('/') || pattern.contains("..") {
        return Err(format!(
            "allowed_ipc pattern '{pattern}' contains disallowed path sequences"
        ));
    }
    for segment in pattern.split('/') {
        if segment.is_empty() || segment == "." || segment == ".." {
            return Err(format!(
                "allowed_ipc pattern '{pattern}' contains invalid path segment"
            ));
        }
        if !segment
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '*'))
        {
            return Err(format!(
                "allowed_ipc pattern '{pattern}' contains disallowed characters"
            ));
        }
    }
    Ok(())
}

/// Matches a concrete IPC endpoint name against a glob-like `allowed_ipc` pattern (`*` wildcard).
/// The `*` wildcard only matches within a single path segment (it never matches `/`).
pub fn ipc_name_matches_pattern(name: &str, pattern: &str) -> bool {
    let pattern = pattern.trim();
    if pattern.is_empty() {
        return false;
    }
    if let Some((prefix, suffix)) = pattern.split_once('*') {
        if suffix.contains('*') {
            return false;
        }
        if name.len() < prefix.len() + suffix.len()
            || !name.starts_with(prefix)
            || !name.ends_with(suffix)
        {
            return false;
        }
        let matched = &name[prefix.len()..name.len() - suffix.len()];
        !matched.contains('/')
    } else {
        name == pattern
    }
}

pub fn check_ipc_allowed(name: &str, allowed_ipc: &[String]) -> Result<(), String> {
    validate_ipc_name(name)?;
    if allowed_ipc.is_empty() {
        return Err(
            "Extension must declare at least one entry in 'allowed_ipc' to use local-ipc"
                .to_string(),
        );
    }
    if !allowed_ipc
        .iter()
        .any(|pat| ipc_name_matches_pattern(name, pat))
    {
        return Err(format!(
            "IPC endpoint '{name}' is not permitted by extension allowed_ipc"
        ));
    }
    Ok(())
}

/// Per-extension table of open local IPC streams, automatically closed when the extension drops.
#[derive(Default)]
pub struct LocalIpcRegistry {
    next_id: u32,
    streams: HashMap<u32, IpcStream>,
}

impl LocalIpcRegistry {
    pub fn connect(
        &mut self,
        candidates: &[String],
        allowed_ipc: &[String],
    ) -> Result<u32, String> {
        if candidates.is_empty() || candidates.len() > MAX_CANDIDATES {
            return Err(format!(
                "IPC candidate list must contain between 1 and {MAX_CANDIDATES} names"
            ));
        }
        if self.streams.len() >= MAX_OPEN_HANDLES {
            return Err(format!(
                "Extension has reached the maximum of {MAX_OPEN_HANDLES} open IPC handles"
            ));
        }

        for name in candidates {
            check_ipc_allowed(name, allowed_ipc)?;
        }

        let mut last_err = None;
        for name in candidates {
            match IpcStream::connect_named(name) {
                Ok(stream) => {
                    self.next_id = self.next_id.wrapping_add(1).max(1);
                    while self.streams.contains_key(&self.next_id) {
                        self.next_id = self.next_id.wrapping_add(1).max(1);
                    }
                    let id = self.next_id;
                    self.streams.insert(id, stream);
                    return Ok(id);
                }
                Err(err) => last_err = Some(err),
            }
        }

        Err(last_err.unwrap_or_else(|| "No matching local IPC endpoint found".to_string()))
    }

    pub fn write(&mut self, handle: u32, data: &[u8]) -> Result<(), String> {
        if data.len() > MAX_WRITE_BYTES {
            return Err(format!(
                "IPC write payload ({} bytes) exceeds limit of {MAX_WRITE_BYTES} bytes",
                data.len()
            ));
        }
        let stream = self
            .streams
            .get_mut(&handle)
            .ok_or_else(|| format!("Invalid or closed IPC handle {handle}"))?;
        stream
            .write_all(data)
            .and_then(|()| stream.flush())
            .map_err(|e| format!("IPC write failed: {e}"))
    }

    pub fn read(&mut self, handle: u32, max_len: u32) -> Result<Vec<u8>, String> {
        let limit = (max_len as usize).min(MAX_READ_BYTES);
        if limit == 0 {
            return Ok(Vec::new());
        }
        let stream = self
            .streams
            .get_mut(&handle)
            .ok_or_else(|| format!("Invalid or closed IPC handle {handle}"))?;
        let mut buf = vec![0u8; limit];
        let n = stream
            .read(&mut buf)
            .map_err(|e| format!("IPC read failed: {e}"))?;
        buf.truncate(n);
        Ok(buf)
    }

    pub fn close(&mut self, handle: u32) {
        self.streams.remove(&handle);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_ipc_name_and_allowlist() {
        assert!(validate_ipc_name("discord-ipc-0").is_ok());
        assert!(validate_ipc_name("app/com.example.App/ipc-0").is_ok());
        assert!(validate_ipc_name(".flatpak/com.discordapp.Discord/xdg-run/discord-ipc-0").is_ok());
        assert!(validate_ipc_name("../secret").is_err());
        assert!(validate_ipc_name("a/../b").is_err());
        assert!(validate_ipc_name("/abs/socket").is_err());
        assert!(validate_ipc_name("./hidden").is_err());
        assert!(validate_ipc_name(".").is_err());
        assert!(validate_ipc_name("").is_err());

        let allowed = vec!["discord-ipc-*".to_string()];
        assert!(check_ipc_allowed("discord-ipc-0", &allowed).is_ok());
        assert!(check_ipc_allowed("discord-ipc-9", &allowed).is_ok());
        assert!(check_ipc_allowed("discord-ipc-0/sub", &allowed).is_err());
        assert!(check_ipc_allowed("docker.sock", &allowed).is_err());
        assert!(check_ipc_allowed("discord-ipc-0", &[]).is_err());

        assert!(validate_ipc_pattern("discord-ipc-*").is_ok());
        assert!(validate_ipc_pattern("app/com.example.App/ipc-*").is_ok());
        assert!(validate_ipc_pattern(".flatpak/com.example.App/xdg-run/ipc-*").is_ok());
        assert!(validate_ipc_pattern("").is_err());
        assert!(validate_ipc_pattern("*").is_err());
        assert!(validate_ipc_pattern("a*b*c").is_err());
        assert!(validate_ipc_pattern("../foo*").is_err());
        assert!(validate_ipc_pattern("/abs*").is_err());
    }
}
