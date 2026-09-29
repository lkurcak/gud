pub fn truncate(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}
