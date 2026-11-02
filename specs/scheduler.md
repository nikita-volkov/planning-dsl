# The scheduler

Tags: scheduler

A deterministic layer over the authored plan: one pure function, `(validated plan.yaml, start date) → schedule artifact`. It applies fixed rules in a fixed order and invents nothing, so the same plan and start produce a byte-identical schedule. TaskJuggler's semantics are the reference model, minus its priorities — urgency here is derived from the DAG, never authored ([taskjuggler-solves-deterministic-scheduling-and-has-been-dormant-since-2020](../facts/taskjuggler-solves-deterministic-scheduling-and-has-been-dormant-since-2020.md)). The tie-break under capacity contention — the one point the plan format deliberately left to this spec — is settled in the [tie-break section](#the-tie-break-under-contention) and recorded as an [autodecision](../autodecisions/capacity-contention-goes-to-the-longest-downstream-path-then-declaration-order.md), which the maintainer promotes to `decisions/` by accepting it.

**Draft for grilling, 2026-11-03.** Not yet accepted; no code follows it.

## Inputs and outputs

- **In:** a plan that passes the validator — the scheduler re-runs the checks and refuses on any error — plus a start date. The start is a scheduler input, not file data: the file holds exactly one date by decision ([the-deadline-is-plan-level-only](../decisions/the-deadline-is-plan-level-only.md)), and a forward pass over availability periods needs a second, because capacity differs by calendar day. Default: the earliest `from:` in any availability period; with no periods declared and no explicit start, the scheduler refuses and asks for one. Anchoring at run time ("today") is excluded — it would break byte-identical output, the layer's defining property.
- **Out:** the schedule artifact, written to stdout or an explicit `--out` — exactly the outputs the plan format reserves for computation: the assignment (task → executor, derived by role matching per [assignment-is-derived-by-role-matching-and-the-plan-stays-role-only](../decisions/assignment-is-derived-by-role-matching-and-the-plan-stays-role-only.md)), per-task start/end dates and active days, the critical path, slippage against the deadline, and the statuses — everything a Gantt needs. When the estimate gate refuses, its report is the maturity output: which tasks still lack estimates ([maturity-is-derived-from-estimates-never-declared](../decisions/maturity-is-derived-from-estimates-never-declared.md)).

The artifact is regenerable and is never written back into `plan.yaml` ([computed-outputs-are-separate-regenerable-artifacts](../decisions/computed-outputs-are-separate-regenerable-artifacts.md)). Drawing the Gantt is the renderers' job; the scheduler emits the data and stops.

## The capacity model

One constant: **1.0 availability = 8 person-hours per day**. Every calendar day is a working day — there is no weekend concept; availability periods are the only calendar texture.

An executor's pool on day d is `availability(d) × 8 h`:

