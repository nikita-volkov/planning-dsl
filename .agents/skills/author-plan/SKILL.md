---
name: author-plan
description: Author or refine a plan.yaml for a team (humans and agents) through a structured interview. Use when the user wants to plan a project from scratch, or to edit, extend or re-estimate an existing plan.yaml. Emits only authored fields; never computed ones.
---

# Author a plan

The artifact is `plan.yaml`: tasks as a dependency DAG with roles, executors (humans and agents uniformly) with capacity, one plan-level deadline. It is a description of the world — nothing computed (assignment, schedule, critical path, renders) ever enters the file. The authoritative format reference is `specs/plan-format.md` in the planning-dsl repository; the canonical example at the top of that spec is the style to match.

If a plan already exists, read it and validate it before asking anything: the errors and warnings tell you what the next conversation should fix.

## Interview flow

Work through the stages in order; skip a stage only when the plan already answers it. Keep it light: propose, then confirm — few questions at a time, each with a recommended answer, answerable by number. Never interrogate; a proposal the person corrects is faster than a question they must answer from nothing.

1. **Frame.** What is being planned, and by when? Record `unit: person-hours` and the single `deadline:`. Both are required plan-level fields — do not emit a file until both are known; a placeholder deadline is a lie the scheduler will treat as real.
2. **Roles.** What kinds of work does this team do? Propose a short vocabulary (three to six roles) with one-line descriptions, and confirm the spellings — every task and executor will reuse these exact tokens. The descriptions are for future sessions and editors; write them plainly.
3. **Decompose.** Goals into tasks. Propose the task list and its `depends-on` edges, then grill lightly for the missing ones: what must exist before X can start? What can run in parallel with Y? A task that needs two roles is two tasks.
4. **Estimate.** One person-hours figure per task. If the person hesitates or offers a range, take the middle and say you did. Leaving tasks unestimated is fine — name which ones, because scheduling stays locked until every leaf has a number.
5. **Executors.** Who (or what) does the work? Humans: roles, plus `availability` when they are not full-time (a scalar, or periods for vacations). Agents: roles, plus `concurrency` slots. Tracker `ids` only if they already exist in the tracker — never invent ids.
6. **Emit and validate.** Write `plan.yaml` in the spec's style, validate it, fix every error, and surface the warnings: a role no executor fills is usually a vocabulary miss, not a staffing fact.

## Hard rules

- Authored fields only: `unit`, `deadline`, `roles`; tasks (`status`, `role`, `depends-on`, `estimate`, `blocked-reason`); executors (`ids`, `roles`, `role-weights`, `availability`, `concurrency`). Never write `assignee`, per-task dates, schedule, start/end, critical path, maturity — or `priority`, which does not exist in v1.
- `status` and `blocked-reason` belong to tracker sync. One exception: when creating a plan for work already underway, seed `done` or `blocked` for what the interview establishes happened before the plan existed — then stop touching status.
- A `role:` token must match the declared vocabulary exactly — `Dev` is not `dev`, and the validator will say so.
- Estimates are single numbers in the plan unit. Ranges are a v2 (PERT) feature and stay out of the file.
- The file describes the world as it is, not as it should be: capacity that does not exist yet is fiction the scheduler will happily schedule around.

## Feeding the format back

Every point where the interview strains the schema — a shape the person wants but the format cannot say, a field that keeps feeling wrong — is a candidate change to the format design. Say so at the end of the session; it belongs in `specs/plan-format.md` or `decisions/` in the planning-dsl repository, not silently worked around in one plan.

## Validation

From the planning-dsl repository root:

```
cargo run -q -- validate <path-to-plan.yaml>
```

Zero errors is the bar; warnings are worth surfacing.
