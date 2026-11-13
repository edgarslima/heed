# Desktop Audio Recorder P00 Bootstrap Plan

**Purpose:** Step-level implementation plan for task P00 (bootstrap). **Audience:** The engineer or AI agent executing P00. **Status:** Executed on 2026-11-13; checkpoint and evidence in `doc/status.md`. Last reviewed: 2026-11-13.

Related documents: [Work breakdown](desktop-audio-recorder-work-breakdown.md) · [Construction plan](desktop-audio-recorder-construction-plan.md) · [Software architecture](../specs/software-architecture-desktop-audio-recorder.md) · [Coding practices](../standard/coding-practices-desktop-audio-recorder.md) · [Status](../status.md)

**Goal:** Produce a pinned Tauri 2 + TypeScript + Rust workspace with a `recorder-core` crate, strict lint and test tooling, a coverage gate, a CI workflow, and a basic packaged window.

**Architecture:** Cargo workspace with `src-tauri` (Tauri app) and `crates/recorder-core` (engine, no Tauri dependency). Plain TypeScript frontend in `src/` built by Vite. No product features in P00.

**Tech stack:** Rust 1.100.0, Tauri 2 (npm `@tauri-apps/cli` and `@tauri-apps/api`), Node 24 / npm, Vite, TypeScript, ESLint with typescript-eslint, Prettier, Vitest with V8 coverage and jsdom, `cargo-llvm-cov`.

**Spec:** Construction plan task P00, architecture section 1, coding practices sections 3 and 6.

## Global constraints

- American English for every identifier, comment, test name, script, config key, and commit message.
- File names: lowercase words separated by hyphens, except names fixed by a tool.
- Pin versions: `rust-toolchain.toml` channel `1.100.0`; `.nvmrc` `24`; `.npmrc` `save-exact=true`; commit `Cargo.lock` and `package-lock.json`.
- Product name `Heed`, app identifier `pro.edgarlima.heed` (local desktop project; no DNS reservation). Window: title "Heed", 560 × 600 initial, 420 × 480 minimum (design system section 1).
- Bundle uses local assets only; capabilities grant only what P00 uses.
- Coverage gate approves only when `covered * 100 > total * 80` on integer counts.
- `recorder-core` must not depend on `tauri`.
- Before running a command, run `source "$HOME/.cargo/env"` (cargo is not on the default `PATH` on the bench).

## Review focus

1. **Coverage off-by-one.** 80 of 100 lines must fail and 81 must pass. Pinned by Task 5 tests.
2. **Silent gate pass.** A missing, empty, or invalid report must fail, never skip. Pinned by Task 5 tests.
3. **Scaffold leftovers.** The template's demo `greet` command and assets must be gone. Pinned by Task 2 step 4.
4. **Floating versions.** No `^` or `~` ranges in `package.json`. Pinned by Task 3 step 5.
5. **Template `expect` in `main.rs`.** It violates the no-`expect` rule on execution paths. Pinned by Task 4 step 3.

---

### Task 1: Repository foundation (P00.1)

**Files:**
- Create: `.gitignore`, `rust-toolchain.toml`, `.nvmrc`, `.npmrc`, `Cargo.toml`, `crates/recorder-core/Cargo.toml`, `crates/recorder-core/src/lib.rs`

**Interfaces:**
- Produces: workspace members `src-tauri` and `crates/recorder-core`; `[workspace.lints]` that members inherit with `[lints] workspace = true`.

- [x] **Step 1:** Run `git init -b main` in the repository root. Done in the initial commit `208539a`; remote `origin` is https://github.com/edgarslima/heed. P00 work happens on branch `initial_dev`.
- [x] **Step 2:** Write `.gitignore` with `target/`, `node_modules/`, `dist/`, `coverage/`, `evidence/**/*.wav`, `.DS_Store`. Done in the initial commit, which also ignores `.env*`, logs, and local assistant settings.
- [x] **Step 3:** Write `rust-toolchain.toml` with channel `1.100.0`, components `rustfmt`, `clippy`, `llvm-tools-preview`, and targets `aarch64-apple-darwin`, `x86_64-apple-darwin`. Write `.nvmrc` (`24`) and `.npmrc` (`save-exact=true`).
- [x] **Step 4:** Write the workspace `Cargo.toml`: `resolver = "3"`, members `["crates/recorder-core"]` (Task 2 adds `src-tauri`), `[workspace.package] edition = "2024"`, and lints:
  - `[workspace.lints.rust]` `unsafe_op_in_unsafe_fn = "deny"`
  - `[workspace.lints.clippy]` `unwrap_used = "warn"`, `expect_used = "warn"`, `panic = "warn"` (CI runs clippy with `-D warnings`)
- [x] **Step 5:** Create `recorder-core` as a library crate with a crate-level Rustdoc comment stating that it has no Tauri dependency. No other code yet; its coverage scope is pending.
- [x] **Step 6:** Run `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings`. Expected: both pass.
- [x] **Step 7:** Commit `chore: add workspace foundation and toolchain pins` on `initial_dev`.

