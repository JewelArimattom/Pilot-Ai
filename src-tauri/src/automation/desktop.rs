use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;

use enigo::{Enigo, Key, KeyboardControllable, MouseButton, MouseControllable};

use super::action::{ActionPlan, SkillAction};
use super::executor::{ExecutionReport, StepResult, StepStatus};
use super::safety::{validate_plan, SafetyPolicy};
use tauri::Manager;

pub fn execute_plan(app: tauri::AppHandle, plan: &ActionPlan, policy: &SafetyPolicy) -> ExecutionReport {
  if let Err(error) = validate_plan(plan, policy) {
    return failure_report(plan, &error);
  }

  let mut enigo = Enigo::new();
  let mut results = Vec::new();
  let mut failed = false;

  for step in &plan.steps {
    if let Some(msg) = &step.note {
      let _ = app.emit_all("overlay-say", serde_json::json!({ "text": msg }));
    }
    
    let outcome = match execute_action(&app, &mut enigo, &step.action) {
      Ok(message) => StepResult {
        step_id: step.step_id.clone(),
        status: StepStatus::Completed,
        message: message.or_else(|| Some("Executed".to_string()))
      },
      Err(error) => {
        failed = true;
        StepResult {
          step_id: step.step_id.clone(),
          status: StepStatus::Failed,
          message: Some(error)
        }
      }
    };
    results.push(outcome);
  }

  ExecutionReport {
    plan_id: plan.plan_id.clone(),
    summary: if failed {
      "Desktop automation finished with errors.".to_string()
    } else {
      format!("Desktop automation completed with {} step(s).", plan.steps.len())
    },
    blocked: failed,
    results
  }
}

fn execute_action(app: &tauri::AppHandle, enigo: &mut Enigo, action: &SkillAction) -> Result<Option<String>, String> {
  match action {
    SkillAction::DesktopOpenApp { path, args } => {
      let mut command = Command::new("cmd");
      command.arg("/c").arg("start").arg("");
      
      // If it looks like a URL, just open it directly
      if path.starts_with("http") || path.contains("://") {
        command.arg(path);
      } else {
        // Try resolving, if fails, use the raw path (for things like "ms-settings:*" or "chrome")
        let resolved = resolve_app_path(path);
        command.arg(&resolved);
      }
      
      if let Some(args) = args {
        for arg in args {
          command.arg(arg);
        }
      }
      
      command
        .spawn()
        .map_err(|error| format!("Failed to open '{}': {}", path, error))?;
      Ok(Some(format!("App launched: {}", path)))
    }
    SkillAction::DesktopCheckApp { app_name } => {
      let resolved = resolve_app_path(app_name);
      if resolved == *app_name && !resolved.contains(':') && !resolved.contains('\\') {
        // If it didn't resolve to a full path and isn't a special URI, it might not be installed.
        Ok(Some(format!("App '{}' is likely NOT installed or not in PATH.", app_name)))
      } else {
        Ok(Some(format!("App '{}' found at {}", app_name, resolved)))
      }
    }
    SkillAction::DesktopClick {
      x,
      y,
      button,
      click_count
    } => {
      let _ = app.emit_all("overlay-move", serde_json::json!({ "x": *x, "y": *y }));
      thread::sleep(Duration::from_millis(300));
      enigo.mouse_move_to(*x, *y);
      let _ = app.emit_all("overlay-click", ());
      let button = parse_button(button.as_deref())?;
      let count = click_count.unwrap_or(1).max(1);
      for _ in 0..count {
        enigo.mouse_click(button);
      }
      Ok(None)
    }
    SkillAction::DesktopMoveMouse { x, y } => {
      let _ = app.emit_all("overlay-move", serde_json::json!({ "x": *x, "y": *y }));
      thread::sleep(Duration::from_millis(150));
      enigo.mouse_move_to(*x, *y);
      Ok(None)
    }
    SkillAction::DesktopTypeText { text } => {
      enigo.key_sequence(text);
      Ok(None)
    }
    SkillAction::DesktopHotkey { keys } => {
      send_hotkey(enigo, keys)?;
      Ok(None)
    }
    SkillAction::DesktopScroll { amount } => {
      enigo.mouse_scroll_y(*amount);
      Ok(None)
    }
    SkillAction::DesktopWait { ms } => {
      thread::sleep(Duration::from_millis(*ms));
      Ok(None)
    }
    SkillAction::DesktopUiClick {
      name,
      control_type,
      window_name
    } => {
      let res = run_ui_action("click", name, control_type.as_deref(), window_name.as_deref(), None)?;
      if res.starts_with("POINT:") {
        let parts: Vec<&str> = res["POINT:".len()..].split(',').collect();
        if parts.len() == 2 {
          if let (Ok(x), Ok(y)) = (parts[0].trim().parse::<i32>(), parts[1].trim().parse::<i32>()) {
            let _ = app.emit_all("overlay-move", serde_json::json!({ "x": x, "y": y }));
            thread::sleep(Duration::from_millis(300));
            enigo.mouse_move_to(x, y);
            thread::sleep(Duration::from_millis(150));
            let _ = app.emit_all("overlay-click", ());
            enigo.mouse_click(MouseButton::Left);
            return Ok(Some("Physically clicked UI element".to_string()));
          }
        }
      }
      Ok(Some("Clicked UI element".to_string()))
    }
    SkillAction::DesktopUiType {
      name,
      text,
      control_type,
      window_name
    } => {
      let res = run_ui_action(
        "type",
        name,
        control_type.as_deref(),
        window_name.as_deref(),
        Some(text)
      )?;
      if res.starts_with("POINT:") {
        let lines: Vec<&str> = res.split('\n').collect();
        let point_line = lines[0];
        let parts: Vec<&str> = point_line["POINT:".len()..].split(',').collect();
        if parts.len() == 2 {
          if let (Ok(x), Ok(y)) = (parts[0].trim().parse::<i32>(), parts[1].trim().parse::<i32>()) {
            let _ = app.emit_all("overlay-move", serde_json::json!({ "x": x, "y": y }));
            thread::sleep(Duration::from_millis(300));
            enigo.mouse_move_to(x, y);
            thread::sleep(Duration::from_millis(150));
          }
        }
      }
      Ok(Some("Typed into UI element".to_string()))
    }
    SkillAction::DesktopUiRead {
      name,
      control_type,
      window_name
    } => {
      let value = run_ui_action("read", name, control_type.as_deref(), window_name.as_deref(), None)?;
      Ok(Some(format!("Read: {}", value)))
    }
    _ => Err("Unsupported desktop action".to_string())
  }
}

