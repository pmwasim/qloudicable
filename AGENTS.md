# AGENTS.md — QloudiCable

Entry point for any agent working in this repository. Read this first, then the files below in order.

## Read order

1. This file: scope, rules, where things live.
2. [AGENTS-HANDOFF.md](AGENTS-HANDOFF.md): current state, open decisions, what to do next.
3. [MEMORY.md](MEMORY.md): stable facts about the project and the owner's working preferences.
4. [LESSONS.md](LESSONS.md): mistakes already made. Read before repeating a class of action.
5. [AGENTS-JOURNAL.md](AGENTS-JOURNAL.md): dated checkpoints. Append to it; do not rewrite earlier entries.
6. [docs/STATUS.md](docs/STATUS.md), [docs/DESIGN.md](docs/DESIGN.md), [docs/RELEASING.md](docs/RELEASING.md): the public record of what is verified and how releases work.

## Project

QloudiCable is a USB-C cable diagnostic tool by qloudihub.com for Windows, macOS, Android and Ubuntu. Rust workspace (core logic, per-OS backends, CLI, Tauri app) plus a vanilla-JS front end with no build step. Licence: Apache-2.0.

## Rules

- **Public repository.** Nothing unreleased or internal may appear in this repo: code, docs, commits, UI, PR text. Keep such notes outside the repo.
- **No push, merge or publish without explicit approval in the current conversation.** Local commits are fine when asked. Approval for one push does not carry to the next.
- **Owner checks the GUI and visuals only.** The backend is the agents' responsibility; judge the front end by how it looks and behaves, and say what was not verified.
- **Verify before reporting "done".** Run the tests, or say they were not run. Say what could not be checked on hardware.
- **Commit style:** conventional prefixes (`feat`, `fix`, `docs`, `ci`, `refactor`), a short subject, and a body that says why. End agent-made commits with the `Co-Authored-By` line the session specifies.
- **Build box:** heavy builds and the dashboard preview run on the ms7978 machine; the MacBook's internet is metered, ms7978's is not.
- **Protected files:** do not change this file's rules without the owner's review. Propose changes in the journal first.

## Where things live

- `crates/` — core logic, per-OS backends (Linux sysfs, macOS IOKit, Windows hub IOCTLs, Android JSON facts), CLI, Tauri plugin.
- `app/` — Tauri 2 app; `app/ui/` is the front end (`index.html`, `app.js`, `features.js`, `viz.js`, `style.css`).
- `docs/` — public status, design, QA and release docs. `docs/design/` holds audits and user-test notes.
- `scripts/`, `.github/workflows/` — release tooling and CI.
