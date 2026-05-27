import sys
import re

with open(r'c:\My\Pilot AI\src-tauri\src\automation\ai.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Patch 1: Replace GeminiPlan
gemini_plan_replacement = """#[derive(Debug, Deserialize)]
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
}"""

content = re.sub(
    r'#\[derive\(Debug, Deserialize\)\]\n#\[serde\(rename_all = "camelCase"\)\]\nstruct GeminiPlan \{\n.*?\}',
    gemini_plan_replacement,
    content,
    flags=re.DOTALL
)

# Patch 2: Replace build_prompt
build_prompt_replacement = """fn build_prompt(request: &str) -> String {
  let skills = skills::registry();
  let skill_list: Vec<String> = skills.iter().map(|s| format!("{}: {}", s.id, s.description)).collect();

  format!(
    "AI planner for desktop automation. JSON only, camelCase, no markdown.\\n\\
     Output: {{\\"goal\\":\\"...\\",\\"steps\\":[{{\\"id\\":\\"step_1\\",\\"action\\":\\"...\\",\\"params\\":{{...}},\\"expectedResult\\":\\"...\\",\\"retryable\\":false,\\"explanation\\":\\"...\\",\\"requiresConfirm\\":true}}],\\"note\\":\\"...\\", \\"toolCalls\\":[{{\\"name\\":\\"check_app\\",\\"args\\":{{\\"appName\\":\\"chrome\\"}}}}]}}\\n\\
     Rules: Use only supported actions. Empty steps if unsupported.\\n\\
     **IMPORTANT:** If you need to verify if an application is installed BEFORE executing, set 'toolCalls' with 'check_app'. We will run it and return the result to you so you can fix the plan.\\n\\
     If there is any doubt or problem (e.g. app not found), or you need to ask the user, explain it in the 'note' and leave 'steps' empty. Think like a human, interact to get a clear understanding.\\n\\
     For open_app, just provide the common app name like 'notepad' or 'chrome', not the full path. To open a website, use the browser name as path and the URL as the first item in the args array.\\n\\
     If screenshot provided, use it to ground UI actions.\\n\\
     Actions: {}\\n\\
     Params: crop{{aspect?,bounds?{{x,y,w,h}}}} levels{{inputBlack?,inputWhite?,gamma?}} resize.canvas{{width,height,unit:px|percent}} export.png/jpeg{{path?,quality?}} rotate/straighten{{angle}} open_app{{path,args?}} click{{x,y,button?,clickCount?}} move_mouse{{x,y}} type_text{{text}} hotkey{{keys[]}} scroll{{amount}} wait{{ms}} ui_click{{name,controlType?,windowName?}} ui_type{{name,text,controlType?,windowName?}} ui_read{{name,controlType?,windowName?}}\\n\\
     ControlTypes: Button,Edit,Text,ComboBox,ListItem,MenuItem,CheckBox\\n\\
     Request: {}",
    skill_list.join("; "),
    request
  )
}"""

content = re.sub(
    r'fn build_prompt\(request: &str\) -> String \{.*?\}',
    build_prompt_replacement,
    content,
    flags=re.DOTALL
)

# Patch 3: Replace plan_request
plan_request_replacement = """pub async fn plan_request(
  request: &str,
  config: &GeminiConfig,
  screen_image: Option<String>
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
    
  let mut parts = build_parts(request, screen_image.as_deref());
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
              parts.push(json!({ "text": format!("System Tool Results:\\n{}\\nUse this information to either output the final plan or ask the user a question via 'note'. Do NOT output more toolCalls if you have enough information.", tool_results.join("\\n")) }));
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
}"""

# The start of plan_request is `pub async fn plan_request(...)`
content = re.sub(
    r'pub async fn plan_request\(.*?\) -> Result<PlannedActions, String> \{.*?\n\}\n\n(?=fn )',
    plan_request_replacement + '\n\n',
    content,
    flags=re.DOTALL
)

with open(r'c:\My\Pilot AI\src-tauri\src\automation\ai.rs', 'w', encoding='utf-8') as f:
    f.write(content)
