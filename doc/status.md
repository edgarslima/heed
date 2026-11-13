# Project Status — Desktop Audio Recorder

**Purpose:** Execution checkpoint with the current task, last verified state, and next step. **Audience:** Engineers and AI agents (Codex, Claude Code, Cursor) starting or resuming work. **Status:** Active. Last reviewed: 2026-11-13.

Related documents: [Construction plan](plan/desktop-audio-recorder-construction-plan.md) · [Coding practices](standard/coding-practices-desktop-audio-recorder.md) · [Agent directives](okf/agent-directives.md)

Each entry follows the checkpoint template in section 4 of the construction plan. Record facts and evidence only; do not paste chat history or user audio data.

## Next task

P00 is ready for review on `initial_dev`. After review and merge into `dev`, two lanes can start (work breakdown section 5): **P01 · macOS proof** (needs the owner present for permission prompts and audible playback) and **P03a · common engine** (pure logic, synthetic source). Each phase gets its step plan in `doc/plan/` before work starts.

## Repository

| Item | Value |
| --- | --- |
| Remote | `origin` → https://github.com/edgarslima/heed (public) |
| Default branch | `main` — stable; initial commit `208539a` |
| Integration branch | `dev` — created from `main` |
| Working branch | `initial_dev` — created from `dev` for P00 bootstrap work |
| CI host | GitHub Actions, `.github/workflows/ci.yml`; jobs on `macos-15`, `windows-2025`, `ubuntu-24.04` |

## Host environment

Observed on 2026-11-13 on the macOS bench.

| Component | Observed version |
| --- | --- |
| macOS | 27.0.1, arm64 |
| Xcode / Swift | Xcode 27.2 / Swift 6.4.2 |
| Rust | rustc 1.100.0, cargo 1.100.0, targets `aarch64-apple-darwin` and `x86_64-apple-darwin` |
| Tauri CLI | `@tauri-apps/cli` 2.12.4 pinned in `package.json` (a cargo `tauri-cli` 2.12.4 is also installed but not used) |
| Coverage | `cargo-llvm-cov` 0.9.1; `llvm-tools-preview` installed through `rust-toolchain.toml` |
| Node / npm | Node 24.21.0 / npm 11.19.0 |
| ffprobe | 8.1.2 |
| Windows 11 x64 bench | Not recorded |
| Ubuntu 24.04 LTS bench | Not recorded |

Environment gaps:

- `~/.cargo/bin` is not on the default shell `PATH`; run `source "$HOME/.cargo/env"` or add it to the shell profile.

## Open decisions

