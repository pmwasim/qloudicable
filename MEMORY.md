# MEMORY.md — QloudiCable

Stable facts for agents. Not a log: the journal holds the history, the handoff holds the current state.

## Project

- Name: QloudiCable, by qloudihub.com. Public repository `pmwasim/qloudicable`, Apache-2.0.
- Platforms: Windows, macOS, Android, Ubuntu. Early release; no real-hardware charging readings yet.
- Front end: vanilla JS, no build step, SVG visuals. The owner judges the front end by look and behaviour.

## Owner preferences

- Keep unreleased or internal material out of this public repository.
- Do not push or publish without explicit approval; approval is per action.
- Prefers brief, plain-language results. Wants unverified things named as unverified.
- Accessibility matters: WCAG contrast, reduced motion, visible focus, labels on every control.

## Environment

- ms7978 is the build and preview machine (unmetered network). The MacBook has metered internet.
- Preview: headless Chrome, `--window-size` minimum 500 px.
- GitHub Actions builds all platforms from version tags; release is a draft until published.

## Related

- [AGENTS.md](AGENTS.md) for rules. [LESSONS.md](LESSONS.md) for mistakes to avoid.
