# UI/UX audit of the redesign (af8d8c6)

Method: rendered desktop (1040 px), phone (390 px target, 500 px min in headless Chrome)
and dark theme in the browser preview with sample data; DOM probe for targets, headings
and labels. Severity: P1 blocks understanding, P2 slows use, P3 polish.

## Findings

| # | Sev | Finding | Evidence | Fix |
|---|---|---|---|---|
| 1 | P1 | "In use: CC" shows a wire name as if it were a result | Port card facts | Replace with plain text or hide when not useful |
| 2 | P1 | Jargon in facts and details: "Data role", "Power role", "Plug side", "e-marker", "PDO", "CC" | DOM labels, "More about this port" | Plain labels, tooltip or glossary link on first use |
| 3 | P1 | Headline repeated: hero and first finding say the same thing, with the same detail | Desktop shot | Hero links to the finding; findings start at the second item when the hero covers it |
| 4 | P2 | Evidence tags ("Reported by the hardware") repeat on every finding | Desktop shot | Move to an info popover, or a single legend |
| 5 | P2 | Phone tabs cut off the last tab with no cue it scrolls | Phone shot | Edge fade and a small scroll hint, or a 2x2 grid |
| 6 | P2 | No "last checked" time, so the 3 s refresh is invisible and trust is low | Whole page | "Updated 10:42:05" under the hero, updated on each change |
| 7 | P2 | Long page (about 2400 px at desktop). Power, devices and access sit below the tests | DOM probe | Put Power next to the hero; collapse devices and access by default |
| 8 | P2 | Charts and timeline have no text table; screen-reader users get a label only | DOM probe: svg[role=img] with no table | Add a short "Data" details under each chart |
| 9 | P2 | Both-ends cable diagram labels "End A/End B" and "Port"/"Other end" are small and abstract | Both-ends panel | Larger labels and a one-line caption under the diagram |
| 10 | P3 | "Source: AppleSmartBattery" and "Refresh" are developer words | Power card, header | "Read from the battery" and remove Refresh (live already) |
| 11 | P3 | Skip link shows 40 px high when focused. Fine, but it is the only target under 44 px | DOM probe | Keep as is; it is off-screen by default |
| 12 | P3 | Dark hero inverts to a light block, which is strong but flips the hierarchy in dark mode | Dark shot | Keep one dark-to-light rule, check it reads as intended |

## What works
- One hero answer with one action, and the action matches the problem.
- Status has a word, a glyph and a colour.
- Headings are in order (h1, then h2 sections, then h3 ports).
- Contrast passes in both themes.
- Tests are one tab bar, with arrow-key support.
- Privacy line is visible at the foot.

## Next pass (in order)
1. Fix findings 1, 2 and 3: plain labels, glossary, de-duplicate the hero.
2. Add the "Updated" time (6) and drop Refresh (10).
3. Reorder: hero, findings, power, ports, tests, devices, access, diagnostics (7).
4. Phone tab overflow cue (5).
5. Chart data tables (8).
