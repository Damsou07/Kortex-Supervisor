use kortex_application::SecretStore;
use kortex_domain::*;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    collections::BTreeMap,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagramTheme {
    #[default]
    Modern,
    Midnight,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LegendStyle {
    #[default]
    Compact,
    Detailed,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Appearance {
    pub diagram_theme: DiagramTheme,
    pub legend_style: LegendStyle,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub version: u32,
    pub profiles: Vec<Profile>,
    pub diagrams: BTreeMap<String, Diagram>,
    pub data_refresh_secs: f32,
    pub schema_refresh_secs: f32,
    pub sql_limit: usize,
    pub layout: Option<serde_json::Value>,
    pub appearance: Appearance,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            version: 1,
            profiles: vec![],
            diagrams: BTreeMap::new(),
            data_refresh_secs: 1.0,
            schema_refresh_secs: 5.0,
            sql_limit: 10000,
            layout: None,
            appearance: Appearance::default(),
        }
    }
}
pub fn config_path() -> Result<PathBuf> {
    // Useful for reproducible UI tests and portable deployments, without touching normal profiles.
    if let Some(path) = std::env::var_os("KORTEX_CONFIG_DIR") {
        return Ok(PathBuf::from(path).join("workspace.json"));
    }
    directories::ProjectDirs::from("", "Kortex", "Kortex Supervisor")
        .map(|p| p.config_dir().join("workspace.json"))
        .ok_or_else(|| Error::Storage("Dossier de configuration introuvable".into()))
}
pub fn load(path: &Path) -> Result<Preferences> {
    if !path.exists() {
        return Ok(Preferences::default());
    }
    let prefs: Preferences = read_json(path)?;
    if prefs.version != 1 {
        return Err(Error::Storage(
            "Version de configuration non prise en charge ; fichier conservé".into(),
        ));
    }
    Ok(prefs)
}
pub fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let file = std::fs::File::open(path).map_err(|e| Error::Storage(e.to_string()))?;
    serde_json::from_reader(file).map_err(|e| Error::Storage(e.to_string()))
}
pub fn save<T: Serialize>(path: &Path, data: &T) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| Error::Storage("Chemin invalide".into()))?;
    std::fs::create_dir_all(parent).map_err(|e| Error::Storage(e.to_string()))?;
    let mut temp =
        tempfile::NamedTempFile::new_in(parent).map_err(|e| Error::Storage(e.to_string()))?;
    serde_json::to_writer_pretty(&mut temp, data).map_err(|e| Error::Storage(e.to_string()))?;
    temp.flush()
        .and_then(|_| temp.as_file().sync_all())
        .map_err(|e| Error::Storage(e.to_string()))?;
    temp.persist(path)
        .map_err(|e| Error::Storage(e.error.to_string()))?;
    Ok(())
}
pub struct NativeSecrets;
impl SecretStore for NativeSecrets {
    fn get(&self, profile: &str) -> Result<Option<String>> {
        let entry = keyring::v1::Entry::new("Kortex Supervisor", profile)
            .map_err(|e| Error::Storage(e.to_string()))?;
        match entry.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::v1::Error::NoEntry) => Ok(None),
            Err(e) => Err(Error::Storage(e.to_string())),
        }
    }
    fn set(&self, profile: &str, secret: &str) -> Result<()> {
        keyring::v1::Entry::new("Kortex Supervisor", profile)
            .and_then(|entry| entry.set_password(secret))
            .map_err(|e| Error::Storage(e.to_string()))
    }
    fn delete(&self, profile: &str) -> Result<()> {
        let entry = keyring::v1::Entry::new("Kortex Supervisor", profile)
            .map_err(|e| Error::Storage(e.to_string()))?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::v1::Error::NoEntry) => Ok(()),
            Err(e) => Err(Error::Storage(e.to_string())),
        }
    }
}
