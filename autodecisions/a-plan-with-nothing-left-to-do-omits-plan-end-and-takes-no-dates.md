# A plan with nothing left to do omits plan-end and takes no dates

Tags: scheduler, schedule-artifact

## Rationale

When every task is `done`, the scheduler schedules nothing and the artifact carries no `plan-end`, no `slippage-days`, no `latest-start` — even when the plan declares a deadline — and `critical-path: []`. `plan-end` names the end of the remaining work; with zero not-done tasks there is no end to name. Slippage compares the plan end against the deadline, and the critical path starts from the not-done task with the latest end — with no remaining work both have nothing to compare or start from. The artifact still confirms completion: every task's line is `status: done`, and `start:` rides through. Chosen by the implementing session on 2026-11-04, without the human.

## Rejected alternatives

- **`plan-end: <start>`** (the day execution begins): a fake end day — it invents a milestone the plan never states, and a Gantt drawn from it would show a project that began and ended before anything happened.
- **Refusing an all-done plan**: it is a perfectly good plan at maturity 100% — the artifact should confirm completion, not error. Refusing would also make the empty-plan case a special failure mode renderers must handle, for no gain.

## Evidence

Session of 2026-11-04 implementing `specs/scheduler.md`; the case falls out of the daily pass's termination condition ("until every not-done task has ended") with zero not-done tasks. Pinned by `a_plan_with_nothing_left_to_do_takes_no_dates` in `tests/schedule.rs`.