### Task 2: Tauri skeleton (P00.2)

**Files:**
- Create: `package.json`, `index.html`, `vite.config.ts`, `tsconfig.json`, `src/main.ts`, `src/app.ts`, `src-tauri/**`
- Modify: `Cargo.toml` (members)

**Interfaces:**
- Produces: `mountApp(root: HTMLElement): void` in `src/app.ts`, which renders an empty `<main class="app">`. `src/main.ts` calls it on `#app`.

- [x] **Step 1:** Run `npm create tauri-app@latest -- --help` and record the non-interactive flags for project name, identifier, template `vanilla-ts`, and package manager `npm`. Do not write the flags from memory.
- [x] **Step 2:** Scaffold into a temporary directory with product name `Heed` and identifier `pro.edgarlima.heed`, then move the files into the repository root. Keep `src-tauri/` as a workspace member and remove its own `[workspace]` section if present.
- [x] **Step 3:** Set in `src-tauri/tauri.conf.json`: `productName` `Heed`, `identifier` `pro.edgarlima.heed`, window `title` "Heed", `width` 560, `height` 600, `minWidth` 420, `minHeight` 480, and a CSP limited to `'self'`.
- [x] **Step 4:** Remove the template's demo code: the `greet` command, demo HTML, and demo assets. Reduce `src-tauri/capabilities/default.json` to `core:default`.
- [x] **Step 5:** Run `npm install`, then `npm run tauri dev`. Expected: one empty window titled "Heed" at 560 × 600 that cannot be resized below 420 × 480.
- [x] **Step 6:** Commit `feat: add Tauri 2 skeleton without demo code`.

### Task 3: TypeScript quality tooling (P00.3)

**Files:**
- Create: `eslint.config.js`, `.prettierrc.json`, `vitest.config.ts`, `src/app.test.ts`
- Modify: `package.json`, `tsconfig.json`

**Interfaces:**
- Produces npm scripts: `lint` (ESLint + `prettier --check`), `format`, `typecheck` (`tsc --noEmit`), `test` (`vitest run`), `test:coverage` (`vitest run --coverage`), `build` (`vite build`).

- [x] **Step 1:** Write the failing test `src/app.test.ts`: `mountApp renders the app shell`. It asserts that after `mountApp(root)`, `root.querySelector("main.app")` is not null. Add a test that importing `src/main.ts` with a `#app` element present mounts the shell.
- [x] **Step 2:** Run `npm test`. Expected: FAIL until Vitest and jsdom are configured.
- [x] **Step 3:** Configure `tsconfig.json` with `strict: true`, `noUncheckedIndexedAccess: true`, and `noImplicitOverride: true`. Configure ESLint with typescript-eslint `strictTypeChecked`, plus `@typescript-eslint/no-explicit-any: "error"` and `@typescript-eslint/no-floating-promises: "error"`.
- [x] **Step 4:** Configure `vitest.config.ts`: environment `jsdom`, coverage provider `v8`, `include: ["src/**/*.ts"]`, exclude `**/*.test.ts`, reporters `json-summary` and `text`, `reportsDirectory: "coverage/frontend"`.
- [x] **Step 5:** Pin every devDependency to an exact version. Verify with `grep -E '"[~^]' package.json`. Expected: no output.
- [x] **Step 6:** Run `npm run lint`, `npm run typecheck`, `npm run test:coverage`, and `npm run build`. Expected: all pass; `coverage/frontend/coverage-summary.json` exists.
- [x] **Step 7:** Commit `chore: add strict TypeScript lint, format, and test tooling`.

### Task 4: Rust quality tooling (P00.4)

**Files:**
- Create: `clippy.toml`
- Modify: `src-tauri/Cargo.toml`, `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`

- [x] **Step 1:** Add `[lints] workspace = true` to `src-tauri/Cargo.toml`.
- [x] **Step 2:** Write `clippy.toml` with `allow-unwrap-in-tests = true`, `allow-expect-in-tests = true`, `allow-panic-in-tests = true`.
- [x] **Step 3:** Replace the template's `.expect(...)` on the Tauri builder. `run() -> Result<(), tauri::Error>` in `src-tauri/src/lib.rs` returns the builder result. `main` prints the error code to stderr and exits with a non-zero status.
- [x] **Step 4:** Run `cargo clippy --locked --workspace --all-targets -- -D warnings`. Expected: pass.
- [x] **Step 5:** Install the coverage tool with `cargo install cargo-llvm-cov --locked`. Run `mkdir -p coverage && cargo llvm-cov --locked --workspace --json --output-path coverage/rust.json`. Expected: the file exists.
- [x] **Step 6:** Commit `chore: enforce Rust lint policy and coverage collection`.

### Task 5: Coverage gate (P00.5)

**Files:**
- Create: `scripts/coverage-gate.mjs`, `scripts/coverage-scopes.json`, `scripts/coverage-gate.test.mjs`, `scripts/fixtures/coverage/*.json`
- Modify: `package.json` (script `coverage:gate`), `vitest.config.ts` (include `scripts/**/*.test.mjs` in test discovery only)

