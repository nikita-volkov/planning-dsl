# The HTML gantt is a vendored dhtmlx-gantt

Tags: render, cli, schedule-artifact, html-gantt

## Rationale

The maintainer compared the hand-rolled HTML gantt against vendored frappe-gantt and dhtmlx-gantt rendering the same demo (`research/gantt-library-comparison`, commit 9a6157f — 792 KB dhtmlx vs 67 KB frappe vs 33 KB native, all three opened and exercised side by side) and chose dhtmlx on 2026-11-04: «frappe полный кал. dhtmlx мне понравился». The comparison file is the specification: the renderer emits that configuration fed from this repo's data, not a re-design.

This retracts `autodecisions/the-html-gantt-is-a-read-only-view-in-the-html-graphs-grammar.md` — its no-libs sense is dead, struck down the same way the HTML graph's layout-in-Rust was (`decisions/the-html-graph-is-built-on-a-vendored-standard-library.md`). What survives from the old discipline, spelled out:

- read-only: `readonly: true` — no lightbox, no editing; quick_info keeps working;
- the artifact's dates are the computed truth: `auto_scheduling: false` — nothing recomputes them client-side;
- offline: one self-contained file, the bundle embedded verbatim, zero network references — the stylesheet's Inter `@font-face` rules point at fonts.gstatic.com and are stripped at render time, the system-font fallback the stylesheet's own stack names takes over;
- byte determinism: same plan and artifact in, byte-identical file out; the today tint is drawn by the viewer's browser from the viewer's clock, only inside the window, never baked into the bytes;
- the opt-in flag and its gate are unchanged: `render --gantt-html [<file>]` with the mermaid gantt's loud scheduler refusal, per `autodecisions/the-html-gantt-is-an-opt-in-render-flag.md`.

The bundle is pinned: the dhtmlx-gantt 10.0.3 npm dist files committed under `assets/dhtmlx-gantt-10.0.3/` with the license file, embedded at compile time.

## Rejected alternatives

- **The native hand-rolled renderer** — the incumbent: lost the side-by-side comparison on the human's judgment; what it bought (one-screen density, zero vendored bytes) did not outweigh the library's zoom UI, arrows and readonly handling.
- **frappe-gantt 1.2.2** — rejected outright by the maintainer: «frappe полный кал.» — the verdict names nothing to keep it for; dhtmlx is the one he liked.
- **A CDN script tag** — breaks offline opening and byte review: the artifact would fetch code at view time instead of carrying it.
- **dhtmlx-gantt v9.x and below** — GPLv2: every generated artifact carrying the inlined bundle would be a GPLv2 covered work, copyleft obligations attached to each rendered schedule file. Pin MIT editions (v10 and up).

## Evidence

`research/gantt-library-comparison` (commit 9a6157f): the comparison files, screenshots and the vendored files' sha256s; dhtmlx-gantt 10.0.3 npm tarball `ce21c3296d72bce4afb26f6cd5bc29e664e367f321499054fc444210e82a828d`, MIT (`LICENSE.md`, "Copyright (c) 2026 XB Software"). Maintainer's verdict in session of 2026-11-04: «frappe полный кал. dhtmlx мне понравился». Implemented by the session of 2026-11-06.
