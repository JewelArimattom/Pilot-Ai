mod automation;

use std::time::{SystemTime, UNIX_EPOCH};
use std::process::Command;

use automation::action::{ActionPlan, ActionStep, GhostPreview, PlanStatus, Point, Rect};
use automation::executor::{execute_plan, ExecutionReport};
use automation::safety::SafetyPolicy;
use automation::{ai, apps, config};

#[tauri::command]
fn set_overlay_mode(app: tauri::AppHandle, enabled: bool) {
  use tauri::Manager;
  if let Some(overlay) = app.get_window("overlay") {
    if enabled {
      let _ = overlay.show();
    } else {
      let _ = overlay.hide();
    }
  }
}

#[tauri::command]
async fn plan_actions(
  request: String,
  ghost_mode: bool,
  teach_mode: bool,
  fast_mode: bool,
  screen_image: Option<String>
) -> ActionPlan {
  let config = match ai::load_gemini_config() {
    Some(config) => config,
    None => return empty_plan("AI planner unavailable. Set GEMINI_API_KEYS and retry.")
  };

  match ai::plan_request(&request, &config, screen_image, teach_mode).await {
    Ok(plan) => build_plan_from_ai(plan, ghost_mode, teach_mode, fast_mode),
    Err(error) => empty_plan(&format!("Gemini error: {error}"))
  }
}

#[tauri::command]
async fn capture_screen() -> Result<String, String> {
  let script = r#"Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
$bounds = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$bitmap = New-Object System.Drawing.Bitmap $bounds.Width, $bounds.Height
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
$graphics.CopyFromScreen($bounds.Location, [System.Drawing.Point]::Empty, $bounds.Size)
$stream = New-Object System.IO.MemoryStream
$bitmap.Save($stream, [System.Drawing.Imaging.ImageFormat]::Png)
$bytes = $stream.ToArray()
[System.Convert]::ToBase64String($bytes)
"#;

  let output = Command::new("powershell")
    .args(["-NoProfile", "-NonInteractive", "-Command", script])
    .output()
    .map_err(|error| format!("Screen capture failed: {error}"))?;

  if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if stderr.is_empty() {
      return Err("Screen capture failed.".to_string());
    }
    return Err(format!("Screen capture failed: {stderr}"));
  }

  let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
  if stdout.is_empty() {
    return Err("Screen capture returned empty output.".to_string());
  }

  Ok(stdout)
}

#[tauri::command]
async fn execute_actions(app: tauri::AppHandle, plan: ActionPlan, policy: SafetyPolicy) -> ExecutionReport {
  execute_plan(app, &plan, &policy)
}

#[tauri::command]
fn list_apps() -> Vec<apps::AppDefinition> {
  apps::list_apps()
}

#[tauri::command]
fn detect_app_paths(app_id: String) -> Vec<String> {
  apps::detect_paths(&app_id)
}

#[tauri::command]
fn get_app_config() -> config::AppConfig {
  config::load_config()
}

#[tauri::command]
fn set_app_config(mut config: config::AppConfig) -> Result<config::AppConfig, String> {
  if let Some(path) = config.selected_app_path.as_ref() {
    let trimmed = path.trim();
    if trimmed.is_empty() {
      config.selected_app_path = None;
    } else {
      config.selected_app_path = Some(trimmed.to_string());
    }
  }

  if config.selected_app_id.is_none() {
    config.selected_app_id = Some("desktop".to_string());
  }

  config::save_config(&config)?;
  Ok(config)
}

fn main() {
  let _ = dotenvy::dotenv();
  let root_env = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(".env");
  let _ = dotenvy::from_path(root_env);
  tauri::Builder::default()
      .setup(|app| {
        use tauri::Manager;
        if let Some(overlay) = app.get_window("overlay") {
          let _ = overlay.set_ignore_cursor_events(true);
        }
        Ok(())
      })
      .invoke_handler(tauri::generate_handler![
        plan_actions,
        capture_screen,
        execute_actions,
        list_apps,
        detect_app_paths,
        get_app_config,
        set_app_config,
        set_overlay_mode
      ])
      .run(tauri::generate_context!())
      .expect("error while running tauri application");
}
fn now_millis() -> u128 {
  SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap_or_default()
    .as_millis()
}

fn build_plan_from_ai(
  plan: ai::PlannedActions,
  ghost_mode: bool,
  teach_mode: bool,
  fast_mode: bool
) -> ActionPlan {
  if plan.steps.is_empty() {
    return empty_plan(plan.note.as_deref().unwrap_or("AI did not return any steps."));
  }

  let base = now_millis();
  let steps = plan
    .steps
    .into_iter()
    .enumerate()
    .map(|(index, step)| ActionStep {
      step_id: step
        .id
        .unwrap_or_else(|| format!("step-{}-{}", base, index)),
      requires_confirm: step.requires_confirm,
      action: step.action,
      expected_result: step.expected_result,
      retryable: step.retryable,
      explanation: step.explanation,
      expected: Vec::new(),
      ghost_preview: build_ghost_preview(ghost_mode, teach_mode, fast_mode),
      note: None
    })
    .collect();

  ActionPlan {
    plan_id: format!("plan-{}", base),
    created_at: base.to_string(),
    goal: plan.goal,
    status: PlanStatus::Ready,
    steps,
    note: plan.note
  }
}

fn empty_plan(note: &str) -> ActionPlan {
  ActionPlan {
    plan_id: format!("plan-{}", now_millis()),
    created_at: now_millis().to_string(),
    goal: None,
    status: PlanStatus::NeedsUser,
    steps: Vec::new(),
    note: Some(note.to_string())
  }
}

fn build_ghost_preview(ghost_mode: bool, teach_mode: bool, fast_mode: bool) -> GhostPreview {
  GhostPreview {
    cursor_path: if ghost_mode {
      vec![Point { x: 240.0, y: 180.0 }, Point { x: 420.0, y: 240.0 }]
    } else {
      Vec::new()
    },
    highlight: if ghost_mode {
      Some(Rect {
        x: 360.0,
        y: 200.0,
        width: 180.0,
        height: 120.0
      })
    } else {
      None
    },
    narration: if teach_mode {
      Some("Previewing the target area before applying the action.".to_string())
    } else if fast_mode {
      Some("Fast mode: minimal narration.".to_string())
    } else {
      None
    }
  }
}
