# Person-hours do not measure agent sessions

Tags: plan-format, estimates

## Rationale

The format treats humans and agents uniformly as executors — agents carry `concurrency` slots instead of availability — but keeps `person-hours` as the only plan unit. Nothing converts between them: how many person-hours is one agent session? The dogfood plan (2026-11-03) has three executors, two of them agent sessions, and every agent-task estimate silently assumed an exchange rate of one agent session ≈ 8 person-hours, inferred from the validator (one session, 1555 insertions — commit b459d6c) counted at a human's rate. The uniform-executor model is the format's core claim; the unit undermines it. Either estimates need a second unit for agent work, or the plan must declare a per-executor rate — both are format changes, not authoring choices.

## Evidence

- `plan.yaml` (dogfood run, 2026-11-03): executors `design-agent` and `dev-agent` with `concurrency: 2`; estimates `8` on `render-mermaid-and-html-graph` grounded as "validator-scale; the validator was one session (b459d6c)".
- `git show --stat b459d6c` — the validator: 1555 insertions, 21 files, one commit = one agent session.
- `specs/plan-format.md` (`executors-are-humans-and-agents-with-availability-and-concurrency`, `estimates-are-single-person-hours-declared-once-at-plan-level`): uniform executors, single human unit; no bridge defined between them.
