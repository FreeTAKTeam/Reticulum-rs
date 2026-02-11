pub fn encode_delivery_display_name_app_data(display_name: &str) -> Option<Vec<u8>> {
    let normalized = normalize_display_name(display_name)?;
    let peer_data = rmpv::Value::Array(vec![
        rmpv::Value::Binary(normalized.into_bytes()),
        rmpv::Value::Nil,
    ]);
    rmp_serde::to_vec(&peer_data).ok()
}

pub fn normalize_display_name(value: &str) -> Option<String> {
    lxmf::helpers::normalize_display_name(value).ok()
}

pub fn parse_peer_name_from_app_data(app_data: &[u8]) -> Option<(String, &'static str)> {
    if app_data.is_empty() {
        return None;
    }

    if lxmf::helpers::is_msgpack_array_prefix(app_data[0]) {
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
