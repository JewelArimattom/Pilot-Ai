use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use once_cell::sync::Lazy;
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::{Mutex, Semaphore};
use tokio::time::sleep;

use super::action::SkillAction;
use super::skills;

// Fix 1: cheapest model as default
const DEFAULT_MODEL: &str = "gemini-2.5-flash-lite";
const DEFAULT_MAX_OUTPUT_TOKENS: u32 = 1024;
const DEFAULT_API_VERSION: &str = "v1";
const API_ROOT_BASE: &str = "https://generativelanguage.googleapis.com";
const RETRY_BASE_MS: u64 = 2000;
const RETRY_MAX_STEP: u32 = 4;
const RESPONSE_CACHE_TTL_SECS: u64 = 300; // 5 minutes
const SCREENSHOT_MAX_WIDTH: u32 = 1280;
const SCREENSHOT_JPEG_QUALITY: u32 = 60;

static GEMINI_SEMAPHORE: Lazy<Semaphore> = Lazy::new(|| Semaphore::new(1));
static KEY_COOLDOWNS: Lazy<Mutex<HashMap<String, KeyCooldown>>> =
  Lazy::new(|| Mutex::new(HashMap::new()));

// Fix 4: backend-level response cache
static RESPONSE_CACHE: Lazy<Mutex<HashMap<String, CachedResponse>>> =
  Lazy::new(|| Mutex::new(HashMap::new()));

#[derive(Debug, Clone)]
struct KeyCooldown {
  next_allowed: Instant,
  backoff_step: u32
}

#[derive(Debug, Clone)]
struct CachedResponse {
  result: PlannedActions,
  cached_at: Instant
}

#[derive(Debug, Clone)]
pub struct GeminiConfig {
  pub api_keys: Vec<String>,
  pub model: String
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
  candidates: Vec<GeminiCandidate>
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
  content: GeminiContent
}

#[derive(Debug, Deserialize)]
struct GeminiContent {
  parts: Vec<GeminiPart>
}

