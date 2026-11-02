# CrewAI is the nearest existing schema but covers agents only

Tags: competition

## Rationale

CrewAI's `agents.yaml` + `tasks.yaml` pair is the closest living schema to this project's format: named executors with roles, tasks with dependencies. It lacks everything else in the combination: no capacity model (availability, concurrency), no deterministic scheduler, no rendered outputs, no tracker round-trip — and it is agents-only, with no notion of human executors wearing a calendar.

## Evidence

- Web research session of 2026-11-02, recorded in the handoff of 2026-11-03.
