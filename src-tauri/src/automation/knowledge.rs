// Local knowledge base for common Windows tasks
// Provides fallback answers when API is unavailable

use super::action::SkillAction;

#[derive(Debug, Clone)]
pub struct KnowledgeEntry {
    pub goal: String,
    pub note: Option<String>,
    pub steps: Vec<(String, SkillAction)>,
}

pub fn lookup_task(query: &str) -> Option<KnowledgeEntry> {
    // Strip any chat-history prefix like "User: " or multi-line history
    // so the actual intent is what we match against.
    let raw = query
        .lines()
        .last()
        .unwrap_or(query)
        .trim_start_matches("User:")
        .trim()
        .to_lowercase();
    // Also work with the full lowercased query as fallback
    let query_lower = query.to_lowercase();
    let q = &raw; // primary match target

    // ── Desktop wallpaper ──
    let is_wallpaper = q.contains("wallpaper")
        || q.contains("wall paper")
        || q.contains("background")
        || (q.contains("desktop") && q.contains("image"))
        // fallback: check original full string too
        || query_lower.contains("wallpaper")
        || query_lower.contains("wall paper");

    if is_wallpaper {

        return Some(KnowledgeEntry {
            goal: "Change desktop wallpaper".to_string(),
            note: Some(
                "Opening Background settings directly. \
                 Manual method: Right-click desktop → Personalize → Background → Browse photos."
                    .to_string(),
            ),
            steps: vec![
                (
                    "Move mouse to the desktop to show the starting point".to_string(),
                    SkillAction::DesktopMoveMouse { x: 700, y: 400 },
                ),
                (
                    "Wait a moment".to_string(),
                    SkillAction::DesktopWait { ms: 600 },
                ),
                (
                    "Open Background settings directly via Windows Settings URI".to_string(),
                    SkillAction::DesktopOpenApp {
                        path: "ms-settings:personalization-background".to_string(),
                        args: None,
                    },
                ),
                (
                    "Wait for Background settings page to open".to_string(),
                    SkillAction::DesktopWait { ms: 2000 },
                ),
                (
                    "Click Browse photos to pick a new wallpaper image".to_string(),
                    SkillAction::DesktopUiClick {
                        name: "Browse photos".to_string(),
                        control_type: Some("Button".to_string()),
                        window_name: Some("Settings".to_string()),
                    },
                ),
            ],
        });
    }

    // ── Screenshot ──
    if q.contains("screenshot") || q.contains("screen shot")
        || (q.contains("capture") && q.contains("screen"))
    {
        return Some(KnowledgeEntry {
            goal: "Take a screenshot".to_string(),
            note: Some("Taking a screenshot using Win+Shift+S (Snipping Tool). The selection will be saved to your clipboard.".to_string()),
            steps: vec![
                (
                    "Press Win+Shift+S to open the Snipping Tool overlay".to_string(),
                    SkillAction::DesktopHotkey {
                        keys: vec!["win".to_string(), "shift".to_string(), "s".to_string()],
                    },
                ),
                (
                    "Wait for the snipping overlay to appear".to_string(),
                    SkillAction::DesktopWait { ms: 800 },
                ),
            ],
        });
    }

    // ── Task Manager ──
    if q.contains("task manager") || q.contains("close app")
        || q.contains("kill process") || q.contains("open task")
    {
        return Some(KnowledgeEntry {
            goal: "Open Task Manager".to_string(),
            note: Some("Opening Task Manager. You can now close unresponsive apps or monitor system performance.".to_string()),
            steps: vec![
                (
                    "Press Ctrl+Shift+Esc to open Task Manager directly".to_string(),
                    SkillAction::DesktopHotkey {
                        keys: vec!["ctrl".to_string(), "shift".to_string(), "esc".to_string()],
                    },
                ),
                (
                    "Wait for Task Manager to open".to_string(),
                    SkillAction::DesktopWait { ms: 1000 },
                ),
            ],
        });
    }

    // ── Command Prompt / PowerShell ──
    if q.contains("command prompt") || q.contains("cmd")
        || q.contains("powershell") || q.contains("terminal")
    {
        return Some(KnowledgeEntry {
            goal: "Open Command Prompt or PowerShell".to_string(),
            note: Some("Opening Command Prompt via the Run dialog.".to_string()),
            steps: vec![
                (
                    "Press Win+R to open the Run dialog".to_string(),
                    SkillAction::DesktopHotkey {
                        keys: vec!["win".to_string(), "r".to_string()],
                    },
                ),
                (
                    "Wait for Run dialog to appear".to_string(),
                    SkillAction::DesktopWait { ms: 400 },
                ),
                (
                    "Type 'cmd' to open Command Prompt".to_string(),
                    SkillAction::DesktopTypeText {
                        text: "cmd".to_string(),
                    },
                ),
                (
                    "Press Enter to launch Command Prompt".to_string(),
                    SkillAction::DesktopHotkey {
                        keys: vec!["Return".to_string()],
                    },
                ),
            ],
        });
    }

    // ── Shutdown / Restart ──
    if q.contains("shutdown") || q.contains("shut down")
        || q.contains("restart") || q.contains("turn off") || q.contains("reboot")
    {
        return Some(KnowledgeEntry {
            goal: "Shutdown or Restart Computer".to_string(),
            note: Some("Opening the Power menu. Select your preferred action.".to_string()),
            steps: vec![
                (
                    "Press Win+X to open the Power user menu".to_string(),
                    SkillAction::DesktopHotkey {
                        keys: vec!["win".to_string(), "x".to_string()],
                    },
                ),
                (
                    "Wait for menu to appear".to_string(),
                    SkillAction::DesktopWait { ms: 400 },
                ),
                (
                    "Click Shut down or sign out".to_string(),
                    SkillAction::DesktopUiClick {
                        name: "Shut down or sign out".to_string(),
                        control_type: Some("MenuItem".to_string()),
                        window_name: None,
                    },
                ),
            ],
        });
    }

    // ── File Explorer ──
    if q.contains("file manager") || q.contains("file explorer")
        || q.contains("open files") || q.contains("explorer")
    {
        return Some(KnowledgeEntry {
            goal: "Open File Explorer".to_string(),
            note: Some("Opening File Explorer so you can browse and manage files.".to_string()),
            steps: vec![
                (
                    "Press Win+E to open File Explorer".to_string(),
                    SkillAction::DesktopHotkey {
                        keys: vec!["win".to_string(), "e".to_string()],
                    },
                ),
                (
                    "Wait for File Explorer to open".to_string(),
                    SkillAction::DesktopWait { ms: 800 },
                ),
            ],
        });
    }

    // ── Settings ──
    if q.contains("settings") && (q.contains("open") || q.contains("system")) {
        return Some(KnowledgeEntry {
            goal: "Open Settings".to_string(),
            note: Some("Opening Windows Settings.".to_string()),
            steps: vec![
                (
                    "Press Win+I to open Settings".to_string(),
                    SkillAction::DesktopHotkey {
                        keys: vec!["win".to_string(), "i".to_string()],
                    },
                ),
                (
                    "Wait for Settings to open".to_string(),
                    SkillAction::DesktopWait { ms: 1000 },
                ),
            ],
        });
    }

    None

}

pub fn convert_to_planned_actions(entry: &KnowledgeEntry) -> crate::automation::ai::PlannedActions {
    use crate::automation::ai::PlannedStep;

    let steps = entry
        .steps
        .iter()
        .enumerate()
        .map(|(i, (explanation, action))| PlannedStep {
            id: Some(format!("step_{}", i + 1)),
            action: action.clone(),
            expected_result: None,
            explanation: Some(explanation.clone()),
            retryable: false,
            requires_confirm: false,
        })
        .collect();

    crate::automation::ai::PlannedActions {
        goal: Some(entry.goal.clone()),
        note: entry.note.clone(),
        steps,
    }
}
