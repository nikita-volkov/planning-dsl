# Role-weights split one executor's capacity across roles

Tags: assignment, capacity

## Rationale

`role-weights` is a within-person capacity split: an executor with `{dev: 2, tester: 7}` gives 2/9 of their own time to dev work and 7/9 to test work. The weights are normalized within that one executor and say nothing about anyone else. Two executors with the same roles can thus be compared only by their absolute availability, never by their weights.

## Rejected alternatives

- A cross-team skill score (executor A is a 2-dev, executor B is a 7-dev): turns the field into a performance rating the scheduler would silently optimize on; not a capacity input; invites calibration arguments the tool cannot settle.
- Separate named capacity buckets per role with absolute hours: more fields to maintain, and the fractions carry the same information relative to the executor's availability.

## Evidence

Grilling sessions of 2026-11-02 (three rounds); recorded in the session handoff of 2026-11-03.
