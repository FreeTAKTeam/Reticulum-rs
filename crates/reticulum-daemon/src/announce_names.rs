const MAX_DISPLAY_NAME_CHARS: usize = 64;

pub fn normalize_display_name(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    if trimmed.chars().any(char::is_control) {
        return None;
    }

    let normalized: String = trimmed.chars().take(MAX_DISPLAY_NAME_CHARS).collect();
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

pub fn parse_peer_name_from_app_data(app_data: &[u8]) -> Option<(String, &'static str)> {
    if app_data.is_empty() {
        return None;
    }

    if is_msgpack_array_prefix(app_data[0]) {
        if let Some(name) = lxmf::helpers::display_name_from_app_data(app_data)
            .and_then(|value| normalize_display_name(&value))
        {
            return Some((name, "delivery_app_data"));
        }
    }

    if let Some(name) = lxmf::helpers::pn_name_from_app_data(app_data)
        .and_then(|value| normalize_display_name(&value))
    {
        return Some((name, "pn_meta"));
    }

    let text = std::str::from_utf8(app_data).ok()?;
    let name = normalize_display_name(text)?;
    Some((name, "app_data_utf8"))
}

fn is_msgpack_array_prefix(byte: u8) -> bool {
    (0x90..=0x9f).contains(&byte) || byte == 0xdc || byte == 0xdd
}
