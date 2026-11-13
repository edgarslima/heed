# Project Status — Desktop Audio Recorder

**Purpose:** Execution checkpoint with the current task, last verified state, and next step. **Audience:** Engineers and AI agents (Codex, Claude Code, Cursor) starting or resuming work. **Status:** Active. Last reviewed: 2026-11-13.

Related documents: [Construction plan](plan/desktop-audio-recorder-construction-plan.md) · [Coding practices](standard/coding-practices-desktop-audio-recorder.md) · [Agent directives](okf/agent-directives.md)

Each entry follows the checkpoint template in section 4 of the construction plan. Record facts and evidence only; do not paste chat history or user audio data.

## Next task

P00 · Bootstrap, following the [P00 bootstrap plan](plan/desktop-audio-recorder-p00-bootstrap.md). Ready to execute.

## Host environment

Observed on 2026-11-13 on the macOS bench.

| Component | Observed version |
| --- | --- |
| macOS | 27.0.1, arm64 |
| Xcode / Swift | Xcode 27.2 / Swift 6.4.2 |
| Rust | rustc 1.100.0, cargo 1.100.0, targets `aarch64-apple-darwin` and `x86_64-apple-darwin` |
| Tauri CLI | tauri-cli 2.12.4 (cargo) |
| Node / npm | Node 24.21.0 / npm 11.19.0 |
| ffprobe | 8.1.2 |
| Windows 11 x64 bench | Not recorded |
| Ubuntu 24.04 LTS bench | Not recorded |

Environment gaps:

- `~/.cargo/bin` is not on the default shell `PATH`; run `source "$HOME/.cargo/env"` or add it to the shell profile.
- `llvm-tools-preview` and `cargo-llvm-cov` are not installed; the Rust coverage gate needs both. `doc/environment/setup-environment.sh` now installs them.
- The project directory is not a git repository, so base commits and branches cannot be recorded yet.

## Open decisions

The full list, with what each blocks and its default, is in [work breakdown section 7](plan/desktop-audio-recorder-work-breakdown.md#7-decisions-before-construction).

1. **Git remote and CI host.** Blocks only the remote CI run in P00.6. Default: GitHub Actions workflow file.
2. **Windows 11 x64 and Ubuntu 24.04 benches.** Not available yet; P02, P03.7, P08, P09, P10, and P11 stay blocked until they exist.
3. **Signing credentials.** Needed at P09.2 only.

## Resolved decisions

Resolved by the project owner on 2026-11-13.

1. **Interface language.** The interface ships in American English only, with strings in a single catalog keyed by stable English identifiers. Updated the coding practices.
2. **Private architecture reference.** Removed the private `chatgpt.com` link from the software architecture.
3. **Assistant file priority.** Canonical documents in `doc/` always win over tool entry points. `doc/README.md` now lists the entry points per tool and the conflict order; `doc/okf/structure.md` and `doc/okf/agent-directives.md` match it.
4. **File naming.** All file names, including scripts, use lowercase words separated by hyphens, except names fixed by a tool. Renamed the setup script to `doc/environment/setup-environment.sh`.
5. **Product name and identifier.** Product `Heed`, app identifier `pro.edgarlima.heed`, window title "Heed". Local desktop project; no DNS reservation. Updated the design system header.
6. **Bench strategy.** Mac only for now; follow the Mac-only path in work breakdown section 5.
7. **P03 split.** Approved: P03a and P03b build the engine on the Mac; P03.7 re-validates it against Windows observations before P08. Updated the construction plan dependency table.
8. **Speech corpus.** WER is measured on both PT-BR and American English corpora, scored separately. Updated the build specification (AC-08) and the plan's speech test.

## Preflight — Repository alignment

- Status — validated.
- Branch and base commit — none; the directory is not a git repository.
- Requirements covered — construction plan section 0 (repository-alignment preflight).
- Changes — created `doc/status.md`; added OKF metadata (purpose, audience, status, review date) to every document; fixed the broken build-specification link in the construction plan; removed duplicated gain sentences in architecture section 4; reduced `AGENTS.md`, `CLAUDE.md`, `.cursorrules`, and `.cursor/rules/okf.mdc` to pointers; `CLAUDE.md` imports `AGENTS.md` and the agent directives; extended the setup script with coverage tooling, `ffprobe`, and the keg-only Node `PATH` fix.
- Evidence — relative-link check over `doc/**/*.md` reports no broken links; `bash -n doc/environment/setup-environment.sh` passes.
- Pending — `git init`; running the updated setup script.
- Next step — start P00 after initializing git.

## Planning — Work breakdown

- Status — ready for review.
- Branch and base commit — none; the directory is not a git repository.
- Requirements covered — construction plan P00–P11; RF-01 to RF-15 and AC-01 to AC-10 traced in the work breakdown.
- Changes — added `doc/plan/desktop-audio-recorder-work-breakdown.md` and `doc/plan/desktop-audio-recorder-p00-bootstrap.md`; the construction plan section 3 now points to coding practices section 6 for the gate commands.
- Evidence — link and metadata checks pass; every RF and AC ID appears in the traceability table.
- Pending — git remote and CI host; Windows and Ubuntu benches.
- Next step — execute P00 Task 1.

## P00 — Bootstrap

- Status — to do.
- Branch and base commit — not created yet.
- Requirements covered — construction plan P00.
- Changes — none.
- Evidence — none.
- Pending — Tauri skeleton, `recorder-core` crate, CI lint/build, and a basic packaged window.
- Next step — execute Task 1 of the P00 bootstrap plan.
