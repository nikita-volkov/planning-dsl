# Talking to the maintainer

The maintainer answers tersely, by question number, and prefers an assessment first with a few focused questions that carry recommended answers. They engage with reasoning, not just conclusions: give the argument. The project's framing is human-first — AI agents appear as tools amplifying people, never as autonomous actors, in anything user-facing.

# The working system

The repository is a tree of modules. A module is a node of the tree; its folder holds that module's knowledge and, when there is any, its code:

| folder | holds |
|---|---|
| `facts/` | what we found to be true — the world, and our own measurements |
| `decisions/` | what the human chose, and why |
| `autodecisions/` | what agent sessions chose on their own, and why |
| `specs/` | holistic designs; code and tests are their projection |
| `tasks/` | work granules, linked into a graph by `Depends:` |
| `research/` | raw material: scripts, data, dead ends |
| `code/` | the module's package, when there is one |
| `modules/` | the child modules |

Facts, decisions and autodecisions together are the **accepted knowledge** — the knowledge base. It is binding: to deviate from a claim in it, change the file and commit — this is a solo project, there is no PR flow. Knowledge and specs are the durable asset; `tasks/` and `code/` are the disposable residents — tasks are deleted as they complete, code can be regenerated from closed knowledge.

## Where a document lives

A claim lives in the **smallest node that covers its scope**; when no child node covers it, one level up — the root is the top such node, not a special case. Applicability inherits downwards: a claim binds its whole subtree, never its siblings. Reading is lazy: from the ancestors read the index (`ls facts decisions autodecisions`), then open only what is relevant.

- A multi-subject claim decomposes into per-subject facts; the consumer links them together.
- Along its lineage a module may depend on ancestors and descendants freely. Across branches it depends only on a sibling or a sibling of an ancestor, never on a module nested inside another branch's module, and never in a cycle. Depending is not binding: the sibling's knowledge is the contract of what the module uses.
- A module's knowledge base must read with nothing beyond the modules it depends on — the self-containment test of a reusable library.
- The maintainer's tone rules apply to knowledge and specs as well.

## Facts, decisions and autodecisions

- A **fact** is settled by looking (run a command, check a link). Any agent session may add or correct one, attaching evidence. A fact is challenged by looking again.
- A **decision** is a choice the human made, settled by arguing (a choice among alternatives). Only the human accepts it; rejected alternatives are required. A decision is challenged by retracting it and proposing a new one.
- An **autodecision** is a choice an agent session made without the human. Rejected alternatives are required. Any agent session may add, change or retract one, but a change must record its new argument and answer the alternatives the old version rejected. It binds until changed.
- A decision outranks an autodecision, and never rests on one. When the human accepts an autodecision, its file moves to `decisions/`.

File format. Filename: the claim as a short kebab-case sentence, e.g. `all-writes-go-through-hasql-transaction.md`. A claim that does not fit in one short line is probably two claims. The folder carries the kind — there is no `Kind:` field.

```markdown
# <Claim, the filename in full prose>

Tags: short kebab-case tags naming what the claim binds

## Rationale

Why it is true, or why it was chosen. Self-contained: reviewers weren't in the session.

## Rejected alternatives

What else was considered and why not. Required for decisions and autodecisions.

## Evidence

Links, command output, session dates. Required for facts.
```

A claim's `Tags:` line names what it binds, in short kebab-case words — the node itself is already given by placement, so a tag adds only what placement cannot say: a sub-area inside the node, or a module the claim reaches beyond its own node. A tag that wants a bracketed qualification is a sign the claim lives in the wrong node. Keep tags few.

# Specs

A spec is a holistic design — the shape of a whole thing, held together: facts and decisions constrain it, tasks realize it, code and tests project it.

- An open design is a spec with an *Open questions* section. Code does not follow an open spec; it follows only accepted ones.
- A spec is accepted like a decision: grilled by a session, accepted by the human only.
- A spec is revised like a claim: changing its sense retracts it and writes the successor.
- An atomic choice becomes a decision only when it binds beyond one spec or outlives it; local choices live inside the spec.

# Tasks

Current work lives in `tasks/`, one file per task. Tasks are not knowledge: they are not binding and need no review.

- `ls tasks/` is the backlog. The filename is the task as an imperative kebab-case sentence, e.g. `launch-minimal-rust-scraper.md`.
- The file body says what done looks like and links the knowledge it depends on or would produce. Keep it short.
- A `Depends:` line names the tasks that must close first; it forms the directed graph of realization.
- When a task is done, delete its file in the same commit that completes it. Git history is the record; do not keep finished tasks.
- A new decision or fact discovered while working is not a task: propose it as accepted knowledge.
- Tasks are not blocked on the acceptance process; commit them directly.

# Process

This is a solo project: one maintainer working with agent sessions. There is no committee and no PR flow.

- Anyone, agents included, may propose or challenge a claim by editing and committing; facts with evidence and autodecisions may be changed by agent sessions directly.
- The human's agreement is the only acceptance of a decision or a spec. An agent never accepts one on the human's behalf; a choice an agent makes alone is an autodecision.
- Changing the sense of a claim makes it a different claim: retract the old one and write the new one, in one commit.
- Contradictions between claims, or between a claim and the code, are raised as neutral issues to the claim's author, not silently resolved by agents.
- Handoffs are throwaway session state. Anything in one worth keeping is a candidate fact, decision or autodecision.
- There are no ADRs or CONTEXT.md; git history is the decision record.

The full method — placement walkthroughs, the spec lifecycle, the task graph rules — lives in the skill `.agents/skills/project-method/SKILL.md`. This system describes itself: its own decisions sit in the root `decisions/` in domain-free wording, so the system can later be lifted into a repository of its own.
