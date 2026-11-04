# research/gantt-library-comparison

The hand-rolled HTML gantt (`research/house-renovation-demo/house-renovation.gantt.html`, 33,003 bytes) compared against the two popular gantt libraries, vendored and fully embedded, rendering the same demo schedule. Two generated files:

- `house-renovation.gantt.frappe.html` — frappe-gantt 1.2.2
- `house-renovation.gantt.dhtmlx.html` — dhtmlx-gantt 10.0.3

Both were opened and exercised in a browser (screenshots/); the interactions verified for each: bars, executor grouping, dependency arrows, status/critical colors, today marker, popup/tooltip with artifact fields, zoom levels, Cyrillic text.

**This folder is evidence only.** `src/`, `tests/` and `autodecisions/` are untouched — the read-only single-file discipline (`autodecisions/the-html-gantt-is-a-read-only-view-in-the-html-graphs-grammar.md`) stands as written. If a library wins, wiring it into the renderer is the human's call.

## Assessment

The native file does the same job at half frappe's size and 4% of dhtmlx's, with zero vendored bytes, byte-deterministic geometry computed Rust-side, and the whole 91-day plan visible in one screen — which neither library achieves at default density (both need scrolling: frappe at 48 px/row × 30 rows, dhtmlx at 36 px/row with smart rendering). What the libraries buy for those bytes: editing (drag/resize/lightbox — all disabled here as a matter of discipline, not capability), out-of-the-box zoom UI, and per-bar features we never needed. Neither library has native executor grouping in its free edition — both are emulated here. The headline finding is dhtmlx's license: v10 ships MIT, not the GPLv2 the free edition carried through v9.x, which dissolves the "GPLv2 in every generated artifact" objection — for v10; pinning v9.x and below would revive it.

## Provenance

Downloaded fresh from the npm registry on 2026-11-04; each tarball extracted into its own empty directory, dist files copied out unchanged.

| package | version | tarball | tarball sha256 | license |
|---|---|---|---|---|
| frappe-gantt | 1.2.2 | https://registry.npmjs.org/frappe-gantt/-/frappe-gantt-1.2.2.tgz | `2e8c82de7b5423466cf28e7e6f52c163f34f2e65dedf2b0e2b5ea217529fc2c5` | MIT |
| dhtmlx-gantt | 10.0.3 | https://registry.npmjs.org/dhtmlx-gantt/-/dhtmlx-gantt-10.0.3.tgz | `ce21c3296d72bce4afb26f6cd5bc29e664e367f321499054fc444210e82a828d` | MIT (Community edition) |

Vendored files (in `vendor/`, embedded verbatim by `generate.py`):

| file | bytes | sha256 |
|---|---|---|
| frappe-gantt-1.2.2/frappe-gantt.umd.js | 48,175 | `f5b9de3b618b72e675dfa7f159f2091fcf2055e62826309dfc4604d7ca649e41` |
| frappe-gantt-1.2.2/frappe-gantt.css | 6,848 | `74ccfd2390849e729cda95a358439f003b79b94770ff732713217bc8045e7a69` |
| frappe-gantt-1.2.2/LICENSE.txt | 1,095 | `f0ee9b1340ea287eacdc522f04f85b80304c2009c237809920ff9cf7684fc4f1` |
| dhtmlx-gantt-10.0.3/dhtmlxgantt.js | 631,897 | `afc8b0bfcdb7d8d7a2f6ec9befbcc473ed2823f102a7994cad16c3aed987d3c5` |
| dhtmlx-gantt-10.0.3/dhtmlxgantt.css | 142,729 | `572519177e6ec6c17fc50e13f161dca25c294673a86ce622676ed1c394bebefe` |
| dhtmlx-gantt-10.0.3/LICENSE.md | 1,099 | `2227d175570548d6b0ea3c286878dd11225b0eaec6f6eb5e125f6b0465fdb717` |

License points, verbatim from the shipped files:

