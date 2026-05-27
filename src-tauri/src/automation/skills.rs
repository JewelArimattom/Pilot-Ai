use super::action::SkillAction;

pub struct SkillDefinition {
  pub id: &'static str,
  pub description: &'static str
}

pub fn registry() -> Vec<SkillDefinition> {
  vec![
    SkillDefinition {
      id: "photoshop.crop",
      description: "Crop the active document"
    },
    SkillDefinition {
      id: "photoshop.adjust.levels",
      description: "Adjust levels with input black/white and gamma"
    },
    SkillDefinition {
      id: "photoshop.export.png",
      description: "Export PNG using the save dialog"
    },
    SkillDefinition {
      id: "photoshop.export.jpeg",
      description: "Export JPEG with quality settings"
    },
    SkillDefinition {
      id: "photoshop.resize.canvas",
      description: "Resize the canvas to specific dimensions"
    },
    SkillDefinition {
      id: "photoshop.rotate",
      description: "Rotate the canvas by a degree angle"
    },
    SkillDefinition {
      id: "photoshop.straighten",
      description: "Straighten the canvas by a degree angle"
    },
    SkillDefinition {
      id: "desktop.open_app",
      description: "Launch a desktop app by file path"
    },
    SkillDefinition {
      id: "desktop.check_app",
      description: "Verify if an application exists on the system"
    },
    SkillDefinition {
      id: "desktop.click",
      description: "Click at screen coordinates with optional button and click count"
    },
    SkillDefinition {
      id: "desktop.move_mouse",
      description: "Move the mouse cursor to screen coordinates"
    },
    SkillDefinition {
      id: "desktop.type_text",
      description: "Type the provided text into the focused field"
    },
    SkillDefinition {
      id: "desktop.hotkey",
      description: "Press a hotkey combination like CTRL+S or ALT+F4"
    },
    SkillDefinition {
      id: "desktop.scroll",
      description: "Scroll the mouse wheel by a signed amount"
    },
    SkillDefinition {
      id: "desktop.wait",
      description: "Pause execution for a number of milliseconds"
    },
    SkillDefinition {
      id: "desktop.ui_click",
      description: "Click a UI element by name/control type"
    },
    SkillDefinition {
      id: "desktop.ui_type",
      description: "Type text into a UI element by name/control type"
    },
    SkillDefinition {
      id: "desktop.ui_read",
      description: "Read text/value from a UI element by name/control type"
    }
  ]
}


pub fn action_id(action: &SkillAction) -> &'static str {
  match action {
    SkillAction::PhotoshopCrop { .. } => "photoshop.crop",
    SkillAction::PhotoshopAdjustLevels { .. } => "photoshop.adjust.levels",
    SkillAction::PhotoshopExportPng { .. } => "photoshop.export.png",
    SkillAction::PhotoshopExportJpeg { .. } => "photoshop.export.jpeg",
    SkillAction::PhotoshopResizeCanvas { .. } => "photoshop.resize.canvas",
    SkillAction::PhotoshopRotate { .. } => "photoshop.rotate",
    SkillAction::PhotoshopStraighten { .. } => "photoshop.straighten",
    SkillAction::DesktopOpenApp { .. } => "desktop.open_app",
    SkillAction::DesktopCheckApp { .. } => "desktop.check_app",
    SkillAction::DesktopClick { .. } => "desktop.click",
    SkillAction::DesktopMoveMouse { .. } => "desktop.move_mouse",
    SkillAction::DesktopTypeText { .. } => "desktop.type_text",
    SkillAction::DesktopHotkey { .. } => "desktop.hotkey",
    SkillAction::DesktopScroll { .. } => "desktop.scroll",
    SkillAction::DesktopWait { .. } => "desktop.wait",
    SkillAction::DesktopUiClick { .. } => "desktop.ui_click",
    SkillAction::DesktopUiType { .. } => "desktop.ui_type",
    SkillAction::DesktopUiRead { .. } => "desktop.ui_read"
  }
}

