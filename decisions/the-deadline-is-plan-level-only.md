# The deadline is plan-level only

Tags: plan-format

## Rationale

One `deadline:` date at plan level — for the whole project — and no date on any task. Task timing is the scheduler's computed output; a per-task deadline would be a second, hand-set timing layer that the schedule must then be reconciled against, dragging priorities back into the file through the back door. The whole-project deadline is the single honest constraint a planner actually has, and it is exactly what the scheduler needs to report slippage against.

## Rejected alternatives

- Per-task deadlines: considered and rejected by the maintainer — they multiply into the renegotiation mess the plan exists to avoid, and they conflict with derived start/end dates.
- Milestone dates: same problem one level up; a milestone is a task with a decorator.
- No deadline at all: slippage reporting loses its anchor; one plan-level date is cheap.

## Evidence

Grilling sessions of 2026-11-02 (three rounds); recorded in the session handoff of 2026-11-03.
