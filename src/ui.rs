pub fn truncate(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

/// Converts a terminal coordinate to `u16`, saturating instead of truncating.
pub fn to_u16(n: usize) -> u16 {
    u16::try_from(n).unwrap_or(u16::MAX)
}
