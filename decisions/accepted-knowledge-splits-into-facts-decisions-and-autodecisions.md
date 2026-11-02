# Accepted knowledge splits into facts, decisions and autodecisions

Tags: working-system

## Rationale

Three kinds of accepted knowledge have different settling procedures and different permissions, so each gets its own folder inside every node:

- A **fact** is settled by looking. It goes stale when the world changes and is challenged by looking again. An agent session may add or correct one on its own, attaching evidence.
- A **decision** is a choice the human made. It is settled by arguing, and only the human accepts it; rejected alternatives are required. It is challenged by retracting it and proposing a new one.
- An **autodecision** is a choice an agent session made on its own, without the human. It is settled by arguing like a decision, and rejected alternatives are required, but any agent session may add, change or retract it on its own, like a fact.

Choices are split by who made them because many of them are derived by agent sessions with no human taking part. Recorded as decisions, they mislabel themselves: the human's authorship stops meaning anything, a later session cannot tell which choices it may revise, and the choices the human really made are as exposed to silent drift as the rest.

An autodecision binds everyone until it is changed; a choice any session could ignore would be reopened by every session. The two layers relate by these rules:

- A decision outranks an autodecision. An autodecision that contradicts a decision is a defect of the autodecision.
- A decision does not rest on an autodecision. When the human accepts a decision that would, the autodecisions under it are promoted with it, or the decision is reworded so it no longer rests on them.
- Changing an autodecision takes a new argument: the change records what is new and answers the alternatives the old version rejected. Without that, two sessions would flip one choice back and forth.
- Promotion: when the human accepts an autodecision, its file moves to `decisions/` — the human's act alone. Demotion of a decision is a retraction, which only the human makes.

Facts stay one layer. A fact is settled by evidence, not by authority: an agent's second look is as good as the human's. Things only the human can observe — the human's own resources and constraints — are givens the human sets, so they live in `decisions/`.

The folder carries the kind; there is no `Kind:` field. The decisions recorded before this split stay in `decisions/` as the human's; the human moves any that turn out to be agent-made.

## Rejected alternatives

- One `decisions/` folder with an `Accepted-by:` field: a field gates nothing, while a folder can be listed, read in order and gated by tool permissions should that ever be wanted.
- One `truths/` folder with a `Kind:` field: the kind gates permissions and reading order, and a field gates nothing physically.
- Splitting facts by who observed them as well: evidence settles a fact, so the human's approval would only add friction to what an agent can check again.
- Leaving agent-made choices in `decisions/` under the human's name: the authorship lies, and the human's own choices lose their protection.
- Agent-made choices as non-binding proposals: every session would reopen them; being binding until changed is what makes them useful.
- Other names for the layer: `defaults/` collides with configuration defaults; `conventions/` sounds more settled than a layer meant to move; `assumptions/` reads as claims about the world, i.e. facts; `drafts/` and `proposals/` read as not yet in force; `presumptions/` is a rare word.
- Splitting by epistemic type any further (e.g. measurements apart from observations): no procedure reads the difference.
