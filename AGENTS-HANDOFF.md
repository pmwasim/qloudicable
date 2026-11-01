# AGENTS-HANDOFF.md — QloudiCable

Current state and what happens next. Update this file when the state changes; record the change itself in [AGENTS-JOURNAL.md](AGENTS-JOURNAL.md).

## State (2026-11-01)

- **Released:** v0.1.0 tag built on all platforms through GitHub Actions (draft release, not published to users).
- **Front end:** the interactive dashboard is committed on the ms7978 checkout (`6246e93`, plus fix `149fb4b`). The Mac checkout of this repo does not have that dashboard yet; the two checkouts have diverged.
- **Audit:** a scripted run of seven user tasks passed after two fixes (tile ids, findings list). Report: [docs/design/UI-USER-TEST.md](docs/design/UI-USER-TEST.md). The 390 px and dark-mode checks are not done.
- **Git:** this checkout is `main`, one local commit ahead of `origin/main` (`f81a1e1`) and six behind. Nothing is pushed in this session.

## Open decisions (owner)

1. Push the local commits, and which checkout is the source of truth for the dashboard.
2. Whether to publish the v0.1.0 draft release.
3. Run the short user test (three to five people) before the next UI change.

## Next steps (in order)

1. Reconcile the Mac and ms7978 checkouts once the owner says which is canonical. Do not force-push.
2. Re-run the scripted test at 390 px and in dark mode. Headless Chrome cannot go below 500 px; find another tool for 390 px.
3. Run the user test; record results in `docs/design/`.
4. Update `docs/STATUS.md` only with things that were actually run.

## Known gaps

- No real-hardware charging readings on any platform (see `docs/STATUS.md`).
- Windows installer and app have not been run on Windows.
- Android: emulator only; battery figures there are simulated.
