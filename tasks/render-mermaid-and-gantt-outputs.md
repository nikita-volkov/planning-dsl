Render derived outputs from a plan: a mermaid DAG, an interactive HTML graph, and a Gantt chart (needs the scheduler's dates).

Done looks like: the canonical example in the spec renders a Gantt chart from the scheduler's dates; outputs are written to separate files, never into plan.yaml.

Mermaid and the interactive HTML are done (2026-11-03): `planning-dsl render plan.yaml` writes both as separate, byte-identical-per-plan artifacts (`--mmd`/`--html` to name them). The scheduler is implemented (2026-11-04): `planning-dsl schedule plan.yaml` computes the dates (start/end, active days, critical path, slippage), so only the Gantt render remains — draw it from the schedule artifact, not by recomputing it.

See: specs/plan-format.md, specs/scheduler.md, computed-outputs-are-separate-regenerable-artifacts.
