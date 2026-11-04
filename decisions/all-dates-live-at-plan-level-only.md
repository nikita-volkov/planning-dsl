# All dates live at plan level only

Tags: plan-format

## Rationale

Two authored dates, both at plan level and both optional — `start:` (when this plan begins executing) and `deadline:` (the whole-project commitment) — and no date on any task. Task timing is the scheduler's computed output; a per-task date would be a second, hand-set timing layer the schedule must then be reconciled against. Keeping every date in the file keeps one source of truth: a what-if run ("what if we start on the 10th?") is a file edit under git, exactly like tweaking an estimate or an availability for the same experiment — not a command-line flag that leaves no trace. The scheduler reads both dates from the file, takes no date inputs, and refuses a plan with no `start`.

Both are optional because both can be honestly unknown. A young plan need not know its start. And a deadline nobody committed to is invented data, worse than none — an authoring session obliged to produce one will produce one ([an-automatic-authoring-session-invents-the-deadline](../facts/an-automatic-authoring-session-invents-the-deadline.md)); without a deadline the scheduler still produces dates and the critical path, and only slippage and latest-start lose their anchor and are omitted.

## Rejected alternatives

- Per-task deadlines and milestone dates: multiply into the renegotiation mess the plan exists to avoid, and conflict with derived start/end dates.
- One date only, `start` as a scheduler CLI flag (the predecessor's shape): rejected by the maintainer — experiments belong in the file under git, and a CLI date smears the single source of truth across inputs.
- Run-time anchoring ("start today"): breaks byte-identical output, the scheduler's defining property.
- A required deadline: forces every authoring session to invent a date; an invented deadline is noise in every report that follows.

## Evidence

Grilling session of 2026-11-04 (the scheduler spec round). Retracts `the-deadline-is-plan-level-only` (2026-11-02): its placement core — dates at plan level, never on tasks — stands; its one-date count does not.
