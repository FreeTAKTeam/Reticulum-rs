use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct DaemonConfig {
    #[serde(default)]
    pub interfaces: Vec<InterfaceConfig>,
}

#[derive(Debug, Deserialize)]
pub struct InterfaceConfig {
    #[serde(rename = "type")]
    pub kind: String,
    pub enabled: Option<bool>,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub name: Option<String>,
}

impl DaemonConfig {
    pub fn from_toml(input: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(input)
    }
}
