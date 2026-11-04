# An unweighted executor's roles each draw the full day pool

Tags: scheduler, capacity

## Rationale

When an executor carries `roles:` (no `role-weights:`) and runs tasks of two different roles on the same day, each role's task draws from the full day pool: the draw rule is "its equal share of its executor's *role* pool", and without weights there is no partition — any one role can have the whole pool. With two roles active at once the executor is over-committed (up to 16 h booked against an 8 h day); the plan's tool to prevent that is `role-weights`, which pre-partitions the pool so shares can never exceed it. `concurrency` still caps how many tasks are active at once. Chosen by the implementing session on 2026-11-04, without the human.

## Rejected alternatives

- **Splitting the pool equally among the roles active that day** (implicit partitioning): invents a division the author never declared — the spec reserves partitioning for `role-weights`, and an implicit one would silently change an executor's throughput depending on what happened to run concurrently.
- **Refusing to activate a second role while one is active**: a capacity-model judgment the format never stated, refusing plans the author expressed with plain `roles:` plus `concurrency:`; and it would make an agent executor's parallelism depend on how many roles its tasks happen to wear.

## Evidence

Session of 2026-11-04 implementing `specs/scheduler.md`; follows from the draw rule (`pool = fraction × 8 h × role share`, share = 1.0 without weights) and [role-weights-split-one-executors-capacity-across-roles](../decisions/role-weights-split-one-executors-capacity-across-roles.md), which makes weights a cap, not a demand. The canonical example never hits the case (its unweighted executors carry disjoint roles per task role), so the interpretation costs nothing there.
