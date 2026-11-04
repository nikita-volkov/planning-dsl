# The HTML graph labels tasks with role and estimate

Tags: render, html-graph

## Rationale

Each node carries a small second line under the task name — `role · estimate` in muted grey — and the corner legend states the unit once (`unit: person-hours`). The role says who the task is for, the estimate how big it is; both are already declared in the plan, so the graph repeats nothing that needs separate maintenance. The unit is stated once in the legend rather than on every node: one line answers "6 of what?" for the whole graph. Node width follows the longer of the two lines, so the second line never overflows its box.

## Rejected alternatives

- The full string on every node (`6 person-hours`): the unit repeated per node drowns the numbers it was meant to clarify; a corner legend states it once.
- Estimate only: a bare number loses the assignment dimension the planner plans with.
- A tooltip only (`<title>`): invisible until hover; the point is to see role and size at a glance, with the title kept as a fallback.

## Evidence

Accepted by the maintainer, 2026-11-04. Rendered in `src/render.rs` (`html`: the `sub` text line, the `#legend` div); pinned in `tests/render.rs`.
