# Maturity is derived from estimates, never declared

Tags: estimates, scheduling

## Rationale

There is no stage field. A plan's maturity is computed: when every leaf task carries an `estimate:`, scheduling tools unlock; before that, they report which tasks still lack estimates. Declared stages would drift from reality and invite process theatre (`ready` plans that are not), while derivation is always truthful and costs nothing to check.

## Rejected alternatives

- A declared stage field (`stage: draft | estimable | schedulable`): a second source of truth that must be kept in step with the estimates; the file gains a flag that validators must cross-check and humans will forget to update.
- Gating scheduling on anything other than full leaf coverage: partial schedules read as plans but are not — the critical path through an unestimated task is a guess.

## Evidence

Grilling sessions of 2026-11-02 (three rounds); recorded in the session handoff of 2026-11-03.
