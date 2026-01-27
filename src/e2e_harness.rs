pub fn is_ready_line(line: &str) -> bool {
    line.contains("listening on http://")
}
