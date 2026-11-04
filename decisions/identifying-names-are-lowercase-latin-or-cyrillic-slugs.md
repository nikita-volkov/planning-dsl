# Identifying names are lowercase Latin or Cyrillic slugs

Tags: naming, validation, plan-format

## Rationale

The identifying names of tasks, executors and roles — the mapping keys of `tasks:`, `executors:` and `roles:` — are slugs: `^([a-z][a-z0-9]*(-[a-z0-9]+)*|[а-яё][а-яё0-9]*(-[а-яё0-9]+)*)$`. A slug starts with a lowercase letter (Latin or Cyrillic, `ё` included), continues with lowercase letters and digits, and carries hyphens only as separators — no underscores, no digit-leading names, no punctuation. One script per name: the alternation is per-name, so a name is entirely Latin or entirely Cyrillic. Free-form text belongs in description fields, never in a decorated name: names identify, descriptions describe.

Successor of identifying-names-are-kebab-case-slugs (retracted 2026-11-04). The ASCII door held for one day of demos: the house-renovation demo read Russian everywhere except its identifiers, and the maintainer chose widening the door over renderer workarounds. The human-first call: a Russian crew reads the graph, the tracker export and the schedule, and the identifier repeats in every one of them — a gloss lives only in the yaml. Names identify for humans too, and the wedge audience authors plans in its own language.

The class fix survives untouched. Still one slug rule at the door (the validator), not in the consumers: no per-renderer escaping, no exporter key-munging. Cyrillic carries no `:` — the gantt's task-line split that triggered the predecessor cannot recur — and mermaid's reserved words (`end`, `o`, `x`) are Latin-only, so the `t0…tN` fallback keeps catching exactly what it caught. The JSON Schema carries the same regex as `patternProperties`, so the door holds for schema-validating consumers too.

Script purity is part of the shape, not style: Latin `p` and Cyrillic `р` are distinct code points rendering identically. With mixed scripts admitted, `ремонт` and its Latin-p lookalike would be two legal names a reader cannot tell apart; per-name purity makes every such pair illegal.

## Rejected alternatives

- **Keep the ASCII door, show Russian glosses in the renderer**: the slug itself repeats in every render, export and conversation; a gloss lives only in the yaml and dies at the first export. Rejected by the maintainer in the same exchange.
- **Transliteration** (`peregorki-pervyj-etazh`): legal without code changes, but identifiers become neither Russian nor English — worse to read than either and hostile to search.
- **Unbounded Unicode** (`\p{L}` classes, any script): case-folding and normalization questions (ё vs e+combining, Turkish dotless i) enter for scripts no user has asked for; widening later is cheap, narrowing is not.
- **Mixed-script names admitted**: the homoglyph trap above — two legal names rendering identically.
- **A validator warning instead of an error**: inherited from the predecessor — a plan that renders broken is not warned into working; grade whether the output means anything, not whether it runs.

## Evidence

The maintainer's answer of 2026-11-04 in the house-renovation demo session — «Расширить дверь на кириллицу», asked after the ASCII refusal was demonstrated on a throwaway plan (a Cyrillic-named task, executor and role → three `name must be a … slug` errors). Enforced in `src/validate.rs` (`check_names` names every offender) and in the schema (`DeclarationMap` emits `patternProperties` from the same `NAME_PATTERN` string); restated as a hard rule in the author-plan skill. The house-renovation demo plan is named in Cyrillic slugs end to end (research/house-renovation-demo/house-renovation.yaml).
