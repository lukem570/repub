# Repub Agent Guide

Repub is a Rust reimplementation of Microsoft Publisher. This file tells agents how to work in the repo.

## Conventions

- Crates live in `crates/`, named `repub-*`. Add new ones to the workspace `members` in the root `Cargo.toml`.
- Use workspace dependencies (`{ workspace = true }`); don't pin versions per-crate.
- Match the surrounding code style; keep changes consistent with the crate you're in.
- No decorative banner or section-divider comments (e.g. `// ===== Title =====` or boxed `====` rules). Use plain doc comments and split large files into modules instead.
- Write tests alongside new functionality. Verify with `cargo build --workspace` and `cargo test`.
- Don't modify files outside the scope of your task.

## Key references

- `docs/ARCHITECTURE.md` — overall design and crate responsibilities.
- `docs/ux/` — desktop UI specification.

## Working with subagents

When spawning a subagent, give it: the task, relevant file paths, the acceptance criteria, clear boundaries on what it may change, and where to report results. Run independent tasks in parallel; sequence them only when one depends on another's output.

## Typical flow for a Publisher feature

1. **Implement** the types and logic in the relevant `repub-*` crate.
2. **Test** — unit tests, round-trip/serialization tests, and edge cases.
3. **Document** — rustdoc on public items; update `docs/` if design changed.

## When things break

- Won't compile: check workspace members, dependency paths, and circular deps.
- Tests fail: run the single failing test for detail, check fixtures and assumptions.
- Stuck: narrow the task, try another approach, or escalate with more context.
