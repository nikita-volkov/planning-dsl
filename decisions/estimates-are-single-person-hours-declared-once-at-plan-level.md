# Estimates are a single person-hours figure, with the unit declared once at plan level

Tags: estimates

## Rationale

A task's `estimate:` is one number, and the unit is a plan-level declaration (`unit: person-hours`), so individual estimates stay bare numbers and unit conversion cannot drift between tasks. Person-hours is the only unit in v1: it is what capacity math (availability fractions, concurrency slots) consumes directly.

## Rejected alternatives

- Estimate ranges (best/likely/worst, PERT): a v2 feature; a single figure already forces the hard conversational work of committing to a number, which is the point of estimating during authoring.
- Story points: not convertible to capacity without a calibration table no first version would have.
- Per-task units: one more field per task to get wrong; the plan has one working calendar and one unit.

## Evidence

Grilling sessions of 2026-11-02 (three rounds); recorded in the session handoff of 2026-11-03.
