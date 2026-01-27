use reticulum_daemon::config::{DaemonConfig, InterfaceConfig};

#[test]
fn parses_tcp_client_interface() {
    let input = r#"
interfaces = [
  { type = "tcp_client", enabled = true, host = "rmap.world", port = 4242, name = "Public RMap" }
]
"#;
    let cfg = DaemonConfig::from_toml(input).expect("parse");
    assert_eq!(cfg.interfaces.len(), 1);
    let iface = &cfg.interfaces[0];
    assert_eq!(iface.name.as_deref(), Some("Public RMap"));
    assert_eq!(iface.host.as_deref(), Some("rmap.world"));
    assert_eq!(iface.port, Some(4242));
    assert!(iface.enabled.unwrap_or(false));
}

#[test]
fn filters_enabled_tcp_clients() {
    let cfg = DaemonConfig {
        interfaces: vec![
            InterfaceConfig {
                kind: "tcp_client".into(),
                enabled: Some(true),
                host: Some("rmap.world".into()),
                port: Some(4242),
                name: None,
            },
            InterfaceConfig {
                kind: "tcp_client".into(),
                enabled: Some(false),
                host: Some("example.com".into()),
                port: Some(1),
                name: None,
            },
        ],
    };
    let enabled: Vec<_> = cfg
        .interfaces
        .iter()
        .filter(|i| i.enabled.unwrap_or(false))
        .collect();
    assert_eq!(enabled.len(), 1);
    assert_eq!(enabled[0].host.as_deref(), Some("rmap.world"));
}
