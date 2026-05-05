use crate::error::{FileSplitError, Result};

/// Parse a human-readable size string into bytes.
///
/// Supported suffixes (case-insensitive):
/// - B, KB, MB, GB, TB
/// - KiB, MiB, GiB, TiB  (binary — same as KB etc. for simplicity)
///
/// Examples: "100MB", "1.5GB", "500kb", "1024"
pub fn parse_size(s: &str) -> Result<u64> {
    let s = s.trim();

    // Try plain number first
    if let Ok(n) = s.parse::<u64>() {
        return Ok(n);
    }

    // Find where the numeric part ends
    let split_pos = s
        .chars()
        .position(|c| c.is_alphabetic())
        .ok_or_else(|| FileSplitError::InvalidSize(s.to_string()))?;

    let (num_str, suffix) = s.split_at(split_pos);
    let num: f64 = num_str
        .trim()
        .parse()
        .map_err(|_| FileSplitError::InvalidSize(s.to_string()))?;

    let multiplier: u64 = match suffix.trim().to_uppercase().as_str() {
        "B" => 1,
        "K" | "KB" | "KIB" => 1_024,
        "M" | "MB" | "MIB" => 1_024 * 1_024,
        "G" | "GB" | "GIB" => 1_024 * 1_024 * 1_024,
        "T" | "TB" | "TIB" => 1_024u64.pow(4),
        other => return Err(FileSplitError::InvalidSize(format!("unknown suffix '{}'", other))),
    };

    Ok((num * multiplier as f64) as u64)
}

/// Format bytes as a human-readable string.
pub fn format_bytes(bytes: u64) -> String {
    use humansize::{format_size, BINARY};
    format_size(bytes, BINARY)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_size() {
        assert_eq!(parse_size("1024").unwrap(), 1024);
        assert_eq!(parse_size("1KB").unwrap(), 1024);
        assert_eq!(parse_size("1MB").unwrap(), 1024 * 1024);
        assert_eq!(parse_size("1GB").unwrap(), 1024 * 1024 * 1024);
        assert_eq!(parse_size("500mb").unwrap(), 500 * 1024 * 1024);
        assert_eq!(parse_size("1.5GB").unwrap(), (1.5 * 1024.0 * 1024.0 * 1024.0) as u64);
        assert!(parse_size("abc").is_err());
        assert!(parse_size("1ZB").is_err());
    }
}
