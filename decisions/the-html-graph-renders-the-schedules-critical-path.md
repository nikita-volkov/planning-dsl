# The HTML graph renders the schedule's critical path

Tags: render, html-graph, scheduler

## Rationale

The schedule artifact already carries the chain (`critical-path`), so `render::html` takes it as `Option<&Schedule>` and consumes it, never recomputes — the same principle as the gantt. Nodes named in the chain wear `crit` (red stroke and text); an edge wears `crit` exactly when it is a consecutive pair of the chain — the chain is emitted dependency-first and the scheduler steps only along real dependencies, so consecutive pairs are real edges by construction. At the CLI the schedule is computed once, up front, and best-effort: a scheduling refusal warns and the html still writes, unmarked — the graph must stay useful on early drafts that cannot schedule. In the hover-dim state crit holds 0.4 opacity so the backbone stays readable while a neighbourhood is inspected; the legend gains `red: critical path` only when a non-empty chain was rendered. Done tasks are never on the chain (the scheduler leaves them off), so done muting and `crit` never collide.

## Rejected alternatives

- Putting html behind the gantt's scheduler gate: a plain render of an early draft would lose both files, when only the enhancement needs the schedule.
- A plan-only longest-estimate-chain heuristic: it ignores capacity contention and calendars, so it can point at the wrong chain; the artifact already carries the right one.
- The both-endpoints-critical edge rule: the chain already yields exact edges — consecutive pairs are real edges by construction — so a heuristic can only mislabel.

## Evidence

Accepted by the maintainer, 2026-11-04. Rendered in `src/render.rs` (`html` takes the artifact; the `crit` classes, their CSS and the legend line) and `src/main.rs` (the schedule computed once, best-effort); pinned in `tests/render.rs`.