fn parse_button(button: Option<&str>) -> Result<MouseButton, String> {
  let normalized = button.unwrap_or("left").trim().to_lowercase();
  match normalized.as_str() {
    "left" => Ok(MouseButton::Left),
    "right" => Ok(MouseButton::Right),
    "middle" => Ok(MouseButton::Middle),
    value => Err(format!("Unsupported mouse button: {value}"))
  }
}

fn send_hotkey(enigo: &mut Enigo, keys: &[String]) -> Result<(), String> {
  let mut parsed = Vec::new();
  for raw in keys {
    for part in raw.split('+') {
      let trimmed = part.trim();
      if trimmed.is_empty() {
        continue;
      }
      let key = parse_key(trimmed)?;
      parsed.push(key);
    }
  }

  if parsed.is_empty() {
    return Err("Hotkey list is empty".to_string());
  }

  if parsed.len() == 1 {
    enigo.key_click(parsed[0]);
    return Ok(());
  }

  for key in parsed.iter().take(parsed.len() - 1) {
    enigo.key_down(*key);
  }
  let last = parsed[parsed.len() - 1];
  enigo.key_click(last);
  for key in parsed.iter().take(parsed.len() - 1).rev() {
    enigo.key_up(*key);
  }

  Ok(())
}

fn parse_key(key: &str) -> Result<Key, String> {
  let normalized = key.trim().to_uppercase();
  let mapped = match normalized.as_str() {
    "CTRL" | "CONTROL" => Key::Control,
    "SHIFT" => Key::Shift,
    "ALT" => Key::Alt,
    "WIN" | "META" | "CMD" | "COMMAND" => Key::Meta,
    "ENTER" | "RETURN" => Key::Return,
    "TAB" => Key::Tab,
    "ESC" | "ESCAPE" => Key::Escape,
    "SPACE" => Key::Space,
    "BACKSPACE" => Key::Backspace,
    "DELETE" | "DEL" => Key::Delete,
    "UP" => Key::UpArrow,
    "DOWN" => Key::DownArrow,
    "LEFT" => Key::LeftArrow,
    "RIGHT" => Key::RightArrow,
    "HOME" => Key::Home,
    "END" => Key::End,
    "PAGEUP" | "PGUP" => Key::PageUp,
    "PAGEDOWN" | "PGDN" => Key::PageDown,
    _ => {
      if normalized.len() == 1 {
        let ch = normalized.chars().next().unwrap();
        Key::Layout(ch)
      } else {
        return Err(format!("Unsupported hotkey: {key}"));
      }
    }
  };

  Ok(mapped)
}

