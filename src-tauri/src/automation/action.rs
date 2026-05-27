use serde::{Deserialize, Serialize};

use super::state::ExpectedState;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", content = "params", rename_all = "camelCase")]
pub enum SkillAction {
  #[serde(rename = "photoshop.crop")]
  PhotoshopCrop {
    aspect: Option<String>,
    bounds: Option<Rect>
  },
  #[serde(rename = "photoshop.adjust.levels")]
  PhotoshopAdjustLevels {
    #[serde(rename = "inputBlack")]
    input_black: Option<f32>,
    #[serde(rename = "inputWhite")]
    input_white: Option<f32>,
    gamma: Option<f32>
  },
  #[serde(rename = "photoshop.export.png")]
  PhotoshopExportPng {
    path: Option<String>,
    quality: Option<u8>
  },
  #[serde(rename = "photoshop.export.jpeg")]
  PhotoshopExportJpeg {
    path: Option<String>,
    quality: Option<u8>
  },
  #[serde(rename = "photoshop.resize.canvas")]
  PhotoshopResizeCanvas {
    width: i32,
    height: i32,
    unit: String
  },
  #[serde(rename = "photoshop.rotate")]
  PhotoshopRotate {
    angle: f32
  },
  #[serde(rename = "photoshop.straighten")]
  PhotoshopStraighten {
    angle: f32
  },
  #[serde(rename = "desktop.open_app")]
  DesktopOpenApp {
    path: String,
    args: Option<Vec<String>>
  },
  #[serde(rename = "desktop.check_app")]
  DesktopCheckApp {
    #[serde(rename = "appName")]
    app_name: String
  },
  #[serde(rename = "desktop.click")]
  DesktopClick {
    x: i32,
    y: i32,
    button: Option<String>,
    #[serde(rename = "clickCount")]
    click_count: Option<u8>
  },
  #[serde(rename = "desktop.move_mouse")]
  DesktopMoveMouse {
    x: i32,
    y: i32
  },
  #[serde(rename = "desktop.type_text")]
  DesktopTypeText {
    text: String
  },
  #[serde(rename = "desktop.hotkey")]
  DesktopHotkey {
    keys: Vec<String>
  },
  #[serde(rename = "desktop.scroll")]
  DesktopScroll {
    amount: i32
  },
  #[serde(rename = "desktop.wait")]
  DesktopWait {
    ms: u64
  },
  #[serde(rename = "desktop.ui_click")]
  DesktopUiClick {
    name: String,
    #[serde(rename = "controlType")]
    control_type: Option<String>,
    #[serde(rename = "windowName")]
    window_name: Option<String>
  },
  #[serde(rename = "desktop.ui_type")]
  DesktopUiType {
    name: String,
    text: String,
    #[serde(rename = "controlType")]
    control_type: Option<String>,
    #[serde(rename = "windowName")]
    window_name: Option<String>
  },
  #[serde(rename = "desktop.ui_read")]
  DesktopUiRead {
    name: String,
    #[serde(rename = "controlType")]
    control_type: Option<String>,
    #[serde(rename = "windowName")]
    window_name: Option<String>
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GhostPreview {
  pub cursor_path: Vec<Point>,
  pub highlight: Option<Rect>,
  pub narration: Option<String>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionStep {
  pub step_id: String,
  pub requires_confirm: bool,
  pub action: SkillAction,
  #[serde(default)]
  pub expected_result: Option<String>,
  #[serde(default)]
  pub retryable: bool,
  #[serde(default)]
  pub explanation: Option<String>,
  pub expected: Vec<ExpectedState>,
  pub ghost_preview: GhostPreview,
  pub note: Option<String>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionPlan {
  pub plan_id: String,
  pub created_at: String,
  pub goal: Option<String>,
  pub status: PlanStatus,
  pub steps: Vec<ActionStep>,
  pub note: Option<String>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Point {
  pub x: f32,
  pub y: f32
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rect {
  pub x: f32,
  pub y: f32,
  pub width: f32,
  pub height: f32
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PlanStatus {
  Ready,
  NeedsUser,
  Blocked
}
