# Capacity contention goes to the longest downstream path, then declaration order

Tags: scheduling, contention

## Rationale

When ready tasks compete for scarce capacity — one executor's slots, one role's pool — they are considered in this order:

1. **Longest downstream path first.** Each task carries a *downstream weight*: its own estimate plus the largest downstream weight among the tasks that depend on it (sinks weigh their estimate alone; done dependents contribute nothing), measured in person-hours — the only magnitude estimates provide. Ready tasks run in descending weight. A task with more work hanging off it delays the plan's end more for every day it waits, so serving it first is the makespan-preserving choice; it is the standard list-scheduling priority, and the DAG is the only urgency signal the plan carries. A fixed heuristic, not an optimization claim.
2. **Declaration order** as the stable final tie: document order in `plan.yaml`, earlier first. The order must be total for determinism, and truly-tied tasks are symmetric, so any fixed order is fair — the author's is the one the author can predict and control.

The rule stays explainable to a human team in one sentence ("your longest chain goes first"), which matters for a planner whose consumers are people.

## Rejected alternatives

- **Declaration order alone (FIFO):** makes file order a hidden priority field — worse than an explicit one, because it is invisible and reordering for scheduling reasons churns every diff; and it carries no urgency signal, so a 1-hour stray task can hold a slot ahead of the chain that decides the deadline. FIFO by ready-time instead is circular (arrival order is itself tie-break-dependent) and starves long chains behind a stream of short ones.
- **Shortest-duration-first (SPT):** minimizes mean task latency but inflates the makespan — the plan's end is governed by the longest chain, and finishing many small tasks first pushes that chain right, past the deadline. Optimizes the wrong metric.
- **An authored `priority:` field:** rejected twice over. v1 settled no priority field — the deadline decision explicitly rejected per-task dates as priorities arriving through the back door, and `priority:` is the same through the front door; and in practice every stakeholder marks their task 1, which is the renegotiation mess the plan exists to avoid. The critical path already *is* the priority the plan expresses. If a priority field ever earns its place (v2), it slots above the path rule — authored intent over heuristic — but not now.

## Evidence

Agent session of 2026-11-03, drafting `specs/scheduler.md`; recorded as an autodecision because the session chose it without the human. Accepted by the maintainer on 2026-11-04 with the rest of the scheduler spec, and promoted to `decisions/`.