fn run_ui_action(
  action: &str,
  name: &str,
  control_type: Option<&str>,
  window_name: Option<&str>,
  text: Option<&str>
) -> Result<String, String> {
  // Retry up to 3 times with 900 ms between attempts so that transient
  // UI elements (menus, dialogs opening slowly) have time to appear.
  let max_attempts = 3u32;
  let mut last_err = String::new();

  for attempt in 0..max_attempts {
    if attempt > 0 {
      thread::sleep(Duration::from_millis(900));
    }
    match run_ui_action_once(action, name, control_type, window_name, text) {
      Ok(v) => return Ok(v),
      Err(e) => last_err = e,
    }
  }
  Err(last_err)
}

fn run_ui_action_once(
  action: &str,
  name: &str,
  control_type: Option<&str>,
  window_name: Option<&str>,
  text: Option<&str>
) -> Result<String, String> {
  // PowerShell script: find a UI Automation element and act on it.
  // Uses env vars so the script source is stable (no shell-injection risk).
  let script = r#"$action = $env:PILOT_UI_ACTION
$name = $env:PILOT_UI_NAME
$controlType = $env:PILOT_UI_CONTROL_TYPE
$windowName = $env:PILOT_UI_WINDOW_NAME
$text = $env:PILOT_UI_TEXT

Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -AssemblyName System.Windows.Forms

$root = [System.Windows.Automation.AutomationElement]::RootElement
$searchRoot = $root
if ($windowName) {
  $windowCondition = New-Object System.Windows.Automation.PropertyCondition(
    [System.Windows.Automation.AutomationElement]::NameProperty,
    $windowName
  )
  $window = $root.FindFirst([System.Windows.Automation.TreeScope]::Children, $windowCondition)
  if (-not $window) {
    Write-Error "Window not found: $windowName"
    exit 2
  }
  $searchRoot = $window
}

$conditions = @()
if ($name) {
  $conditions += New-Object System.Windows.Automation.PropertyCondition(
    [System.Windows.Automation.AutomationElement]::NameProperty,
    $name
  )
}
if ($controlType) {
  $ct = [System.Windows.Automation.ControlType]::$controlType
  $conditions += New-Object System.Windows.Automation.PropertyCondition(
    [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
    $ct
  )
}
if ($conditions.Count -eq 0) {
  Write-Error "No search criteria provided"
  exit 2
}
if ($conditions.Count -eq 1) {
  $condition = $conditions[0]
} else {
  $condition = New-Object System.Windows.Automation.AndCondition($conditions)
}

$element = $searchRoot.FindFirst([System.Windows.Automation.TreeScope]::Subtree, $condition)
if (-not $element) {
  Write-Error "Element not found: name=$name controlType=$controlType window=$windowName"
  exit 2
}

function TryGetPattern($el, $pat) {
  try { return $el.GetCurrentPattern($pat) } catch { return $null }
}

switch ($action) {
  "click" {
    $pt = New-Object System.Windows.Point
    if ($element.TryGetClickablePoint([ref]$pt)) {
      "POINT:$([int]$pt.X),$([int]$pt.Y)"
      exit 0
    }
    $invoke = TryGetPattern $element ([System.Windows.Automation.InvokePattern]::Pattern)
    if ($invoke) { $invoke.Invoke(); "clicked"; exit 0 }
    $element.SetFocus()
    [System.Windows.Forms.SendKeys]::SendWait("{ENTER}")
    "invoked"; exit 0
  }
  "type" {
    if (-not $text) { Write-Error "Missing text"; exit 2 }
    $pt = New-Object System.Windows.Point
    if ($element.TryGetClickablePoint([ref]$pt)) {
      Write-Output "POINT:$([int]$pt.X),$([int]$pt.Y)"
    }
    $value = TryGetPattern $element ([System.Windows.Automation.ValuePattern]::Pattern)
    if ($value) { $value.SetValue($text); "typed"; exit 0 }
    $element.SetFocus()
    [System.Windows.Forms.SendKeys]::SendWait($text)
    "typed"; exit 0
  }
  "read" {
    $value = TryGetPattern $element ([System.Windows.Automation.ValuePattern]::Pattern)
    if ($value) { $value.Current.Value; exit 0 }
    $tp = TryGetPattern $element ([System.Windows.Automation.TextPattern]::Pattern)
    if ($tp) { $tp.DocumentRange.GetText(-1); exit 0 }
    $element.Current.Name; exit 0
  }
  default { Write-Error "Unknown action: $action"; exit 2 }
}
"#;

  let mut command = Command::new("powershell");
  command.args(["-NoProfile", "-NonInteractive", "-Command", script]);
  command.env("PILOT_UI_ACTION", action);
  command.env("PILOT_UI_NAME", name);
  command.env("PILOT_UI_CONTROL_TYPE", control_type.unwrap_or(""));
  command.env("PILOT_UI_WINDOW_NAME", window_name.unwrap_or(""));
  command.env("PILOT_UI_TEXT", text.unwrap_or(""));

  let output = command
    .output()
    .map_err(|error| format!("UI automation launch failed: {error}"))?;

  if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    return Err(if stderr.is_empty() {
      "UI automation failed (no details).".to_string()
    } else {
      format!("UI automation failed: {stderr}")
    });
  }

  Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn failure_report(plan: &ActionPlan, summary: &str) -> ExecutionReport {
  ExecutionReport {
    plan_id: plan.plan_id.clone(),
    summary: summary.to_string(),
    blocked: true,
    results: Vec::new()
  }
}

pub fn resolve_app_path(name: &str) -> String {
  let name = name.trim();

  // 1. If it's already an absolute path that exists, use it
  if Path::new(name).is_absolute() && Path::new(name).exists() {
    return name.to_string();
  }

  // 2. Try system PATH via `where` command
  if let Some(path) = find_in_path(name) {
    return path;
  }

  // 3. Try Start Menu shortcut scan
  if let Some(path) = find_in_start_menu(name) {
    return path;
  }

  // 4. Try common install directories
  if let Some(path) = find_in_common_dirs(name) {
    return path;
  }

  // 5. Fallback: return as-is (let the OS try to resolve it)
  name.to_string()
}

/// Use the Windows `where` command to find an executable on the system PATH.
fn find_in_path(name: &str) -> Option<String> {
  // Try the name as given, and with .exe appended
  let candidates = if name.contains('.') {
    vec![name.to_string()]
  } else {
    vec![name.to_string(), format!("{}.exe", name)]
  };

  for candidate in candidates {
    let output = Command::new("where")
      .arg(&candidate)
      .output()
      .ok()?;
    if output.status.success() {
      let stdout = String::from_utf8_lossy(&output.stdout);
      if let Some(first_line) = stdout.lines().next() {
        let path = first_line.trim();
        if !path.is_empty() && Path::new(path).exists() {
          return Some(path.to_string());
        }
      }
    }
  }

  None
}

/// Scan Start Menu shortcuts to find an app by name.
/// Uses PowerShell to resolve .lnk shortcut targets.
fn find_in_start_menu(name: &str) -> Option<String> {
  let lower = name.to_lowercase().replace(".exe", "");
  let script = format!(
    r#"$shell = New-Object -ComObject WScript.Shell
$dirs = @(
  [System.Environment]::GetFolderPath('CommonStartMenu'),
  [System.Environment]::GetFolderPath('StartMenu')
)
foreach ($dir in $dirs) {{
  Get-ChildItem -Path $dir -Recurse -Filter '*.lnk' -ErrorAction SilentlyContinue | ForEach-Object {{
    $lnk = $shell.CreateShortcut($_.FullName)
    $target = $lnk.TargetPath
    if ($target -and ($_.BaseName -like '*{search}*' -or [System.IO.Path]::GetFileNameWithoutExtension($target) -like '*{search}*')) {{
      if (Test-Path $target) {{
        Write-Output $target
        return
      }}
    }}
  }}
}}"#,
    search = lower
  );

  let output = Command::new("powershell")
    .args(["-NoProfile", "-NonInteractive", "-Command", &script])
    .output()
    .ok()?;

  if output.status.success() {
    let stdout = String::from_utf8_lossy(&output.stdout);
    if let Some(first_line) = stdout.lines().next() {
      let path = first_line.trim();
      if !path.is_empty() && Path::new(path).exists() {
        return Some(path.to_string());
      }
    }
  }

  None
}

/// Check common Windows install directories for the executable.
fn find_in_common_dirs(name: &str) -> Option<String> {
  let lower = name.to_lowercase().replace(".exe", "");
  let exe_name = format!("{}.exe", lower);

  // Build list of common directories to search
  let mut search_dirs: Vec<PathBuf> = Vec::new();

  if let Ok(pf) = std::env::var("ProgramFiles") {
    search_dirs.push(PathBuf::from(&pf));
  }
  if let Ok(pf86) = std::env::var("ProgramFiles(x86)") {
    search_dirs.push(PathBuf::from(&pf86));
  }
  if let Ok(local) = std::env::var("LOCALAPPDATA") {
    search_dirs.push(PathBuf::from(&local));
  }
  search_dirs.push(PathBuf::from(r"C:\Windows"));
  search_dirs.push(PathBuf::from(r"C:\Windows\System32"));

  for dir in &search_dirs {
    // Direct match: dir/name.exe
    let direct = dir.join(&exe_name);
    if direct.exists() {
      return Some(direct.to_string_lossy().to_string());
    }
    // Subfolder match: dir/name/name.exe (common pattern like C:\Program Files\App\App.exe)
    let sub = dir.join(&lower).join(&exe_name);
    if sub.exists() {
      return Some(sub.to_string_lossy().to_string());
    }
  }

  None
}
