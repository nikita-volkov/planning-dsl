# Roles are a declared top-level vocabulary enforced by validation

Tags: roles, validation

## Rationale

Every role used by a task or an executor must be declared in the plan's top-level `roles:` block, with an optional plain-language description. Validators reject undeclared or unknown roles. This kills the silent-DAG-break bug class — `dev`, `Dev` and `development` reading as three disjoint roles, so role-matching quietly assigns to nobody — at commit time, before any scheduler run. The optional descriptions feed the authoring agent: when an LLM interview decomposes work into tasks, the vocabulary tells it which roles exist and what they mean.

## Rejected alternatives

- Free-text roles: exactly the `dev`/`Dev`/`development` bug class, discovered only when a derived assignment comes back empty.
- A global fixed role taxonomy shipped by the tool: every team's role language differs; the plan must carry its own vocabulary to stay self-describing.

## Evidence

Grilling sessions of 2026-11-02 (three rounds); recorded in the session handoff of 2026-11-03.
