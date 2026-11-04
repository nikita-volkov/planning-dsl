# Identifying names are kebab-case slugs

Tags: naming, validation, plan-format

## Rationale

The identifying names of tasks, executors and roles — the mapping keys of `tasks:`, `executors:` and `roles:` — are kebab-case slugs: `^[a-z][a-z0-9]*(-[a-z0-9]+)*$`. A slug starts with a lowercase letter, continues with lowercase letters and digits, and carries hyphens only as separators — no underscores, no digit-leading names, no punctuation. Free-form text belongs in description fields, never in a decorated name: names identify, descriptions describe.

The constraint sits at the door (the validator), not in the consumers. The trigger, 2026-11-04: the gantt renderer has no escape hatch for a name mermaid's gantt task line cannot carry — a `:` splits the line's metadata early. The maintainer's call was to fix the class, not the renderer: one slug rule at the validator retires a whole family of per-consumer machinery — mermaid escaping, HTML entities, exporter key-shape munging — that every consumer would otherwise reimplement, with the failure surfacing only at render time, farthest from the commit that wrote the name. The JSON Schema carries the same regex as `patternProperties`, so the door holds for schema-validating consumers too.

Scope, accepted in the same exchange: all three key spaces. Roles already refuse undeclared vocabulary (roles-are-a-declared-vocabulary-enforced-by-validation); the slug shape is the same door, one check earlier.

Mermaid's reserved words (`end`, `o`, `x`, …) are valid slugs and stay a renderer concern: the flowchart's `t0…tN` fallback survives for exactly that — for a slug, the only names it still catches are the reserved ones.

## Rejected alternatives

- **Per-renderer escaping and synthetic-id fallbacks for arbitrary names**: machinery every consumer reimplements, each reimplementation a fresh place to be subtly wrong; and the failure only surfaces at render time — a plan that renders broken has already passed every gate short of the eye.
- **A validator warning instead of an error**: a plan that renders broken is not warned into working — the estimate-ceiling lesson (a-task-estimated-over-a-working-month-is-refused): grade whether the output means anything, not whether it runs.
- **Digit-leading names or underscores**: mermaid node ids and future tracker keys choke on digit-leading names, and permitting underscores invites `snake_case` beside `kebab-case` — one convention, not two.

## Evidence

The maintainer's three answers of 2026-11-04 — «ок» to the slug rule, to its scope over all three key spaces, and to the `description:` field that takes the prose a slug cannot carry (tasks-carry-a-free-form-description). Trigger: the gantt render (commit 57efa88) formats each task as `    <name> :<dates>`, and a name containing `:` splits the task-line metadata early — no escape hatch exists. Enforced in `src/validate.rs` (`check_names` names every offender) and in the schema (`DeclarationMap` emits `patternProperties` from the same `NAME_PATTERN` string); restated as a hard rule in the author-plan skill.