- **frappe-gantt**: `package.json` says `"license": "MIT"`; `license.txt` is the MIT text. No runtime dependencies — the 1.x rewrite dropped moment.js (it ships its own `date_utils`), so no companion needed, nothing extra vendored.
- **dhtmlx-gantt**: `package.json` says `"license": "MIT"`; `LICENSE.md` is "The MIT License (MIT), Copyright (c) 2026 XB Software". The brief expected GPLv2 — that was true of the free/Standard edition through **v9.x and below**. Starting with v10 the `dhtmlx-gantt` npm package is the MIT-licensed Community Edition (dhtmlx's own announcement and installation docs say so; PRO features remain commercial). So the honest statement: **the vendored bytes are MIT**, and if the human ever preferred the older GPLv2 editions, every generated HTML artifact carrying the inlined v9.x bundle would be a GPLv2 covered work — copyleft obligations attached to each rendered schedule file, which an intended open-format standard cannot afford.

## Comparison

| | native (hand-rolled) | frappe-gantt 1.2.2 | dhtmlx-gantt 10.0.3 |
|---|---|---|---|
| generated file, bytes | 33,003 | 67,059 | 792,049 |
| vendored lib, bytes | 0 | 56,118 (js+css+license) | 775,725 (js+css+license) |
| license | none (project code) | MIT | MIT (v9.x and below: GPLv2) |
| bars per dated task | 23 | 23 | 23 |
| done tasks | skipped (no dates) | skipped | skipped |
| rows grouped by executor | yes, first-appearance bands | emulated — full-span pseudo tasks, centered label | emulated — project summary rows, collapsible |
| dependency arrows | 36, crit ones red | 36 | 36 |
| status colors (todo/in-progress/blocked) | fill classes | per-task `color` field | `task_class` template |
| critical path | red stroke + red arrows | red stroke via `custom_class` (arrows stay gray) | red border via `task_class` (arrows stay gray) |
| today marker | dashed line, viewer clock, only inside window | day highlight + ball, viewer clock, visible in padding days too | cell shading (scale+timeline), viewer clock, only inside window |
| popup | bottom-left card, click-pin | popup on click: name, executor, start, end, active-days, status | hover tooltip + quick_info on click, same fields |
| axis tiers | 2 (month + day) | 2 (month + day) in Day view | 2 (month + day) at day level |
| zoom levels | wheel zoom + fit, Rust geometry | dropdown: Hour/Quarter Day/Half Day/Day/Week/Month/Year | zoom ext wired, 3 buttons: day/week/month |
| weekend shading | none — artifact carries no calendar | **on by default** (Sat/Sun tinted #f7f7f7) | **off by default** (no nonworking-time classes in the bundle) |
| Cyrillic | fine | fine | fine |
| offline | fully | fully | opens offline; CSS `@font-face`es Inter from fonts.gstatic.com, so offline it falls back to a system font |
| readonly configuration | by autodecision | `readonly: true` (drag/progress editing disabled; popup keeps working) | `readonly: true` (lightbox/add/edit disabled; quick_info keeps working) |

Both libraries carry editing modes (frappe drag/resize/progress by default; dhtmlx lightbox, auto-scheduling engine) that this demo deliberately disables — the read-only autodecision would bind any wired-in renderer too. The files are configured `readonly` so what the human sees is the configuration that could actually ship. `auto_scheduling` is explicitly off in the dhtmlx file: the artifact's dates are the computed truth and must not be recomputed client-side.

## What was hacked or configured

- **frappe, grouping**: frappe 1.2.2 has no grouping option — executor bands are emulated with one full-span pseudo task per executor (`custom_class: 'grp-header'`, styled to the native band color), which also pins the chart window to the artifact's start..plan-end. Header labels center on the band; the native's are left-aligned.
- **frappe, `custom_class` takes exactly one token**: `classList.add('a b')` throws `InvalidCharacterError`, so status colors go through frappe's per-task `color` field (inline fill) and only `crit` rides the custom-class hook. Two status colors could not share `custom_class` with `crit`.
- **frappe, today**: the today highlight renders from the viewer's clock, and because frappe pads the window ±7d, a today before plan-start is still visible (the native clips to the window and draws nothing).
- **dhtmlx, grouping**: the community bundle has no `groupBy` (the plugin is referenced only in a feature map, never defined) — executor grouping is emulated with `type: 'project'` summary rows; children hang off them and the summary bar spans the group's date range.
- **dhtmlx, today**: the community bundle ships neither the marker extension nor `addTaskLayer` (both `undefined` at runtime), so the today date is highlighted through `scale_cell_class`/`timeline_cell_class` — a tinted column, not the native's dashed line. Verified by probing with the window widened to include 2026-11-04 (`screenshots/dhtmlx-today-highlight-probe.png`); the shipped window starts 2026-11-05, so today falls outside and nothing is drawn — the native's inside-window rule.
- **dhtmlx, end dates**: `end_date` is exclusive; the generator emits end+1 day so bars cover start..end inclusive, and the grid's End column therefore shows the artifact's end + 1 day. The tooltip reports the artifact's inclusive end from the embedded meta.
- **dhtmlx, zoom**: the zoom extension is bundled but adds no UI of its own — three buttons (day/week/month) call `gantt.ext.zoom.setLevel`; verified switching.
- **inlining safety**: the vendor files contain no `</script`/`</style` sequences (checked), so both are embedded verbatim; the embedded JSON escapes `</` as `<\/` defensively.

## Differences left as defaults, recorded rather than tuned

- **Weekend shading**: frappe highlights weekends out of the box (dhtmlx does not). Neither file tweaks it. The artifact carries no calendar — the autodecision's rationale for the native's no-shading applies to any library too; wiring either lib would mean deciding whether to suppress weekend highlights (frappe: `holidays: {}` + `ignore`) or hand it the authored calendar.
- **Information density**: the native gantt fits the whole plan in one viewport; both libraries default to larger rows/columns and scroll. frappe's density is configurable (`bar_height`, `padding`, `column_width`) — left at defaults here.
- **Critical-path arrows**: the native paints the chain's arrows red; frappe offers no per-arrow class hook, and no dhtmlx link-class hook was wired — both files mark the chain on bars only.

## Regenerate

```sh
# one-time vendoring (what was done, from an empty directory per package):
npm view frappe-gantt version dist.tarball   # 1.2.2
npm view dhtmlx-gantt version dist.tarball   # 10.0.3
curl -sSfL -o frappe.tgz https://registry.npmjs.org/frappe-gantt/-/frappe-gantt-1.2.2.tgz
curl -sSfL -o dhtmlx.tgz https://registry.npmjs.org/dhtmlx-gantt/-/dhtmlx-gantt-10.0.3.tgz
# extract each into its own empty dir, copy dist files into vendor/ as above

# regenerate both HTML files (stdlib-only; deterministic — same inputs, same bytes):
python3 -I generate.py
```

`generate.py` reads the demo's `house-renovation-schedule.yaml` (start/deadline/plan-end/critical-path + per-task executor/dates/active-days/status) and `house-renovation.yaml` (depends-on edges) with a hand-rolled reader for these two known files, embeds the task data as JSON, and writes both HTMLs with the library JS+CSS inlined. No timestamps, no randomness — the bytes are a pure function of the inputs.

## Screenshots

- `screenshots/native-baseline.png` — the native gantt, whole plan in one screen
- `screenshots/frappe-popup.png` — frappe Day view, popup open (executor/start/end/active-days/status)
- `screenshots/frappe-week-view.png` — frappe switched to Week via its dropdown
- `screenshots/dhtmlx-quick-info.png` — dhtmlx Day view, quick_info card open
- `screenshots/dhtmlx-week-view.png` — dhtmlx zoomed to week level via the buttons
- `screenshots/dhtmlx-today-highlight-probe.png` — window widened to include today, today-column tint visible

## Untouched

`src/`, `tests/`, `autodecisions/`, `decisions/`, `facts/` are byte-unchanged by this research; the CLI renders the native gantt exactly as before. This folder exists so the human can make the library call with both candidates rendered against the same demo.
