# UI redesign plan

Status: planning. Public app only. Internal tools stay out of this repo.

## Why
Research (WhatCable reviews, PortPulse and USB-C Tester listings, NN/G progressive
disclosure, WCAG 2.2) points the same way:

- Lead with one plain answer, then let people open detail.
- Keep the result, not the raw data, on the first screen.
- Accessibility is a requirement: 4.5:1 text contrast, 24 px minimum targets, visible focus.
- Users arrive with one question: "what is wrong with this cable?"

Current problems (from the browser preview review):
1. No one-line promise. The header says the name, not the job.
2. Every card carries equal weight. The hero answer competes with eight buttons.
3. Findings repeat long sentences. Detail should open on demand.
4. The four tests are stacked. They need one place with tabs.
5. Empty states show large empty cards.

## Principles
1. One answer first: a status, one headline, one action.
2. Progressive disclosure: three levels at most (answer, findings, raw data).
3. Status is never colour alone: word, glyph and colour together.
4. Plain words: "Cable limits charging" not "negotiated_below_limit".
5. Brand: black and white base, colour only for status.
6. Accessible by default: keyboard, screen reader, reduced motion, 44 px targets.

## Information architecture
- Header: logo, name, one-line promise, "Copy summary", "Refresh".
- Hero: status chip, headline, one sentence, one primary action.
- What we found: top 3 findings, "Show all".
- Your ports: one card per port, three key facts, details disclosure.
- Power right now: big figure plus four facts.
- Tests (tabs): Charging, Port watch, Both ends, Drive speed (desktop only).
- Connected devices: five shown, "Show all".
- What this device can show: Yes / Partly / No with reasons.
- Diagnostics (collapsed): full JSON, self-check.

## Visual system
- Tokens on :root for light and dark, with explicit data-theme override.
- Type scale 13 / 15 / 17 / 22 / 30 px, system font stack.
- Spacing on an 8 px grid. Max width 1040 px.
- Status colours: good #13703a, warn #8a4b00, bad #a8261b (each over 6:1 on white).
- Focus: 3 px ink outline with 3 px offset on every control.

## Components
button (primary, ghost, link), status chip, hero, finding row, port card,
fact list, tab bar, tab panel, stat tile, chart, timeline, cable diagram,
step chips, matrix cells, empty state.

## Phases
1. Tokens and layout (index.html, style.css). Keep element ids used by features.js.
2. Rendering (app.js): hero logic, findings top 3, ports, tabs, copy summary.
3. Verify: JS syntax, demo preview at 980 px and 390 px, contrast check, keyboard path.
4. Report to the owner with screenshots; commit locally; push only after approval.

## Acceptance
- Hero answers the question in under five seconds on the demo cases.
- No horizontal page scroll at 390 px.
- Every control reachable by keyboard, with visible focus.
- All text meets 4.5:1 on its background in both themes.
- Reduced motion turns off all animation.
- Feature ids unchanged so features.js still works.

## Out of scope
Paid edition features, internal sandbox, new Rust logic.
