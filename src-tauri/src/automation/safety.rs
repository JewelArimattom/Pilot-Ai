use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use super::action::ActionPlan;
use super::skills::action_id;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SafetyPolicy {
  pub allowed_apps: Vec<String>,
  pub allowed_actions: Vec<String>,
  pub require_confirm: bool,
  pub ghost_mode: bool
}

impl Default for SafetyPolicy {
  fn default() -> Self {
    Self {
      allowed_apps: vec!["Desktop".to_string(), "Photoshop".to_string()],
      allowed_actions: vec![
        "photoshop.crop".to_string(),
        "photoshop.adjust.levels".to_string(),
        "photoshop.export.png".to_string(),
        "photoshop.export.jpeg".to_string(),
        "photoshop.resize.canvas".to_string(),
        "photoshop.rotate".to_string(),
        "photoshop.straighten".to_string(),
        "desktop.open_app".to_string(),
        "desktop.click".to_string(),
        "desktop.move_mouse".to_string(),
        "desktop.type_text".to_string(),
        "desktop.hotkey".to_string(),
        "desktop.scroll".to_string(),
        "desktop.wait".to_string(),
        "desktop.ui_click".to_string(),
        "desktop.ui_type".to_string(),
        "desktop.ui_read".to_string()
      ],
      require_confirm: true,
      ghost_mode: true
    }
  }
}

pub fn validate_plan(plan: &ActionPlan, policy: &SafetyPolicy) -> Result<(), String> {
  if plan.steps.is_empty() {
    return Err("Plan has no steps.".to_string());
  }

  let allowed: HashSet<String> = policy.allowed_actions.iter().cloned().collect();
  for step in &plan.steps {
    let id = action_id(&step.action);
    if !allowed.contains(id) {
      return Err(format!("Action not allowed: {id}"));
    }
  }

  Ok(())
}
