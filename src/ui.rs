pub fn truncate(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

/// Converts a terminal coordinate to `u16`, saturating instead of truncating.
pub fn to_u16(n: usize) -> u16 {
    u16::try_from(n).unwrap_or(u16::MAX)
}

pub fn to_u32(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}
