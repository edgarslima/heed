# OKF Structure and Taxonomy

**Purpose:** Defines the folder taxonomy, naming, and metadata conventions. **Audience:** Contributors and AI coding assistants. **Status:** Active. Last reviewed: 2026-11-13.

This document defines the project taxonomy and the folder conventions that keep the documentation set aligned with Open Knowledge Format (OKF).

## Folder taxonomy

- `doc/` — repository documentation root
- `doc/README.md` — master index
- `doc/status.md` — execution checkpoint for the active task
- `doc/okf/` — canonical OKF rules and AI assistance policy
- `doc/specs/` — product, behavior, and architecture specs
- `doc/plan/` — roadmap, delivery plan, and execution sequencing
- `doc/standard/` — engineering standards and quality gates
- `doc/environment/` — setup and environment automation

## File naming rules

Use a consistent naming pattern:
- lowercase words separated by hyphens for every document and script;
- names fixed by a tool or convention keep their required form, such as `README.md`, `AGENTS.md`, and `CLAUDE.md`;
- descriptive names that reflect the subject, not the author or temporary status;
- keep file names stable across references and links;
- use American English for all repo-facing names and descriptions.

Examples:
- `setup-environment.sh`
- `desktop-audio-recorder-construction-plan.md`
- `software-architecture-desktop-audio-recorder.md`

## Required document metadata

At minimum, each project document should clearly contain:
- title
- purpose or scope
- audience
- related documents
- review date or status when useful

## Priority rules

When multiple documents cover the same subject:
1. The canonical file wins. Tool entry points (`AGENTS.md`, `CLAUDE.md`, Cursor rules) never take precedence over `doc/`; the conflict order is defined in `doc/README.md`.
2. Supporting files must link to the canonical file, not duplicate the source text.
3. Old or superseded docs remain only when historical traceability matters.

## Content quality rules

- Prefer explicit statements over implied assumptions.
- State requirements and conditions directly.
- Keep diagrams, tables, and checklists readable and not inflated with noise.
- Maintain references to the exact repo documents they depend on.
- Keep language clear enough for both humans and AI agents.
- Write all documentation in American English.

## Change management

When a file is edited:
- review related documents for dependency drift;
- update cross-references if the file name or purpose changed;
- keep the directory taxonomy stable unless an explicit repo-wide reorganization is requested.

## Canonical documents

The following are the authoritative sources for the repo documentation model:

- `doc/README.md`
- `doc/okf/README.md`
- `doc/okf/agent-directives.md`

No tool-specific instruction file should override the canonical structure without being explicitly aligned with the OKF rules.