- Scalar availability → that fraction on every day. Period list → the fraction of the period covering d; `until` is **inclusive** (the validator's overlap rule reads it that way); a day in no period → 0. No availability field → 1.0.
- With `role-weights`, the pool is pre-partitioned: role r gets `w_r/Σw` of it, always — while doing dev work, ivan works at 2/9 speed ([role-weights-split-one-executors-capacity-across-roles](../decisions/role-weights-split-one-executors-capacity-across-roles.md)). An unused share stays unused: weights are a cap, not a demand. Without weights the pool is undivided — `roles:` is what an executor *can* do, and any one of it can have the whole pool. `roles` and `role-weights` are two shapes of the same answer; an executor carries one, not both — the scheduler treats both as an input error.
- `concurrency` (default 1) caps how many tasks are active at once. It never multiplies capacity: parallel tasks split their role's pool for the day equally — a 4-slot review-agent at full availability runs four tasks at 2 h each ([executors-are-humans-and-agents-with-availability-and-concurrency](../decisions/executors-are-humans-and-agents-with-availability-and-concurrency.md): parallelism, not more calendar).

An active task holds its slot through zero-capacity days: a vacation pauses work, it does not reshuffle it.

## The algorithm

**Gate.** Scheduling is locked until every not-done task carries a positive estimate. Done tasks are exempt — the canonical example's `design-architecture` is done and unestimated. "Leaf" in the maturity decision reads as *every not-done task*: the DAG has no hierarchy, and an unestimated interior task blocks its dependents exactly as a leaf blocks the end. On refusal the scheduler lists the offenders and schedules nothing — a partial schedule reads as a plan but is not, and the critical path through an unestimated task is a guess.

**Static pass.** Each task gets a **downstream weight**: its estimate plus the largest downstream weight among the tasks that depend on it (sinks weigh their estimate alone; done dependents contribute nothing). Computed in one reverse-topological pass, Kahn's algorithm with the ready set ordered by declaration order. **Declaration order = document order in `plan.yaml`**, for tasks and executors alike; the typed model must preserve it (the current `BTreeMap` sorts it away — an implementation note, not a format change).

**Daily pass.** From the start date, one pass per calendar day until every not-done task has ended:

1. *Ready:* tasks not started whose dependencies all ended before today — a dependency ending today unblocks its dependents tomorrow (finish-to-start, next-day). A done dependency counts as ended before day one.
2. *Activation:* ready tasks in contention order — descending downstream weight, then declaration order (the [tie-break](#the-tie-break-under-contention)). For each: eligible executors fill its role, hold fewer than `concurrency` active tasks, and have a positive pool for its role today. If none, the task waits. Otherwise it goes to the eligible executor with the least outstanding work for its role — the sum of estimates of that executor's assigned, not-yet-ended tasks of the role — ties in declaration order. Start = today. Assignment happens once, at activation; it never migrates. The rule balances load, not speed: deterministic first, optimal never.
3. *Draw:* every active task adds its equal share of its executor's role pool for today to its cumulative work.
4. *End:* tasks whose cumulative reaches their estimate end today (end = today) and free their slots for tomorrow. Leftover capacity on a day is idle — no intra-day packing.

A plan whose capacity runs dry is caught, not spun on: if no task is active and no executor has positive capacity on any later day, the scheduler refuses, naming the stranded tasks.

**Critical path.** After the last day: start from the not-done task with the latest end (ties: declaration order), and repeatedly step to its dependency with the latest end (ties: declaration order), skipping done dependencies, until a task with no dependencies. One chain. Where contention, not precedence, delayed a start, the artifact's dates show it — v1 does not model contention edges.

**Slippage.** Plan end against the deadline: meets it, or late by N days. `latest-start` = deadline − (span − 1): the last start date that still lands.

## Determinism

Same plan, same start → byte-identical artifact.

- No wall clock, no randomness, no environment: the only inputs are the file and the start date.
- Every iteration order is fixed — declaration order for tasks and executors — and every tie in every rule above is spelled out.
- Arithmetic runs in one fixed order (the daily pass defines it); the artifact serializes in one fixed field order.

## What it refuses

- A plan with validator errors; a not-done task without an estimate (listing them all).
- A not-done task whose role no executor fills. The validator warns — young plans have partial rosters — but the scheduler refuses: there is nobody to assign, and a default executor would be invented capacity.
- Capacity that ran dry (above). It never widens a fraction, never exceeds a weight partition, never promotes a role.
- Anything back into `plan.yaml`. The scheduler is read-only over the file, always.

## Status and the schedule

Status is display data that flows back from trackers ([sync-is-asymmetric-structure-flows-out-and-status-flows-back](../decisions/sync-is-asymmetric-structure-flows-out-and-status-flows-back.md)); the scheduler projects capacity, not tracker state:

- `done` — satisfied: occupies no capacity, unblocks its dependents from day one, takes no dates, stays off the critical path; its status rides through to the artifact.
- `blocked`, `in-progress` — scheduled like any todo, full estimate from the start. There is no percent-complete credit to take (rejected in [status-is-a-four-value-enum-and-blocked-reason-is-written-by-tooling](../decisions/status-is-a-four-value-enum-and-blocked-reason-is-written-by-tooling.md)); when the tracker brings the truth back, the next run re-schedules.

## The tie-break under contention

When ready tasks compete for scarce capacity — one executor's slots, one role's pool — they are considered in this order:

1. **Longest downstream path first:** descending downstream weight from the static pass. The DAG is the only urgency signal the plan carries; a task with more work hanging off it delays the plan's end more for every day it waits, so serving it first preserves the makespan. A fixed heuristic, not an optimization claim.
2. **Declaration order** as the stable final tie: a total order the author controls and can predict. Truly-tied tasks are symmetric, so any fixed order is fair — the author's is the one that reads honestly.

Recorded as an autodecision: [capacity-contention-goes-to-the-longest-downstream-path-then-declaration-order](../autodecisions/capacity-contention-goes-to-the-longest-downstream-path-then-declaration-order.md) — an agent session's choice, binding until changed; the maintainer promotes it to `decisions/` by accepting it. Its rejected alternatives (declaration order alone, shortest-duration-first, an authored `priority:` field) live there.

## The schedule artifact

Sketch — YAML, fixed field order, tasks in declaration order:

```yaml
# schedule.yaml — computed, regenerable; never written into plan.yaml
start: 2026-11-04
deadline: 2026-12-18
plan-end: 2026-12-21
slippage-days: 3
latest-start: 2026-11-01
critical-path: [plan-out-the-tasks, implement-services]
tasks:
  design-architecture:
    status: done
  plan-out-the-tasks:
    executor: nikita
    start: 2026-11-04
    end: 2026-11-13
    active-days: 8
    status: todo
  # ...
```

Renderers consume this; nothing recomputes it.

## Grilling

The maintainer answers by number. Each question carries a recommended answer.

1. **Where does the second date come from?** The file holds one date by decision, but a forward pass over availability periods needs a start — capacity differs by calendar day, so the schedule is a function of the plan *and* the start. Recommendation: `start` is a scheduler input, defaulting to the earliest availability `from:` and refused when the plan declares no periods and none is given. Anchoring at run time would break the byte-identical guarantee, which is the layer's defining property. This fills the plan format's "produces start/end dates" without touching the format.
2. **Over-capacity plans: schedule late, or refuse?** Recommendation: always schedule to completion and report slippage — plan end vs deadline, per-task late flags. The deadline decision exists precisely to anchor slippage reporting; refusing tells the author nothing about how late, and truncating at the deadline lies.
3. **The calendar constants.** Recommendation: every day is a working day (no weekend concept), and 1.0 availability = 8 person-hours — one constant, stated here. Vacations and part-time are exactly what availability periods express; a weekend concept would need a field the format lacks and would split humans from agents against the uniform-executors decision. A Mon–Fri calendar is the alternative if the Gantts read wrong.
4. **Concurrency × role-weights semantics.** Recommendation: the pool model above — `concurrency` caps simultaneous tasks and never multiplies capacity; weights pre-partition the pool and an unused share stays unused; parallel tasks of a role split that role's pool equally; an executor carries `roles` or `role-weights`, never both. The sharpest edge to confirm: an unweighted two-role executor with concurrency 2 runs both roles in parallel, each at half speed.
5. **Shape.** Recommendation: `planning-dsl schedule plan.yaml --start 2026-11-04` — a subcommand of the one Rust binary ([the-tooling-is-written-in-rust](../decisions/the-tooling-is-written-in-rust.md)), emitting the schedule artifact to stdout or `--out`. The artifact is the machine contract between the scheduler and the Gantt/mermaid renderers; renderers consume it, never recompute it.
