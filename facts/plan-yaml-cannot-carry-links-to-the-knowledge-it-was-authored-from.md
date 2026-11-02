# plan.yaml cannot carry links to the knowledge it was authored from

Tags: plan-format, authoring-skill

## Rationale

This repository's method binds every task to the knowledge it depends on or would produce (`See:` lines in `tasks/*.md`, linked decisions in specs). The plan format has no place for any of it: a task's authored fields are `status`, `role`, `depends-on`, `estimate`, `blocked-reason` — no notes, no links, no provenance. Interviewing from this repo's own backlog (dogfood run, 2026-11-03), the grounding behind each estimate — which commit scales it, which task file it came from, which fact constrains it — survived only as inline YAML comments, which the validator ignores and nothing guarantees. A plan that cannot say why an estimate is believed true is weaker than the task files it was born from.

## Evidence

- `tasks/settle-the-capacity-contention-tie-break.md` and `tasks/render-mermaid-and-gantt-outputs.md`: each carries `See:` links into facts and decisions; `plan.yaml` tasks (`settle-tie-break`, `render-mermaid-and-html-graph`) have no field for them.
- `src/model.rs`, `struct Task` — the authored-field list, no notes/links field.
- `plan.yaml` (dogfood run, 2026-11-03): grounding survives only as comments, e.g. `estimate: 8  # validator-scale or smaller; the validator was one session (b459d6c)`.
