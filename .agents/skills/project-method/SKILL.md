---
name: project-method
description: The working system of this repository — where a document lives, how facts, decisions, specs and tasks are established, revised and realized. Use when placing or editing knowledge in the tree, opening or closing a spec, cutting tasks, or deciding which node owns a new claim.
---

# The project method

The repository is a tree of modules; each node holds `facts/`, `decisions/`, `autodecisions/`, `specs/`, `tasks/`, optional `research/` and `code/`, and its children in `modules/`. AGENTS.md is the constitution (always in context); this skill is the operational playbook. The system's own decisions sit in the root `decisions/` in domain-free wording, so it can be lifted out into its own repository later.

## Placing a document

1. Name the **subject**: what is this claim about — the world (a tracker's API, a scheduling algorithm's behaviour), or our machinery (validation, rendering, scheduling, sync)?
2. Walk down the tree: the document lives in the **smallest node that covers its scope**. No child covers it → one level up. The root is the top node, not a special case.
3. Multi-subject claim (e.g. "Jira bulk-edits at 50 issues and Linear at 100") → decompose into per-subject facts; the consumer links them.
4. Self-containment test for a reusable library's node: its knowledge base must read with nothing beyond the modules it depends on. If a claim needs the domain to make sense, it belongs to the consumer, not the library.
5. A thing two siblings both build on is not lifted to their common ancestor: it becomes a module of its own, and both depend on it. Along its lineage a module depends freely, up or down; across branches only on siblings and siblings of its ancestors, never on a module nested inside another branch's module, never in a cycle.

Worked examples:

- "Jira's REST API caps bulk issue edits at 50 per call" — subject is Jira → `modules/trackers/modules/jira/facts/`.
- "The mermaid renderer collapses parallel chains by default" — the renderer's own choice → `modules/renderers/modules/mermaid/decisions/`.
- "The scheduler breaks ties by critical-path length" — the scheduler's knob, even though measurements ran on real plans → `modules/scheduler/decisions/`.
- "We target Jira and Linear as the first trackers" — our choice among trackers → root `decisions/`.
- "Task and executor names share one validated identifier grammar" — used by the validator and by every renderer, owned by neither → its own module `modules/identifiers/decisions/`, which both depend on.

## Facts, decisions and autodecisions

- A **fact** is settled by looking. Establishing it needs no grilling: look it up, attach `## Evidence`, commit. An agent session may add or correct facts on its own; never a decision.
- A **decision** is the human's choice, settled by arguing. Grill the human first, record `## Rejected alternatives`, and acceptance is the human's agreement alone.
- An **autodecision** is a choice a session makes without the human: a format detail, a default, a design call the human did not weigh in on. Write it to `autodecisions/` with `## Rejected alternatives` and commit it. To change one, record what is new in the argument and answer the alternatives it rejected; a change without a new argument is churn.
- Which layer? If the human took part in the choice and agreed, it is a decision; if the session settled it alone, it is an autodecision — however confident the session is. An autodecision that contradicts a decision is wrong; one a decision would rest on is promoted with it.
- Promoting: when the human accepts an autodecision, `git mv` it into `decisions/` — the human's act alone.
- A contradiction with an existing claim is raised to its author, never silently resolved.
- Changing the sense of a claim = a different claim: retract the old file and write the new one, in one commit.

## Specs

A spec is a holistic design. It is written while a design is open, and it **outlives the implementation**: code and tests are its projection, and diverging code is a bug in the code (or the spec is deliberately revised, like a claim).

Lifecycle:

1. **Open.** The spec is born in a node's `specs/` with an *Open questions* section. Code does not follow an open spec. Tasks are cut from it as it firms up, their `Depends:` lines forming the realization graph.
2. **Accepted.** The open questions are empty and the human has accepted the design after grilling. Code and tests may now project it.
3. **Revised.** A change of sense retracts the spec and writes its successor in one commit. A spec never rots silently: a session that finds a spec with no open questions and no movement raises "revise or retire".

An atomic choice is promoted to a `decisions/` file only when it binds beyond one spec or outlives it; local choices stay inside the spec.

## Tasks

- One imperative kebab-case file per task; the body says what done looks like and links the knowledge it depends on or would produce.
- `Depends:` names the tasks that must close first — the directed graph of realization. A task without `Depends:` roots its own graph.
- A task is deleted in the same commit that completes it.

## Session duties

- Arriving in a node: `ls facts decisions autodecisions` (and `specs/`, `tasks/`) before reading texts; read ancestors by index only.
- Leaving: anything in the handoff worth keeping is a candidate fact, decision or autodecision — not a task.
- A change of sense is one commit whose message says what changed and why; mechanical moves and renames do not change sense. A fact correction carries its evidence in the body.