**Interfaces:**
- `passesThreshold(covered: number, total: number): boolean` returns `covered * 100 > total * 80`.
- `evaluateScope(scope, report, sourceFiles): { name, status: "pass" | "fail" | "pending", metrics: Array<{ metric, covered, total }>, reasons: string[] }`.
- Scope fields in `coverage-scopes.json`: `name`, `kind` (`"llvm-cov" | "vitest" | "xccov"`), `report` (path), `include` (globs), `metrics` (`["lines"]` or `["lines", "branches"]`), `status` (`"active" | "pending"`).
- Initial scopes: `frontend` (active, lines and branches, `src/**/*.ts`); `rust-shared` (pending until `recorder-core` has executable code); `rust-tauri` (active, `src-tauri/src/**`); `swift-bridge` (pending until P01.1).
- Report shapes: `cargo llvm-cov` JSON `data[0].files[].filename` and `.summary.lines.{count, covered}`; Vitest `coverage-summary.json` per-file `{ lines, branches }.{ total, covered }`.

- [x] **Step 1:** Write the failing tests in `scripts/coverage-gate.test.mjs`:
  - `passes at 81 of 100 lines`: `passesThreshold(81, 100) === true`
  - `fails at exactly 80 of 100 lines`: `passesThreshold(80, 100) === false`
  - `passes at 801 of 1000 lines`: `passesThreshold(801, 1000) === true`
  - `fails when the report file is missing`: status `fail`, reason mentions the report path
  - `fails when the report is empty or invalid JSON`
  - `fails when an active scope has zero measured items`
  - `fails when a production file is missing from the report`: a source file matching `include` that is absent from the report
  - `reports a pending scope as pending, not pass`
  - `fails the frontend scope when branches are at exactly 80%`
- [x] **Step 2:** Run `npx vitest run scripts`. Expected: FAIL (module not found).
- [x] **Step 3:** Implement `scripts/coverage-gate.mjs` with Node built-ins only. The CLI evaluates every scope, prints one line per scope with integer counts, and exits 1 if any active scope fails. Production source inventory: list files with `git ls-files --cached --others --exclude-standard` (so untracked production files count) and filter by each scope's `include`.
- [x] **Step 4:** Run `npx vitest run scripts`. Expected: PASS.
- [x] **Step 5:** Run `npm run test:coverage`, the Task 4 `cargo llvm-cov` command, then `npm run coverage:gate`. Expected: `frontend` pass, `rust-tauri` pass or fail with real counts, others pending. If `rust-tauri` fails, add tests for the failing code; if a line cannot be reached without a window, report it to the owner as a P00 pending item. Never exclude files silently.
- [x] **Step 6:** Commit `feat: add coverage gate with exact integer threshold`.

### Task 6: CI workflow (P00.6)

**Files:**
- Create: `.github/workflows/ci.yml`

- [x] **Step 1:** Define one job per OS: `macos-15`, `windows-2025`, `ubuntu-24.04`. Install the Linux WebKitGTK prerequisites listed in the Tauri prerequisites page. Use Node from `.nvmrc` and Rust from `rust-toolchain.toml`.
- [x] **Step 2:** Steps per job: `npm ci`, `npm run build`, install `cargo-llvm-cov`, `cargo fmt --all -- --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings`, `cargo llvm-cov --locked --workspace --json --output-path coverage/rust.json`, `npm run lint`, `npm run typecheck`, `npm run test:coverage`, `npm run coverage:gate`. Upload `coverage/` as an artifact. The frontend build runs first because release builds embed `frontendDist` (`dist/`, git-ignored) at compile time; debug builds use `devUrl` and do not need it (verified on 2026-11-13). Confirm this against the Tauri 2 configuration docs while writing the job, and keep a comment in the workflow explaining the order.
- [x] **Step 3:** Run the same commands locally on M. Expected: all pass. Push `initial_dev` and confirm the GitHub Actions run with `gh run watch`; record the run URL and result in `doc/status.md`.
- [x] **Step 4:** Commit `ci: add lint, test, coverage, and build jobs per OS`.

### Task 7: Packaged window and checkpoint (P00.7)

**Files:**
- Create: `evidence/p00/packaging.md`
- Modify: `doc/status.md`

- [x] **Step 1:** Run `npm run tauri build -- --bundles app`. Expected: a `.app` under `target/release/bundle/macos/`.
- [x] **Step 2:** Open the `.app` with networking disabled. Expected: the empty "Heed" window opens.
- [x] **Step 3:** Record in `evidence/p00/packaging.md`: commit, OS, toolchain versions, command, the `.app` SHA-256 (`shasum -a 256` over a zip of the bundle), and the result.
- [x] **Step 4:** Update the `doc/status.md` P00 entry with the plan section 4 template: status, base commit, changes, evidence, pending items (remote CI, W and L runs), and next step.
- [x] **Step 5:** Commit `docs: record P00 checkpoint and packaging evidence`.
