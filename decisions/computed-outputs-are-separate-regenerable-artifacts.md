# Computed outputs are separate regenerable artifacts, never written back into the plan

Tags: outputs

## Rationale

Everything computed — assignment, maturity, schedule (start/end dates), critical path, mermaid and interactive HTML graphs, Gantt charts, tracker exports — is a disposable artifact regenerated from `plan.yaml` on demand. Nothing computed is ever cached inside the plan file. A planning tool that stores derived state in its source of truth inherits cache invalidation: every edit must find and update every stale copy, and every consumer must guess whether a stored schedule predates the last edit. Keeping the file pure description makes staleness impossible to express — there is nothing to go stale — and makes `git diff` on the plan mean exactly one thing: the world changed.

The maintainer asked for and worked through this argument before accepting it.

## Rejected alternatives

- Writing the schedule back into the file (annotated start/end per task): convenient for one render, fatal forever after — cache invalidation in a planning tool.
- A sidecar cache file beside the plan (`plan.computed.yaml`): a second file to keep in step, commit, and conflict over; regenerating is cheap and deterministic.

## Evidence

Grilling sessions of 2026-11-02 (three rounds); recorded in the session handoff of 2026-11-03.
