use serde::{Deserialize, Serialize};

use super::photoshop;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppDefinition {
  pub id: String,
  pub name: String
}

pub fn list_apps() -> Vec<AppDefinition> {
  vec![
    AppDefinition {
      id: "desktop".to_string(),
      name: "Windows Desktop".to_string()
    },
    AppDefinition {
      id: "photoshop".to_string(),
      name: "Adobe Photoshop".to_string()
    }
  ]
}

pub fn detect_paths(app_id: &str) -> Vec<String> {
  match app_id {
    "photoshop" => photoshop::detect_paths(),
    "desktop" => Vec::new(),
    _ => Vec::new()
  }
}

pub fn is_supported(app_id: &str) -> bool {
  matches!(app_id, "photoshop" | "desktop")
}
