# The format is descriptive and the scheduler is a deterministic layer over it

Tags: plan-format, scheduling

## Rationale

`plan.yaml` describes the world — tasks, dependencies, roles, executors, estimates — and nothing more. Timing is never part of the description: a deterministic scheduler computes start and end dates from estimates plus executor capacity, so any consumer recomputing the schedule from the same file gets the same answer. Estimates are in the schema from day one but optional per task: a plan may contain unestimated tasks, and plans mature by gaining fields, not by passing through declared stages.

## Rejected alternatives

- An imperative planning format (plans as executable programs): planning is description, not execution; the file must stay readable and diffable as a world-state document.
- Estimates mandatory for every task: forces invented numbers into young plans; optionality lets a plan start coarse and mature field by field.
- Stage flags marking a plan's maturity (`draft`, `ready`): see [maturity-is-derived-from-estimates-never-declared](maturity-is-derived-from-estimates-never-declared.md).

## Evidence

Grilling sessions of 2026-11-02 (three rounds); recorded in the session handoff of 2026-11-03.
