# Assignment is derived by role matching, and the plan stays role-only

Tags: assignment

## Rationale

Tasks carry `role:`, executors declare the roles they can fill, and who works on what is computed: an assignment is a match between the two, made by tooling against capacity. There is no `assignee:` field on a task. The plan stays a description of the work and of the team's shape; the named-person layer is a derived artifact, regenerable like every other computed output.

## Rejected alternatives

- An `assignee:` field on tasks: couples the plan's structure to a specific roster, so every staffing change edits the plan; duplicates what role-matching computes; invites hand-assignment that silently disagrees with capacity.
- Assignment stored in the plan but marked as tool-written: still a cache of a derived value inside the source of truth — see [computed-outputs-are-separate-regenerable-artifacts](computed-outputs-are-separate-regenerable-artifacts.md).

## Evidence

Grilling sessions of 2026-11-02 (three rounds); recorded in the session handoff of 2026-11-03.
