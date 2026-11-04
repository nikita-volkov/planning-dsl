# Availability periods are sparse overrides on full days

Tags: plan-format, calendar, availability

## Rationale

One norm covers absence and sparsity alike: **nothing said — a full working day.** An executor with no `availability` works full days; a period list names the stretches that differ, and every working day outside the periods is a full day. A vacation is one `fraction: 0.0` period; part-time until a date is one `fraction: 0.5` period with `until`; what follows the last period is again a full day, unless another period says otherwise. The scalar form (`availability: 0.5`) is the degenerate case: one period spanning the whole project.

Availability scales, it never gates: whether a day is working is the calendar's verdict ([working-days-come-from-an-authored-calendar](working-days-come-from-an-authored-calendar.md)), and a fraction applies to working days only — no period can make a non-working day working.

## Rejected alternatives

- Exhaustive periods — a day in no period is zero (the scheduler draft's shape): declaring one vacation would force enumerating 1.0 across every other working day of the project; authoring noise with one more typo class.
- Availability overriding the calendar (working a holiday by a period): two systems both deciding working-ness; the executor-level calendar replacement says it in one place.

## Evidence

Grilling session of 2026-11-04 (the scheduler spec round, question 3c): accepted by number as part of the calendar design.
