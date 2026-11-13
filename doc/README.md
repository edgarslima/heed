# Documentation Index

**Purpose:** Documentation index and repository entry point. **Audience:** Contributors and AI coding assistants. **Status:** Active. Last reviewed: 2026-11-13.

This repository keeps its operational knowledge under the `doc/` folder. The source of truth for repository guidance is the Open Knowledge Format (OKF) standard defined here.

## Purpose

The documentation architecture is intended to:
- keep product, engineering, environment, and process knowledge in one place;
- ensure all generated and maintained docs follow a consistent structure;
- give AI coding tools a single navigation point for project context;
- keep human-readable and machine-readable conventions easy to follow;
- standardize the repository on American English for all documentation, scripts, comments, and repository guidance.

## OKF structure

- `doc/README.md` — documentation index and repository entry point
- `doc/okf/README.md` — OKF rules and policy
- `doc/okf/structure.md` — folder taxonomy and metadata conventions
- `doc/okf/agent-directives.md` — rules for assistants and AI tools

## Current documentation inventory

- `doc/status.md` — execution checkpoint: current task, environment, open decisions, and next step
- `doc/environment/setup-environment.sh` — macOS environment bootstrap script for Tauri 2 + Rust + Swift + TypeScript
- `doc/plan/desktop-audio-recorder-construction-plan.md` — delivery plan for the desktop recorder project
- `doc/plan/desktop-audio-recorder-work-breakdown.md` — work packages, traceability, hosts, critical path, and pre-construction decisions
- `doc/plan/desktop-audio-recorder-p00-bootstrap.md` — step-level plan for task P00
- `doc/specs/software-architecture-desktop-audio-recorder.md` — software architecture specification
- `doc/specs/design-system-desktop-audio-recorder.md` — design system specification
- `doc/specs/build-specification-desktop-audio-recorder.md` — product and build specification
- `doc/standard/coding-practices-desktop-audio-recorder.md` — engineering standards and quality rules

## OKF rules

1. All project documentation must live under `doc/` unless a tool is hard-coded to look elsewhere.
2. Root-level assistant directive files exist only as lightweight pointers to the canonical documentation in `doc/`.
3. Every document should have a clear title, purpose, audience, and review date when relevant.
4. American English is the required language for all documentation, scripts, comments, repo-level guidance, and AI-generated repository text.
5. New documentation should be added with a consistent naming pattern, short readable names, and a clear relationship to the repo domain.
6. When a document is superseded, keep it only if it is needed for auditability or historical traceability.

## Required assistant policy

Each tool starts from its own entry-point file. Entry points only point to the canonical documents; they never add or override policy.

| Tool | Entry point |
| --- | --- |
| Codex and other repository agents | `AGENTS.md` |
| Claude Code | `CLAUDE.md`, which imports `AGENTS.md` and `doc/okf/agent-directives.md` |
| Cursor | `.cursor/rules/okf.mdc` and `.cursorrules` |

When guidance conflicts, the canonical document wins, in this order:

1. `doc/okf/agent-directives.md` for assistant behavior
2. `doc/okf/README.md` and `doc/okf/structure.md` for documentation rules
3. `doc/standard/coding-practices-desktop-audio-recorder.md` for code, scripts, and tests
4. `doc/specs/` and `doc/plan/` for product, architecture, and delivery decisions

If an entry-point file disagrees with a canonical document, correct the entry point.

## Contribution expectations

Before creating or editing repository documentation:
- check whether the information already exists in `doc/`;
- prefer updating the existing canonical document over duplicating content;
- keep the document structure consistent with OKF;
- use clear headings, stable naming, direct links to dependencies, and American English throughout.

## Review note

This folder is intentionally the single documentation surface for the repository. Tool-specific wrappers remain minimal and always redirect to the canonical OKF docs.
