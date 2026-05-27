# Quick Start: Automated Pilot AI

## 🚀 What You Get

An **automated system that:**
1. **Does the job** - Executes tasks automatically without asking
2. **Makes decisions** - Understands what you want and picks the right action
3. **Teaches you** - Shows each step so you learn how to do it yourself

## ⚡ Quick Setup

### Step 1: Restart the App
Close and reopen Pilot AI completely to load the new automated code.

### Step 2: Try It
Ask Pilot AI to do something:
```
"How to change wallpaper?"
"Take a screenshot"
"Open task manager"
"Restart my computer"
```

### Step 3: Watch It Work
The system will:
- ✅ Generate a plan instantly (no waiting for API)
- ✅ Execute automatically (no confirmation needed)
- ✅ Show each step (so you learn what's happening)

## 📋 Supported Tasks (Auto-Execution)

| Task | Example Query |
|------|---|
| Change Wallpaper | "change wallpaper", "set background", "desktop image" |
| Screenshot | "take a screenshot", "capture screen" |
| Task Manager | "open task manager", "close app", "kill process" |
| Command Prompt | "open cmd", "command prompt", "powershell", "terminal" |
| Restart/Shutdown | "restart", "shutdown", "turn off", "reboot" |
| File Manager | "open files", "file explorer", "file manager" |
| Settings | "open settings", "system settings" |

## 🎯 How It Works

```
You ask: "How to change wallpaper?"
         ↓
System checks knowledge base (instant, no API)
         ↓
Found! Generates plan automatically
         ↓
"Executing: Change desktop wallpaper"
  ✓ Right-click desktop to open context menu
  ✓ Wait for menu to appear
  ✓ Click Personalize
  ✓ Wait for Settings to open
  ✓ Navigate to Background
  ✓ Wait and select Picture option
  ✓ Click Browse to select an image file
         ↓
Done! Task completed, you learned how to do it ✓
```

## 🔧 What Changed

### Before
- Asked "how to change wallpaper"
- API request to Gemini
- Gemini error (503)
- Falls back to Google search
- ❌ Confused, nothing happens

### After
- Asked "how to change wallpaper"
- Local knowledge base lookup (<1ms)
- Auto-generates plan instantly
- Auto-executes without confirmation
- ✅ Done! Steps shown as they happen

## 🎓 Teaching Features

Every automated task shows:
1. **What it's doing** - "Right-click desktop to open context menu"
2. **Why it's doing it** - Wait delays, UI element names
3. **How to replicate** - Users see exact steps for manual repeat

## ⚙️ Technical Details

- **Knowledge Base**: 7 pre-configured tasks
- **Decision Making**: Keyword matching + intent detection
- **Execution**: Fully automated, no user intervention needed
- **Fallback**: Complex tasks still use Gemini API
- **Speed**: <1ms for knowledge base lookup vs 2-5s for API

## ❓ FAQ

**Q: Will this execute on my computer?**
A: Yes! The system uses Windows automation (UI clicks, keyboard, hotkeys) to execute tasks directly on your machine.

**Q: What if it fails?**
A: The system will show which step failed and why. You can then do it manually.

**Q: How is it teaching me?**
A: Each step is displayed with a clear explanation. By watching the automation, you learn the exact steps needed.

**Q: What about privacy?**
A: Everything runs locally. No personal data sent anywhere except for API calls to Gemini (which only happen for non-knowledge-base tasks).

**Q: Can I add my own tasks?**
A: Yes! Edit `src-tauri/src/automation/knowledge.rs` and add more task definitions.

## 🚨 Important

**RESTART THE APP** after installing this update. The old binary won't have the knowledge base.

## 📞 Support

If something doesn't work:
1. Check console for error messages
2. Verify you're using the latest binary (restart app)
3. Clear browser cache if UI seems broken
4. Check that Tauri backend is running

---

**You now have a fully automated system!** 🎉
