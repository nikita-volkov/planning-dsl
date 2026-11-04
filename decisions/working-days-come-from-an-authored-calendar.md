# Working days come from an authored calendar

Tags: plan-format, calendar

## Rationale

Which days are working days is authored data in `plan.yaml`, in three layers:

1. **Base, plan level** — a `calendar:` block: `workdays` (weekday names, required inside the block), `holidays` (dates off), `extra-workdays` (dates on that the weekday rule would mark off, e.g. a working Saturday). No `calendar:` block means Monday–Friday and no holidays — the honest default for a human-first tool; an executor that works otherwise declares so.
2. **Replacement, executor level** — an executor inherits the plan calendar and may replace it wholesale: `calendar: all` (every day working, no holidays — agents) or a full calendar object of the same shape. No merge, no per-role calendars: a merged calendar hides its result in glue rules nobody can recite.
3. **Correction, availability** — per-executor availability scales working days down ([availability-periods-are-sparse-overrides-on-full-days](availability-periods-are-sparse-overrides-on-full-days.md)) but cannot resurrect a non-working day; "this executor works the holidays" is a calendar replacement, not an availability trick.

The jurisdiction question — whose holidays? — is deliberately not format data. A `calendar: ru` string would hide a holiday table inside the binary: maintained per release, changing under byte-identical output, invisible to a plain read of the plan. Holidays are 10–15 authored dates a year; expanding them from legislation is the authoring skill's job at interview time, not the format's.

## Rejected alternatives

- A jurisdiction string in the format (`calendar: ru`): drags a per-release holiday database into the tool, breaking self-containment and file-anchored determinism; rejected by the maintainer 2026-11-04.
- No calendar field, all days working (the scheduler draft's shape): truthful for agents, false for humans — the plan's dates would lie about the people carrying it.
- Weekends hardcoded Mon–Fri in the tool, no field: false for agents, which work 24/7; a mixed team needs the field, not a constant.
- Merge semantics or per-role calendars at executor level: the working week becomes a function of combination rules instead of something the author wrote.

## Evidence

Grilling session of 2026-11-04 (the scheduler spec round, question 3): the maintainer asked for exact working days and proposed the base-plus-corrections shape; sub-answers 3a/3b/3d accepted by number.
