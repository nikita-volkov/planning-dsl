# The HTML gantt is a read-only single-file view in the HTML graph's interaction grammar

Tags: render, html-gantt

## Rationale

The dates on the chart live in `plan.yaml` under git — the single source of truth — and the schedule artifact is a computed, regenerable view of them (`decisions/computed-outputs-are-separate-regenerable-artifacts.md`), so the gantt offers nothing to edit: it is a read-only view an author regenerates after editing the plan, the same way the mermaid gantt and the flat tables are.

Everything else it inherits from the HTML graph's recorded discipline. Byte determinism: all geometry is computed Rust-side in integers (day columns, row bands, bar extents, arrow elbows), the inline ES5 script only reads what Rust printed (data-* attributes on the bars), no libs, no CDN, opens offline. Rows group by executor in first-appearance order along the artifact's task declaration order — the mermaid gantt's precedent — and done tasks take no bar, for the same reason: the artifact gives them no dates. No weekend shading and no `excludes`: the artifact carries no calendar, and inventing one here would recompute the pauses the scheduler already folded into the dates (the mermaid gantt's recorded rationale). The today line is drawn by the viewer's browser from the viewer's clock (local date via `getFullYear`/`getMonth`/`getDate`, never `toISOString`, which shifts by UTC) and only when it falls inside the window, so the file's bytes stay deterministic — the clock lives in the viewer, like the window-size fit already does. Interactions — hover dim + highlight of a bar, its direct dependency arrows and their other endpoints, click-to-pin with the four-pixel drag-vs-click rule, pan, wheel zoom, fit — are the HTML graph's script grammar unchanged; the one addition, a bottom-left detail card, is built from the data attributes Rust printed, with the unit named once in the corner legend.

Chosen by the implementing session on 2026-11-04, without the human.

## Rejected alternatives

- **Drag editing of bars**: moves the source of truth into a regenerable artifact — the edited dates would live nowhere, and the next render would silently contradict them; what-ifs are plan.yaml edits under git.
- **JS-side multi-scale day/week/month re-layout**: breaks the Rust-computes-geometry discipline the HTML graph established; the script would need date arithmetic and layout state, and the bytes would stop being a function of the input.
- **A Rust-computed today line**: puts a wall clock in the tool and breaks byte-identical output — two runs on different days would produce different files from the same plan.
- **Weekend shading**: assumes a Sat/Sun calendar the authored calendar may contradict (the demos run six-day weeks, agents run `calendar: all`); shading would draw a calendar the artifact does not carry.

## Evidence

Session of 2026-11-04 implementing the HTML gantt; determinism pinned by `the_same_schedule_renders_the_gantt_html_byte_identically`, bars/statuses/chain by `the_gantt_html_draws_one_bar_per_dated_task_and_marks_chain_and_statuses`, and the empty frame by `a_plan_with_everything_done_still_renders_a_valid_empty_gantt_html` in `tests/render.rs`.
