# The HTML graph mutes done tasks

Tags: render, html-graph

## Rationale

A task with `status: done` keeps its node and its edges but renders grey — muted fill, stroke and text (`class="done"` on the node group; the legend explains the grey with a `grey: done` line, shown only when something is done). Progress becomes visible without inventing dates: the plan carries no completion records beyond the status itself, so the graph stays a pure function of the plan. Done tasks stay structurally present, because the structure is the truth the graph exists to show.

## Rejected alternatives

- Dropping done tasks the way the gantt does: there a bar needs dates, which a done task has none of; here dropping nodes would lie about what depends on what — a surviving task would seem to depend on nothing.
- A viewer-side filter or collapsed group for done tasks: state in the page, more JS, and the default view would no longer show the whole plan.

## Evidence

Accepted by the maintainer, 2026-11-04. Rendered in `src/render.rs` (the `done` class and its CSS); pinned in `tests/render.rs`.
