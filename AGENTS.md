# Repository agent instructions

Entry point for Codex and other repository agents. The canonical policy lives in `doc/`; this file only points to it.

## Read first

- `doc/README.md` — documentation index and conflict order
- `doc/okf/README.md` — OKF documentation standard
- `doc/okf/agent-directives.md` — canonical AI assistant policy (OKF, language, memory, coding obligations)
- `doc/status.md` — execution checkpoint: current task, environment, open decisions, next step
- `doc/standard/coding-practices-desktop-audio-recorder.md` — mandatory for any code, script, or test change
- `doc/plan/desktop-audio-recorder-construction-plan.md` — task sequence P00–P11, verification commands (section 3), and task prompts (section 4)

## Scope rules

- `doc/` is the single source of truth. Update the canonical document; keep this file a pointer.
- Write all repository-facing text, code, comments, tests, and diagnostics in American English.
- At task start, recover state from AI Memory and `doc/status.md`. Record checkpoints in `doc/status.md`.
- Before finishing, run the smallest relevant validation and record command, environment, and result.
