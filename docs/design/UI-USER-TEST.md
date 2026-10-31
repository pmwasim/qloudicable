# Dashboard audit and scripted user test

Scope: interactive dashboard (commit 6246e93, local on ms7978, not pushed).
Method: headless Chrome on the preview (demo=mac, cable=ok), a script that runs seven user tasks and counts small targets and unlabelled controls. Checked at 1280x800 and 500x757.
Limit: this is a scripted run, not a test with people. It shows what the page does, not what people understand.

## Bugs found

| # | Severity | Finding | Effect | Status |
|---|----------|---------|--------|--------|
| 1 | High | `el()` in `app/ui/app.js` writes the `data` prop as a plain attribute (`data="power"`), so tiles have no `data-id`. | The tile reuse map reads `dataset.id`, which is always undefined, so every refresh rebuilds all tile buttons. Selecting a tile by id also fails. The scripted tile tap failed on this. | Fixed in preview copy only: `el()` sets `data-*` attributes, tile uses `"data-id"`. Tile test then passes. |
| 2 | High | "Findings (n)" opens `sheet-details`, but nothing fills `#findings`. | The count says there are findings, the sheet is empty. This is the main route to "why". | Open. Fill the list from `findingsFor(r)` when the sheet opens. |
| 3 | Medium | Demo mac scenario always emits `cable_limits_charging`, even with `cable=ok`. | The "ok" demo shows a cable warning, which misleads anyone reviewing the visuals. | Open. Demo data only (`app/dev/demo.js`). |

## Scripted tasks

| Task | Result at 1280x800 | Result at 500 px |
|------|--------------------|------------------|
| 1. Is my cable OK? (headline, status, action) | Pass: "Cable is limiting charging speed", status "Check", action "Run the both-ends test" | Same |
| 2. Open a reading to see how it was measured | Pass after fix 1 (detail title "Cable", dash `detail=open`) | Same |
| 3. Open the findings | Fail: sheet opens, 0 items listed (bug 2) | Same |
| 4. Start a test from the Tests chooser | Pass: chooser opens, charge test opens, start button "Start charging test" | Same |
| 5. Close with Escape | Pass | Pass |
| 6. Dock: Ports, then Overview | Pass: Ports selected (`aria-pressed`), Overview clears the detail pane | Same |
| 7. Keyboard reach | 17 visible controls | 17 visible controls |

Accessibility checks: no page scroll at either size; 0 unlabelled controls; 1 control under 44 px (the "Skip to the dashboard" link, 38 px).

## UX notes (not bugs)

- The status pill says "Check". It is vague next to a headline that is already specific. Use the severity word the findings already use ("Warning", "Action needed") so the pill and headline agree.
- The "ok" cable shows a warning (bug 3). A user can't tell a healthy cable from a limited one until the demo data is fixed.

## Not verified

- Phone width below 500 px (390 px). Headless Chrome's minimum is 500 px.
- Dark mode on the dashboard. The contrast check covered the earlier layout only.
- Screen reader behaviour and real keyboard focus order.
- Any test with people.

## Next steps (need your go-ahead)

1. Apply fix 1 to the repo, fill the findings list (fix 2), and correct the demo (fix 3). Commit locally; no push.
2. Re-run the scripted test at 390 px and in dark mode, once a tool can go below 500 px.
3. Run a short test with 3 to 5 people: "Is my cable OK?", "Why?", "Run a test".
