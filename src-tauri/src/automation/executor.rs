use serde::{Deserialize, Serialize};

use super::action::ActionPlan;
use super::{apps, config, desktop, photoshop};
use super::safety::{validate_plan, SafetyPolicy};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StepResult {
  pub step_id: String,
  pub status: StepStatus,
  pub message: Option<String>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StepStatus {
  Pending,
  Completed,
  Blocked,
  Failed
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionReport {
  pub plan_id: String,
  pub summary: String,
  pub blocked: bool,
  pub results: Vec<StepResult>
}

pub fn execute_plan(app: tauri::AppHandle, plan: &ActionPlan, policy: &SafetyPolicy) -> ExecutionReport {
  if let Err(error) = validate_plan(plan, policy) {
    return failure_report(plan, &error);
  }

  let app_config = config::load_config();
  let app_id = policy.allowed_apps.first().cloned().unwrap_or_else(|| config::resolve_app_id(&app_config));
  
  if !apps::is_supported(&app_id) {
    return failure_report(plan, &format!("Unsupported app: {app_id}"));
  }

  if app_id == "desktop" {
    return desktop::execute_plan(app, plan, policy);
  }

  let app_path = match config::resolve_app_path(&app_id, &app_config) {
    Some(path) => path,
    None => {
      return failure_report(plan, "Photoshop path not configured. Set it in App settings.");
    }
  };

  if !config::path_exists(&app_path) {
    return failure_report(plan, "Photoshop executable not found at the configured path.");
  }

  let script = match photoshop::build_script(plan) {
    Ok(script) => script,
    Err(error) => return failure_report(plan, &error)
  };
  let script_path = match photoshop::write_script(&script) {
    Ok(path) => path,
    Err(error) => return failure_report(plan, &error)
  };

  if let Err(error) = photoshop::run_script(&app_path, &script_path) {
    return failure_report(plan, &error);
  }

  let results = plan
    .steps
    .iter()
    .map(|step| StepResult {
      step_id: step.step_id.clone(),
      status: StepStatus::Completed,
      message: Some("Sent to Photoshop".to_string())
    })
    .collect();

  ExecutionReport {
    plan_id: plan.plan_id.clone(),
    summary: format!("Sent {} step(s) to Photoshop.", plan.steps.len()),
    blocked: false,
    results
  }
}

fn failure_report(plan: &ActionPlan, summary: &str) -> ExecutionReport {
  ExecutionReport {
    plan_id: plan.plan_id.clone(),
    summary: summary.to_string(),
    blocked: true,
    results: Vec::new()
  }
}
