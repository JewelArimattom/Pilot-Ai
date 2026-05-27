use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::action::{ActionPlan, Rect, SkillAction};

const EXECUTABLE_NAME: &str = "Photoshop.exe";

pub fn detect_paths() -> Vec<String> {
  let mut paths: Vec<String> = Vec::new();
  let mut seen = HashSet::new();

  if let Ok(value) = std::env::var("PHOTOSHOP_PATH") {
    let trimmed = value.trim();
    if !trimmed.is_empty() {
      let normalized = trimmed.to_string();
      if Path::new(&normalized).exists() && seen.insert(normalized.clone()) {
        paths.push(normalized);
      }
    }
  }

  let roots = ["ProgramFiles", "ProgramFiles(x86)"];
  for root_key in roots {
    if let Ok(root) = std::env::var(root_key) {
      let adobe_dir = Path::new(&root).join("Adobe");
      if let Ok(entries) = fs::read_dir(adobe_dir) {
        for entry in entries.flatten() {
          let path = entry.path();
          if !path.is_dir() {
            continue;
          }
          let name = entry.file_name();
          let name = name.to_string_lossy();
          if !name.starts_with("Adobe Photoshop") {
            continue;
          }

          let exe_path = path.join(EXECUTABLE_NAME);
          if exe_path.exists() {
            let value = exe_path.to_string_lossy().to_string();
            if seen.insert(value.clone()) {
              paths.push(value);
            }
          }
        }
      }
    }
  }

  paths
}

pub fn build_script(plan: &ActionPlan) -> Result<String, String> {
  let mut lines: Vec<String> = Vec::new();
  lines.push("#target photoshop".to_string());
  lines.push("app.displayDialogs = DialogModes.NO;".to_string());
  lines.push("function requireDocument() {".to_string());
  lines.push("  if (app.documents.length === 0) {".to_string());
  lines.push("    throw new Error('No active document');".to_string());
  lines.push("  }".to_string());
  lines.push("  return app.activeDocument;".to_string());
  lines.push("}".to_string());
  lines.push("".to_string());
  lines.push("try {".to_string());
  lines.push("  var doc = requireDocument();".to_string());

  for step in &plan.steps {
    let script = build_step_script(&step.action)?;
    for line in script {
      lines.push(format!("  {line}"));
    }
  }

  lines.push("} catch (error) {".to_string());
  lines.push("  alert('Pilot AI error: ' + error.message);".to_string());
  lines.push("}".to_string());

  Ok(lines.join("\n"))
}

pub fn write_script(script: &str) -> Result<PathBuf, String> {
  let filename = format!("pilot-ai-{}.jsx", timestamp_millis());
  let path = std::env::temp_dir().join(filename);
  fs::write(&path, script).map_err(|error| format!("Script write error: {error}"))?;
  Ok(path)
}

pub fn run_script(photoshop_path: &str, script_path: &Path) -> Result<(), String> {
  let mut command = Command::new(photoshop_path);
  command.arg("-r").arg(script_path);
  command.spawn().map_err(|error| format!("Photoshop launch error: {error}"))?;
  Ok(())
}

fn build_step_script(action: &SkillAction) -> Result<Vec<String>, String> {
  match action {
    SkillAction::PhotoshopCrop { aspect, bounds } => Ok(build_crop_script(aspect.as_deref(), bounds)),
    SkillAction::PhotoshopAdjustLevels {
      input_black,
      input_white,
      gamma
    } => Ok(build_levels_script(*input_black, *input_white, *gamma)),
    SkillAction::PhotoshopExportPng { path, quality } => Ok(build_export_script("png", path, *quality)),
    SkillAction::PhotoshopExportJpeg { path, quality } => Ok(build_export_script("jpeg", path, *quality)),
    SkillAction::PhotoshopResizeCanvas { width, height, unit } => {
      Ok(build_resize_script(*width, *height, unit))
    }
    SkillAction::PhotoshopRotate { angle } => Ok(build_rotate_script(*angle)),
    SkillAction::PhotoshopStraighten { angle } => Ok(build_rotate_script(*angle)),
    _ => Err("Unsupported action for Photoshop executor".to_string())
  }
}

