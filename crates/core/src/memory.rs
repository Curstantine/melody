//! Lightweight process-memory introspection used by diagnostics telemetry and
//! the `bench_memory` example.
//!
//! On Linux the resident and peak-resident set sizes are read directly from
//! procfs. Platforms without a cheap, dependency-free source return `None`.

/// Current resident set size of the process in bytes, if the platform exposes it.
pub fn resident_bytes() -> Option<u64> {
    platform_field("VmRSS:")
}

/// Peak resident set size of the process in bytes, if the platform exposes it.
pub fn peak_resident_bytes() -> Option<u64> {
    platform_field("VmHWM:")
}

#[cfg(target_os = "linux")]
fn platform_field(field: &str) -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    parse_proc_status_kb(&status, field).map(|kb| kb.saturating_mul(1024))
}

#[cfg(not(target_os = "linux"))]
fn platform_field(_field: &str) -> Option<u64> {
    None
}

#[cfg(target_os = "linux")]
fn parse_proc_status_kb(contents: &str, field: &str) -> Option<u64> {
    contents
        .lines()
        .find_map(|line| line.strip_prefix(field))
        .and_then(|rest| rest.split_whitespace().next())
        .and_then(|value| value.parse::<u64>().ok())
}

/// Formats a byte count into a compact human-readable string (B / KB / MB / GB).
pub fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    const GB: f64 = 1024.0 * 1024.0 * 1024.0;

    let bytes_f = bytes as f64;
    if bytes_f < KB {
        format!("{} B", bytes)
    } else if bytes_f < MB {
        format!("{:.1} KB", bytes_f / KB)
    } else if bytes_f < GB {
        format!("{:.1} MB", bytes_f / MB)
    } else {
        format!("{:.2} GB", bytes_f / GB)
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    fn parses_proc_status_fields() {
        let sample = "Name:\tmelody\nVmHWM:\t  123456 kB\nVmRSS:\t   65432 kB\n";
        assert_eq!(parse_proc_status_kb(sample, "VmRSS:"), Some(65432));
        assert_eq!(parse_proc_status_kb(sample, "VmHWM:"), Some(123456));
        assert_eq!(parse_proc_status_kb(sample, "VmSwap:"), None);
    }

    #[test]
    fn formats_bytes() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GB");
    }
}
