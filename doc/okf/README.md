# Open Knowledge Format (OKF)

**Purpose:** Defines the Open Knowledge Format (OKF) documentation standard. **Audience:** Contributors and AI coding assistants. **Status:** Active. Last reviewed: 2026-11-13.

Open Knowledge Format (OKF) is the repository documentation standard used to keep technical and product knowledge organized, discoverable, and reviewable.

## Goals

- Keep all canonical documentation in `doc/`
- Standardize document naming, structure, and ownership
- Create navigation points for humans and AI tools
- Preserve reviewability and traceability over time
- Reduce duplication and drift across engineering documents
- Require American English for repository-level documentation, guides, comments, and AI instructions

## Scope

OKF applies to:
- architecture and design documents
- requirements and specs
- implementation plans
- engineering standards
- environment setup and operational runbooks
- AI assistant guidance files that reference repository policy

## Required structure

Every document in `doc/` should follow this baseline:

1. Title
2. Purpose or scope
3. Audience
4. Related documents and dependencies
5. Core content
6. Status or review date
7. Follow-up actions if applicable

## Naming rules

- Use lowercase words separated by hyphens for all file names, including scripts
- Exception: names fixed by a tool or convention keep their required form, such as `README.md`, `AGENTS.md`, `CLAUDE.md`, and `.cursorrules`
- Prefer descriptive names over abbreviations
- Keep domain-specific names stable across related documents
- Use American English for all repo-level rules, scripts, and code comments

## Directory rules

- `doc/README.md` is the navigation index
- `doc/status.md` is the execution checkpoint
- `doc/okf/` contains the canonical rules and templates
- `doc/specs/` holds product, behavior, and architecture specs
- `doc/plan/` holds execution and delivery plans
- `doc/standard/` holds coding and quality standards
- `doc/environment/` holds environment bootstrap and operational setup scripts

## Link policy

- Prefer relative links inside the repo
- Use a stable and readable structure between parent and child documents
- When the source document is the canonical one, point to it rather than duplicating its content
- Avoid broken links and stale references

## AI tool compatibility rule

The repo can have tool-specific files such as `AGENTS.md`, `CLAUDE.md`, or Cursor rules, but those files must not become the main document store. They should always behave as short pointers to the canonical docs in `doc/`.

## Minimum content quality

A document is OKF-compliant only if it is:
- clear in purpose
- easy to find through the index
- internally consistent with adjacent docs
- understandable by both humans and AI coding agents
- updated when the project or process changes
- written in American English for all repository-facing text

## Canonical references

- `doc/README.md`
- `doc/okf/structure.md`
- `doc/okf/agent-directives.md`

## Review expectation

Documentation should be reviewed when:
- a repo workflow changes;
- a platform requirement changes;
- a feature architecture changes;
- a tool-specific directive changes.

When this happens, update the canonical document before making the support files narrower or shorter.
