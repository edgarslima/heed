# OKF AI Assistant Directives

**Purpose:** Canonical policy for AI assistant behavior in this repository. **Audience:** AI coding assistants and the reviewers who supervise them. **Status:** Active. Last reviewed: 2026-11-13.

This document defines how AI coding assistants must behave when working inside this repository.

## Mandatory repository behavior

1. Treat `doc/` as the canonical documentation store.
2. Keep all product and engineering knowledge discoverable through `doc/README.md`.
3. Prefer updates to canonical docs over ephemeral or duplicated instructions.
4. Do not create documentation outside `doc/` when the information is project knowledge and belongs to the repository.
5. Use tool-specific wrapper files only as lightweight references to the canonical documentation.
6. Write every repository-facing instruction, note, comment, summary, and response in American English.

## AI tool rules

### Codex / AGENTS.md

`AGENTS.md` is a lightweight repository entry point. It must:
- be short and concise;
- describe where the canonical docs live;
- point to `doc/README.md`, `doc/okf/README.md`, and this file;
- avoid duplicating detailed engineering policy;
- write all project-facing guidance in American English only.

### Claude Code / CLAUDE.md

`CLAUDE.md` must:
- be a concise pointer to the project documentation under `doc/`;
- import `AGENTS.md` and this file with `@` imports so the policy is loaded into Claude Code context without restating it;
- mention that the OKF policy is the source of truth;
- refer to the same canonical files as the repo index;
- keep all written guidance in American English.

### Cursor

Cursor rule files should:
- be minimal and project-scoped;
- confirm that documentation is centered on `doc/`;
- reference the canonical OKF docs rather than restating them in full;
- use American English in all instructions and comments.

## Required response pattern

When an assistant is asked to work on repo knowledge or docs:
- identify the relevant canonical document before editing;
- keep the repo structure aligned with OKF;
- update related references when necessary;
- avoid documentation drift between `doc/` and tool-specific files;
- ensure all written output is in American English.

## Memory AI usage

Use AI Memory as an operational hook and recovery layer for every session:
- consult AI Memory at the start of a task to recover the latest project state, recent decisions, open loops, and pending handoffs;
- use it as a lightweight working memory for high-value context that should survive session churn, not as a dump of raw logs or full transcripts;
- save concise checkpoints containing the current objective, constraints, last successful validation, open blockers, and next action;
- use memory to reduce context-window pressure by keeping only distilled state in the active prompt while preserving detailed artifacts in the repo or memory store;
- treat memory as the primary fallback when a tool, shell, or agent fails mid-task: restore the last known checkpoint, confirm the last verified state, and resume without redoing unrelated work;
- when a task is interrupted or a tool returns an error, use the memory record to resume from the exact point of failure rather than restarting from scratch;
- keep memory entries compact, structured, and reviewable so the agent can reuse them across sessions and tools without burning tokens.

Recoverable execution state lives in the repository in `doc/status.md`, using the checkpoint template from the construction plan. AI Memory complements that file; it does not replace it.

Memory must support continuity, reduce unnecessary token usage, and improve recovery from stalled or failed work. It should not replace the repo's canonical documentation; it should complement it.

## Coding standards obligation

All code changes, scripts, comments, and tests must follow the repository standards defined in `doc/standard/coding-practices-desktop-audio-recorder.md`.

Required compliance:
- use American English for all code, comments, tests, and diagnostics;
- follow the naming conventions for Rust, TypeScript, and Swift;
- preserve architecture boundaries between frontend, Rust engine, native adapters, storage, and IPC contracts;
- use strict validation, linting, formatting, and test coverage for the affected scope;
- keep any `unsafe` native code minimal, justified, and documented with explicit `SAFETY:` notes when required;
- protect recorder integrity, error handling, recovery, and file safety for the desktop audio workflow;
- do not merge low-coverage or unverified code paths.

## Quality bar

The assistant must not:
- hide important project information in ad hoc notes;
- create competing documentation systems outside `doc/`;
- duplicate the same knowledge in multiple conflicting forms;
- ignore the repo index when choosing relevant context;
- write repository-facing content in any language other than American English;
- ignore the coding standards in `doc/standard/coding-practices-desktop-audio-recorder.md` when making or reviewing code changes.

## Canonical references

- `doc/README.md`
- `doc/okf/README.md`
- `doc/okf/structure.md`

This file is the policy entry point for AI tooling behavior. All tool-specific files should mirror this intent without becoming parallel knowledge stores.
