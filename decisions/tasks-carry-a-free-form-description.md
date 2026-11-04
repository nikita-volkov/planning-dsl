# Tasks carry a free-form description

Tags: tasks, naming, plan-format

## Rationale

A task carries an optional free-form `description:` — prose about the work that no slug can say. With identifying-names-are-lowercase-latin-or-cyrillic-slugs the division of labour is fixed: the name identifies (the slug, the key every consumer — scheduler, renderer, tracker — addresses), the description describes (what the work is, why it is shaped this way). The producer is the authoring skill: a grilling interview harvests this prose anyway and had nowhere to put it; the readers are the implementing agents, who open the plan and read the prose instead of re-deriving it from a name. Optional: a self-evident name needs no prose, and absence serializes away.

Roles already carry optional descriptions (roles-are-a-declared-vocabulary-enforced-by-validation); this brings tasks to the same shape.

## Rejected alternatives

- **Encoding the description in the name**: exactly what the slug rule closes off — decorated names are the bug class the door exists for.
- **Deferring the field until a consumer exists**: producer and readers both exist now — the authoring skill writes it, implementing agents read it. A field the format's own toolchain writes and reads is not speculative; waiting would leave the interview's prose homeless.
- **Descriptions on executors**: executors are capacity data and their names suffice; no consumer needs executor prose. Revisit if one appears.

## Evidence

The maintainer's «ок» of 2026-11-04 — the third of the three answers originally recorded in identifying-names-are-kebab-case-slugs (retracted 2026-11-04, successor: identifying-names-are-lowercase-latin-or-cyrillic-slugs; the record lives in git history). Implemented as `Task.description` in `src/model.rs` (optional, free-form, `skip_serializing_if` so it omits cleanly); the renderers stay functions of names and dates — the canonical example carries a description and its mermaid, HTML and gantt goldens are byte-identical.
