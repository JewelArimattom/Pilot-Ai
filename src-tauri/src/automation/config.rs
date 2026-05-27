use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ActionCatalogEntry {
  pub action_set: String,
  pub action_name: String,
  pub description: Option<String>
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
  pub selected_app_id: Option<String>,
  pub selected_app_path: Option<String>,
  #[serde(default)]
  pub photoshop_actions: Vec<ActionCatalogEntry>
}

pub fn load_config() -> AppConfig {
  let path = match config_path() {
    Ok(path) => path,
    Err(_) => return AppConfig::default()
  };

  let content = match fs::read_to_string(path) {
    Ok(content) => content,
    Err(_) => return AppConfig::default()
  };

  serde_json::from_str(&content).unwrap_or_default()
}

pub fn save_config(config: &AppConfig) -> Result<(), String> {
  let path = config_path()?;
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent).map_err(|error| format!("Config dir error: {error}"))?;
  }

  let payload = serde_json::to_string_pretty(config)
    .map_err(|error| format!("Config serialize error: {error}"))?;
  fs::write(&path, payload).map_err(|error| format!("Config write error: {error}"))?;
  Ok(())
}

pub fn config_path() -> Result<PathBuf, String> {
  let base = tauri::api::path::config_dir().ok_or("Config directory not available")?;
  Ok(base.join("pilot-ai").join("config.json"))
}

pub fn resolve_app_id(config: &AppConfig) -> String {
  config
    .selected_app_id
    .clone()
    .unwrap_or_else(|| "desktop".to_string())
}

pub fn resolve_app_path(app_id: &str, config: &AppConfig) -> Option<String> {
  if app_id != "photoshop" {
    return None;
  }

  if let Ok(value) = std::env::var("PHOTOSHOP_PATH") {
    let trimmed = value.trim();
    if !trimmed.is_empty() {
      return Some(trimmed.to_string());
    }
  }

  config
    .selected_app_path
    .as_ref()
    .map(|path| path.trim().to_string())
    .filter(|path| !path.is_empty())
}

pub fn path_exists(path: &str) -> bool {
  Path::new(path).exists()
}
