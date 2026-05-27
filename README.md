# Pilot AI

Pilot AI is a Windows desktop assistant that teaches and executes a small set of Photoshop skills using a hybrid automation architecture: deterministic skills first, vision fallback second.

## MVP goals
- Reliability-first execution using deterministic skills
- Ghost mode previews before actions
- Action sandbox limited to Photoshop
- Step-by-step teaching with highlights
- Fast, efficient screen sensing

## Development
- npm install
- npm run dev
- npm run build

## Environment
- Copy .env.example to .env
- Set GEMINI_API_KEYS to one or more comma-separated keys
- Optional: set GEMINI_MODEL (default: gemini-1.5-flash)

## Notes
- Photoshop skills are stubbed in the Rust automation modules.
- The UI uses a minimal local planner when the backend is not running.