#[derive(Debug, Deserialize)]
struct GeminiPart {
  text: Option<String>
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiPlan {
  goal: Option<String>,
  #[serde(default)]
  steps: Vec<GeminiStep>,
  #[serde(default)]
  note: Option<String>,
  #[serde(default)]
  tool_calls: Vec<GeminiFunctionCall>
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct GeminiFunctionCall {
  name: String,
  args: Value
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiStep {
  id: Option<String>,
  action: String,
  #[serde(default)]
  params: Value,
  expected_result: Option<String>,
  explanation: Option<String>,
  retryable: Option<bool>,
  requires_confirm: Option<bool>
}

#[derive(Debug, Clone)]
pub struct PlannedStep {
  pub id: Option<String>,
  pub action: SkillAction,
  pub expected_result: Option<String>,
  pub explanation: Option<String>,
  pub retryable: bool,
  pub requires_confirm: bool
}

#[derive(Debug, Clone)]
pub struct PlannedActions {
  pub goal: Option<String>,
  pub note: Option<String>,
  pub steps: Vec<PlannedStep>
}

pub fn load_gemini_config() -> Option<GeminiConfig> {
  let keys = std::env::var("GEMINI_API_KEYS").ok()?;
  let key_list = split_keys(&keys);
  if key_list.is_empty() {
    return None;
  }

  let model = std::env::var("GEMINI_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string());

  Some(GeminiConfig {
    api_keys: key_list,
    model
  })
}

fn load_api_version() -> String {
  std::env::var("GEMINI_API_VERSION")
    .ok()
    .map(|value| value.trim().to_string())
    .filter(|value| !value.is_empty())
    .unwrap_or_else(|| DEFAULT_API_VERSION.to_string())
}

fn build_api_root() -> String {
  format!("{}/{}", API_ROOT_BASE, load_api_version())
}

fn load_max_output_tokens() -> u32 {
  std::env::var("GEMINI_MAX_OUTPUT_TOKENS")
    .ok()
    .and_then(|value| value.trim().parse::<u32>().ok())
    .filter(|value| *value > 0)
    .unwrap_or(DEFAULT_MAX_OUTPUT_TOKENS)
}

pub async fn plan_request(
  request: &str,
  config: &GeminiConfig,
  screen_image: Option<String>,
  teach_mode: bool
) -> Result<PlannedActions, String> {
  let has_image = screen_image.as_ref().map_or(false, |v| !v.trim().is_empty());
  let cache_key = request.trim().to_lowercase();
  if !has_image {
    let cache = RESPONSE_CACHE.lock().await;
    if let Some(entry) = cache.get(&cache_key) {
      if entry.cached_at.elapsed() < Duration::from_secs(RESPONSE_CACHE_TTL_SECS) {
        return Ok(entry.result.clone());
      }
    }
  }

  let _permit = GEMINI_SEMAPHORE
    .acquire()
    .await
    .map_err(|_| "Gemini request failed: concurrency guard closed".to_string())?;
  let client = Client::new();
  
  let screen_image = screen_image
    .filter(|value| !value.trim().is_empty())
    .map(|img| downscale_screenshot(&img));
    
  let mut parts = build_parts(request, screen_image.as_deref(), teach_mode);
  let max_output_tokens = load_max_output_tokens();
  let api_root = build_api_root();

  let max_iterations = 3;
  let mut iteration = 0;
  
  while iteration < max_iterations {
      iteration += 1;
      
      let payload = json!({
        "contents": [
          {
            "role": "user",
            "parts": parts
          }
        ],
        "generationConfig": {
          "temperature": 0.1,
          "maxOutputTokens": max_output_tokens
        }
      });

      let mut last_error: Option<String> = None;
      let max_attempts = config.api_keys.len().max(1);
      let mut attempts = 0;
      let mut blocked_keys: HashSet<String> = HashSet::new();

      let mut iteration_result = None;

      while attempts < max_attempts {
        let (key, wait) = next_key_or_wait(&config.api_keys, &blocked_keys).await;
        if let Some(wait) = wait {
          sleep(wait).await;
          continue;
        }
        let api_key = match key {
          Some(key) => key,
          None => break
        };

        attempts += 1;
        let url = format!("{api_root}/models/{}:generateContent?key={}", config.model, api_key);
        let redacted_url = redact_url(&url);
        
        let response = match client.post(&url).json(&payload).send().await {
          Ok(response) => response,
          Err(error) => {
            let sanitized = sanitize_error(error.to_string(), &api_key);
            last_error = Some(format!("Gemini request failed: {sanitized}"));
            let delay = mark_cooldown(&api_key).await;
            sleep(delay).await;
            continue;
          }
        };

        let status = response.status();
        if status.is_success() {
          reset_cooldown(&api_key).await;
          let data: GeminiResponse = match response.json().await {
            Ok(d) => d,
            Err(e) => {
              last_error = Some(format!("Gemini decode failed: {e}"));
              break;
            }
          };

          let text = match extract_text(&data) {
             Some(t) => t,
             None => {
                 last_error = Some("Gemini response missing text".into());
                 break;
             }
          };
          
          let json_text = match extract_json(&text) {
             Some(t) => t,
             None => {
                 last_error = Some("Gemini response missing JSON".into());
                 break;
             }
          };
          
          let plan: GeminiPlan = match serde_json::from_str(&json_text) {
             Ok(p) => p,
             Err(e) => {
                 last_error = Some(format!("Gemini JSON parse error: {e}"));
                 break;
             }
          };
          
          iteration_result = Some((plan, json_text));
          break;
        }

        let error_body = response.text().await.unwrap_or_default();
        let failure = if error_body.trim().is_empty() {
          format!("Gemini HTTP status error ({}) for url ({})", status, redacted_url)
        } else {
          format!("Gemini HTTP status error ({}) for url ({}). Resp: {}", status, redacted_url, truncate_text(error_body.trim(), 600))
        };

        if is_retryable_status(status) {
          last_error = Some(failure);
          let delay = mark_cooldown(&api_key).await;
          sleep(delay).await;
          continue;
        }

        last_error = Some(failure);
        blocked_keys.insert(api_key);
        continue;
      }
      
      let (plan, json_text) = match iteration_result {
          Some(res) => res,
          None => return Err(last_error.unwrap_or_else(|| "Gemini request failed: No API keys configured or all failed".to_string()))
      };
      
      if !plan.tool_calls.is_empty() && iteration < max_iterations {
          let mut tool_results = Vec::new();
          for tool in plan.tool_calls {
              if tool.name == "check_app" {
                  if let Some(app_name) = tool.args.get("appName").and_then(|v| v.as_str()) {
                      let resolved = super::desktop::resolve_app_path(app_name);
                      let exists = resolved != app_name || std::path::Path::new(&resolved).exists();
                      tool_results.push(format!("Tool check_app({}): installed={}, path={}", app_name, exists, resolved));
                  }
              }
          }
          if !tool_results.is_empty() {
              parts.push(json!({ "text": json_text }));
              parts.push(json!({ "text": format!("System Tool Results:
{}
Use this information to either output the final plan or ask the user a question via 'note'. Do NOT output more toolCalls if you have enough information.", tool_results.join("
")) }));
              continue;
          }
      }

      let mut steps = Vec::new();
      for step in plan.steps {
        let action = parse_action(&step.action, &step.params)?;
        steps.push(PlannedStep {
          id: step.id,
          action,
          expected_result: step.expected_result,
          explanation: step.explanation,
          retryable: step.retryable.unwrap_or(false),
          requires_confirm: step.requires_confirm.unwrap_or(true)
        });
      }

      let result = PlannedActions {
        goal: plan.goal,
        note: plan.note,
        steps
      };

      if !has_image {
        let mut cache = RESPONSE_CACHE.lock().await;
        cache.retain(|_, entry| entry.cached_at.elapsed() < Duration::from_secs(RESPONSE_CACHE_TTL_SECS));
        cache.insert(cache_key.clone(), CachedResponse {
          result: result.clone(),
          cached_at: Instant::now()
        });
      }

      return Ok(result);
  }
  
  Err("Exceeded max iterations".to_string())
}

fn build_prompt(request: &str, teach_mode: bool) -> String {
  let skills = super::skills::registry();
  let skill_list: Vec<String> = skills.iter().map(|s| format!("{}: {}", s.id, s.description)).collect();

  let mode_rules = if teach_mode {
    "**TEACHING MODE ACTIVE:** You MUST perform tasks interactively using physical mouse moves and clicks so the user can see what is happening. Describe what you're doing. Use desktop.move_mouse to hover and desktop.click to click."
  } else {
    "**EXECUTION MODE ACTIVE:** You may use programmatic methods like ui_click, ui_read, or hotkey actions for faster completion."
  };

  format!(
    "AI planner for desktop automation. JSON only, camelCase, no markdown.\n\
     Output: {{\"goal\":\"...\",\"steps\":[{{\"id\":\"step_1\",\"action\":\"...\",\"params\":{{...}},\"expectedResult\":\"...\",\"retryable\":false,\"explanation\":\"...\",\"requiresConfirm\":false}}],\"note\":\"...\"}}\n\
     Rules: Use only supported actions. Empty steps if unsupported.\n\
     **HUMAN-LIKE CONVERSATIONAL FLOW — THE MOST IMPORTANT RULE:**\n\
     You are a smart human assistant, NOT a bot. Think before acting. Always follow these 3 steps:\n\
     STEP 1 — READ: Read the FULL chat history to understand what the user wants and what info you already have.\n\
     STEP 2 — CLARIFY IF NEEDED: If the task is ambiguous or requires information you do not have yet, set steps=[] and put a friendly question in 'note'. Do NOT execute without asking first. Examples:\n\
       • User: 'change wallpaper' or 'how to change wallpaper' → Ask: 'Do you already have an image you want to use as your wallpaper, or should I open a browser and find/download a nice one for you?'\n\
       • User: 'create a file' → Ask: 'What should the file name be and where should I save it?'\n\
       • User: 'open something' → Ask: 'Which app or website would you like me to open?'\n\
     STEP 3 — EXECUTE FULLY: After the user replies (visible in chat history), generate a COMPLETE multi-step plan covering everything needed. Examples:\n\
       • Wallpaper — user said 'please download one for me': (1) desktop.check_app chrome/edge, (2) desktop.open_app the browser, (3) desktop.ui_type to go to wallpaper site e.g. wallhaven.cc, (4) guide user to right-click save an image, (5) hotkey Win+I to open Settings, (6) ui_click Personalization, (7) ui_click Background, (8) ui_click Browse photos so user can pick the downloaded file.\n\
       • Wallpaper — user said 'I have my own image': (1) hotkey Win+I, (2) ui_click Personalization, (3) ui_click Background, (4) ui_click Browse photos.\n\
     NEVER skip the clarify step for open-ended tasks. NEVER assume the user has or does not have something. Think like a helpful human colleague.\n\
     **CRITICAL — NEVER REPEAT A SUCCESSFUL STEP:**\n\
     When the log shows \"[Step N] action → SUCCESS\", that step is COMPLETE. You MUST NOT output the same action again.\n\
     • \"desktop.hotkey WIN+I → SUCCESS\" means Windows Settings IS OPEN on screen. Do NOT press Win+I again. Your next step must be ui_click 'Personalization'.\n\
     • \"desktop.open_app chrome → SUCCESS\" means Chrome IS running. Do NOT open it again. Next: navigate to a URL.\n\
     • \"desktop.ui_click Personalization → SUCCESS\" means you are on the Personalization page. Next: ui_click 'Background'.\n\
     A screenshot is provided after each step. LOOK AT IT to see what is currently visible on screen, then decide the next action based on what you see.\n\
     **AGENTIC BEHAVIOR:** Read the execution log carefully. If a step FAILED, try a different approach. If the task is done, set steps=[] and write a completion note. Never repeat a step that already succeeded.\n\
     **APP VERIFICATION:** When a specific app is needed, FIRST use desktop.check_app. If not found, ask the user which app to use instead.\n\
     For open_app, use the common name like 'chrome' or 'notepad'. To open a URL, pass it as the first arg.\n\
     {mode_rules}\n\
     If a screenshot is provided, use it to read what is on screen and ground your UI actions.\n\
     Actions: {}\n\
     Params: check_app{{appName}} open_app{{path,args?}} click{{x,y,button?,clickCount?}} move_mouse{{x,y}} type_text{{text}} hotkey{{keys[]}} scroll{{amount}} wait{{ms}} ui_click{{name,controlType?,windowName?}} ui_type{{name,text,controlType?,windowName?}} ui_read{{name,controlType?,windowName?}}\n\
     ControlTypes: Button,Edit,Text,ComboBox,ListItem,MenuItem,CheckBox\n\
     Request: {}",
    skill_list.join("; "),
    request
  )
}

fn parse_action(action: &str, params: &serde_json::Value) -> Result<SkillAction, String> {
  let action = action.trim().to_lowercase();
  let payload = json!({
    "action": action,
    "params": params
  });
  serde_json::from_value(payload).map_err(|error| format!("Invalid action or params: {error}"))
}

fn split_keys(value: &str) -> Vec<String> {
  value
    .split(',')
    .map(|key| key.trim())
    .filter(|key| !key.is_empty())
    .map(|key| key.to_string())
    .collect()
}

fn build_parts(request: &str, screen_image: Option<&str>, teach_mode: bool) -> Vec<Value> {
  let mut parts = vec![json!({ "text": build_prompt(request, teach_mode) })];
  if let Some(screen_image) = screen_image {
    // Fix 3: send as JPEG (already downscaled at this point)
    parts.push(json!({
      "inlineData": {
        "mimeType": "image/jpeg",
        "data": screen_image
      }
    }));
  }

  parts
}

/// Fix 3: downscale a base64 PNG screenshot to a smaller JPEG via PowerShell.
/// Falls back to the original image on any error.
fn downscale_screenshot(base64_png: &str) -> String {
  let script = format!(
    r#"Add-Type -AssemblyName System.Drawing
$bytes = [System.Convert]::FromBase64String('{base64_input}')
$ms = New-Object System.IO.MemoryStream(,$bytes)
$img = [System.Drawing.Image]::FromStream($ms)
$maxW = {max_width}
if ($img.Width -le $maxW) {{
  $nw = $img.Width; $nh = $img.Height
}} else {{
  $ratio = $maxW / $img.Width
  $nw = $maxW; $nh = [int]($img.Height * $ratio)
}}
$bmp = New-Object System.Drawing.Bitmap($nw, $nh)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
$g.DrawImage($img, 0, 0, $nw, $nh)
$enc = [System.Drawing.Imaging.ImageCodecInfo]::GetImageEncoders() | Where-Object {{ $_.MimeType -eq 'image/jpeg' }}
$ep = New-Object System.Drawing.Imaging.EncoderParameters(1)
$ep.Param[0] = New-Object System.Drawing.Imaging.EncoderParameter([System.Drawing.Imaging.Encoder]::Quality, [long]{quality})
$out = New-Object System.IO.MemoryStream
$bmp.Save($out, $enc, $ep)
[System.Convert]::ToBase64String($out.ToArray())"#,
    base64_input = base64_png.trim(),
    max_width = SCREENSHOT_MAX_WIDTH,
    quality = SCREENSHOT_JPEG_QUALITY
  );

  let output = std::process::Command::new("powershell")
    .args(["-NoProfile", "-NonInteractive", "-Command", &script])
    .output();

  match output {
    Ok(out) if out.status.success() => {
      let result = String::from_utf8_lossy(&out.stdout).trim().to_string();
      if result.is_empty() {
        base64_png.to_string()
      } else {
        result
      }
    }
    _ => base64_png.to_string()
  }
}

fn redact_url(url: &str) -> String {
  if let Some(index) = url.find("?key=") {
    return format!("{}?key=REDACTED", &url[..index]);
  }

  url.to_string()
}

fn sanitize_error(error: String, api_key: &str) -> String {
  if api_key.is_empty() {
    return error;
  }

  error.replace(api_key, "REDACTED")
}

fn truncate_text(text: &str, max_len: usize) -> String {
  if text.len() <= max_len {
    return text.to_string();
  }

  let mut snippet = text[..max_len].to_string();
  snippet.push_str("...");
  snippet
}

fn backoff_duration(step: u32) -> Duration {
  let exponent = step.min(RETRY_MAX_STEP.saturating_sub(1));
  let delay_ms = RETRY_BASE_MS.saturating_mul(1u64 << exponent);
  Duration::from_millis(delay_ms)
}

fn is_retryable_status(status: StatusCode) -> bool {
  status == StatusCode::TOO_MANY_REQUESTS
    || status == StatusCode::INTERNAL_SERVER_ERROR
    || status == StatusCode::SERVICE_UNAVAILABLE
}

async fn next_key_or_wait(keys: &[String], blocked: &HashSet<String>) -> (Option<String>, Option<Duration>) {
  if keys.is_empty() {
    return (None, None);
  }

  let now = Instant::now();
  let mut earliest: Option<Duration> = None;
  let map = KEY_COOLDOWNS.lock().await;
  for key in keys {
    if blocked.contains(key) {
      continue;
    }
    match map.get(key) {
      None => return (Some(key.clone()), None),
      Some(state) => {
        if state.next_allowed <= now {
          return (Some(key.clone()), None);
        }

        let wait = state.next_allowed.saturating_duration_since(now);
        earliest = Some(earliest.map_or(wait, |current| current.min(wait)));
      }
    }
  }

  (None, earliest)
}

async fn mark_cooldown(key: &str) -> Duration {
  let mut map = KEY_COOLDOWNS.lock().await;
  let entry = map.entry(key.to_string()).or_insert(KeyCooldown {
    next_allowed: Instant::now(),
    backoff_step: 0
  });

  let delay = backoff_duration(entry.backoff_step);
  entry.backoff_step = (entry.backoff_step + 1).min(RETRY_MAX_STEP.saturating_sub(1));
  entry.next_allowed = Instant::now() + delay;
  delay
}

async fn reset_cooldown(key: &str) {
  let mut map = KEY_COOLDOWNS.lock().await;
  map.remove(key);
}

fn extract_text(response: &GeminiResponse) -> Option<String> {
  let candidate = response.candidates.first()?;
  let mut text = String::new();
  for part in &candidate.content.parts {
    if let Some(fragment) = &part.text {
      text.push_str(fragment);
    }
  }

  if text.trim().is_empty() {
    None
  } else {
    Some(text)
  }
}

fn extract_json(text: &str) -> Option<String> {
  let start = text.find('{')?;
  let end = text.rfind('}')?;
  if end < start {
    return None;
  }

  Some(text[start..=end].to_string())
}