The full list, with what each blocks and its default, is in [work breakdown section 7](plan/desktop-audio-recorder-work-breakdown.md#7-decisions-before-construction).

1. **Windows 11 x64 and Ubuntu 24.04 benches.** Not available yet; P02, P03.7, P08, P09, P10, and P11 stay blocked until they exist.
2. **Signing credentials.** Needed at P09.2 only.

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
9. **Git remote and CI host.** Public GitHub repository `edgarslima/heed`; CI runs on GitHub Actions. Branches: `main` (stable), `dev` (integration), `initial_dev` (P00 work).

## Preflight — Repository alignment

- Status — validated.
- Branch and base commit — `main` at `208539a` (initial commit).
- Requirements covered — construction plan section 0 (repository-alignment preflight).
- Changes — created `doc/status.md`; added OKF metadata (purpose, audience, status, review date) to every document; fixed the broken build-specification link in the construction plan; removed duplicated gain sentences in architecture section 4; reduced `AGENTS.md`, `CLAUDE.md`, `.cursorrules`, and `.cursor/rules/okf.mdc` to pointers; `CLAUDE.md` imports `AGENTS.md` and the agent directives; extended the setup script with coverage tooling, `ffprobe`, and the keg-only Node `PATH` fix.
- Evidence — relative-link check over `doc/**/*.md` reports no broken links; `bash -n doc/environment/setup-environment.sh` passes.
- Pending — running the updated setup script (P00 Task 4 installs the missing coverage tooling).
- Next step — P00 on `initial_dev`.

## Planning — Work breakdown

- Status — validated by the owner (decisions recorded above).
- Branch and base commit — `main` at `208539a` (initial commit).
- Requirements covered — construction plan P00–P11; RF-01 to RF-15 and AC-01 to AC-10 traced in the work breakdown.
- Changes — added `doc/plan/desktop-audio-recorder-work-breakdown.md` and `doc/plan/desktop-audio-recorder-p00-bootstrap.md`; the construction plan section 3 now points to coding practices section 6 for the gate commands.
- Evidence — link and metadata checks pass; every RF and AC ID appears in the traceability table.
- Pending — Windows and Ubuntu benches.
- Next step — execute P00.

## P00 — Bootstrap

- Status — ready for review.
- Branch and base commit — `initial_dev`, base `e37ba38` (from `dev`).
- Requirements covered — construction plan P00; work breakdown P00.1–P00.7.
- Changes:
  - Toolchain pins (`rust-toolchain.toml` 1.100.0, `.nvmrc` 24, `.npmrc` exact versions), Cargo workspace with `crates/recorder-core` (no code yet) and `src-tauri`, workspace lints (`unsafe_op_in_unsafe_fn` denied; `unwrap`, `expect`, `panic` rejected by clippy outside tests).
  - Tauri 2 skeleton without demo code: product `Heed`, identifier `pro.edgarlima.heed`, window 560 × 600, minimum 420 × 480, CSP `default-src 'self'`, capability `core:default` only.
  - Frontend tooling: strict `tsconfig`, ESLint `strictTypeChecked` with `no-explicit-any` and `no-floating-promises`, Prettier, Vitest with jsdom and V8 coverage; `mountApp` app shell.
  - `src-tauri/src/lib.rs` exposes `configure` and `exit_code`, tested on the Tauri mock runtime; `main.rs` is a one-line window entry point.
  - Coverage gate `scripts/coverage-gate.mjs` with scopes in `scripts/coverage-scopes.json`; integer rule `covered * 100 > total * 80`. It also validates the scope schema, requires literal paths and reasons for justified exclusions, prints every exclusion with its measured counts, fails on coverage pragmas in production sources, and fails a pending scope whose report already measures code.
  - CI workflow per OS; `.gitattributes` forces LF; Windows test binaries embed the app manifest (`src-tauri/build.rs`).
- Evidence:
  - CI run [43501415439](https://github.com/edgarslima/heed/actions/runs/43501415439) on `1d3cba8`: success on macOS 15, Windows 2025, Ubuntu 24.04. Gate: frontend lines 6/6 and branches 2/2, rust-tauri lines 9/9 (`main.rs` excluded with 0/6 shown), rust-shared and swift-bridge pending. Gate tests: 21; total frontend and script tests: 24.
  - Local bench: the same command sequence passes; `tauri dev` window "Heed" 560 × 600, clamps to 420 × 480.
  - Packaged app: [evidence/p00/packaging.md](../evidence/p00/packaging.md).
- Pending:
  - **Owner review:** `src-tauri/src/main.rs` is a justified coverage exclusion (window entry point; composition tested through `configure`).
  - Packaged-window check on Windows and Linux (no benches yet).
  - Merge `initial_dev` into `dev` after review.
- Deviations from the P00 plan (each decided and verified during execution):
  - Task 2: dropped template dependencies not used in P00 (`@tauri-apps/api`, `@tauri-apps/plugin-opener`, `serde`, `serde_json`), demo assets, `README.md`, and `.vscode/`; moved `[profile.release]` to the workspace root; `withGlobalTauri` is `false`.
  - Task 3: `mountApp` was created in Task 3 after its failing test instead of in Task 2; Prettier ignores `doc/`.
  - Task 4: instead of `run() -> Result` in `lib.rs`, the app composition is `configure<R: Runtime>(Builder<R>) -> Builder<R>` and the exit mapping is `exit_code(Result<(), tauri::Error>) -> ExitCode`, both tested; `main.rs` only calls them. The mobile entry point attribute was removed (desktop only).
  - Task 5: test reports are built from literals in temporary directories instead of committed fixture files; the scope schema gained `exclude` (rule-based exclusions such as tests) and `justifiedExclusions` (reviewed exceptions with a reason). The gate was hardened after the final review (see Changes).
  - Task 6: the CI build order rationale was corrected (only release builds embed `frontendDist`); CI creates `coverage/` before `cargo llvm-cov`, pins `cargo-llvm-cov` 0.9.1, and Windows test binaries embed the app manifest. `.gitattributes` forces LF.
  - Task 7: the "networking disabled" check was replaced by `lsof` showing no sockets for the app process; the full offline check stays in P07.5 and P09.3.
- Owner actions:
  - Approve or reject the `src-tauri/src/main.rs` coverage exclusion; on approval, replace "Pending owner review" in `scripts/coverage-scopes.json` with the approver and date.
  - Enable branch protection on `main` and `dev` with the `verify` CI checks required, so coding practices section 6 rule 5 is enforced at merge.
- Next step — review P00, merge into `dev`, then write the P01 or P03a step plan.