fn build_crop_script(aspect: Option<&str>, bounds: &Option<Rect>) -> Vec<String> {
  if let Some(bounds) = bounds {
    return vec![format!(
      "doc.crop([{},{},{},{}]);",
      bounds.x,
      bounds.y,
      bounds.x + bounds.width,
      bounds.y + bounds.height
    )];
  }

  let mut lines = Vec::new();
  lines.push("var w = doc.width.as('px');".to_string());
  lines.push("var h = doc.height.as('px');".to_string());
  if let Some(aspect_value) = aspect {
    if aspect_value.contains("square") || aspect_value.contains("1:1") {
      lines.push("var size = Math.min(w, h) * 0.9;".to_string());
    } else {
      lines.push("var size = Math.min(w, h) * 0.9;".to_string());
    }
  } else {
    lines.push("var size = Math.min(w, h) * 0.9;".to_string());
  }
  lines.push("var left = (w - size) / 2;".to_string());
  lines.push("var top = (h - size) / 2;".to_string());
  lines.push("doc.crop([left, top, left + size, top + size]);".to_string());
  lines
}

fn build_levels_script(input_black: Option<f32>, input_white: Option<f32>, gamma: Option<f32>) -> Vec<String> {
  let input_black = input_black.unwrap_or(0.0);
  let input_white = input_white.unwrap_or(255.0);
  let gamma = gamma.unwrap_or(1.0);
  vec![
    "var layer = doc.activeLayer;".to_string(),
    "if (layer && layer.adjustLevels) {".to_string(),
    format!(
      "  layer.adjustLevels({}, {}, {}, 0, 255);",
      input_black, input_white, gamma
    ),
    "} else {".to_string(),
    "  throw new Error('Active layer does not support levels.');".to_string(),
    "}".to_string()
  ]
}

fn build_resize_script(width: i32, height: i32, unit: &str) -> Vec<String> {
  let unit = unit.to_lowercase();
  let mut lines = Vec::new();
  lines.push(format!("var widthValue = {width};"));
  lines.push(format!("var heightValue = {height};"));
  lines.push(format!("var unit = '{unit}';"));
  lines.push("if (unit === 'percent') {".to_string());
  lines.push(
    "  doc.resizeCanvas(UnitValue(widthValue, '%'), UnitValue(heightValue, '%'), AnchorPosition.MIDDLECENTER);"
      .to_string()
  );
  lines.push("} else {".to_string());
  lines.push(
    "  doc.resizeCanvas(UnitValue(widthValue, 'px'), UnitValue(heightValue, 'px'), AnchorPosition.MIDDLECENTER);"
      .to_string()
  );
  lines.push("}".to_string());
  lines
}

fn build_export_script(format: &str, path: &Option<String>, quality: Option<u8>) -> Vec<String> {
  let mut lines = Vec::new();
  lines.push("var exportFile;".to_string());
  if let Some(path) = path {
    lines.push(format!("exportFile = new File('{}');", escape_js_path(path)));
  } else {
    lines.push("var baseName = doc.name.replace(/\\.[^\\.]+$/, '');".to_string());
    lines.push("var stamp = new Date().getTime();".to_string());
    lines.push(format!(
      "exportFile = new File(Folder.desktop.fsName + '/' + baseName + '{}_' + stamp + '.{}');",
      if format == "png" { "_pilot" } else { "_pilot" },
      if format == "png" { "png" } else { "jpg" }
    ));
  }

  lines.push("var options = new ExportOptionsSaveForWeb();".to_string());
  if format == "png" {
    lines.push("options.format = SaveDocumentType.PNG;".to_string());
    lines.push("options.PNG8 = false;".to_string());
    lines.push("options.transparency = true;".to_string());
  } else {
    let quality_value = quality.unwrap_or(92).min(100).max(1);
    lines.push("options.format = SaveDocumentType.JPEG;".to_string());
    lines.push(format!("options.quality = {};", quality_value));
    lines.push("options.optimized = true;".to_string());
  }
  lines.push("doc.exportDocument(exportFile, ExportType.SAVEFORWEB, options);".to_string());
  lines.push("".to_string());
  lines
}

fn build_rotate_script(angle: f32) -> Vec<String> {
  vec![format!("doc.rotateCanvas({});", angle)]
}

fn escape_js_path(path: &str) -> String {
  path
    .replace('\\', "\\\\")
    .replace('"', "\\\"")
    .replace('\'', "\\'")
}

fn timestamp_millis() -> u128 {
  use std::time::{SystemTime, UNIX_EPOCH};
  SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap_or_default()
    .as_millis()
}
