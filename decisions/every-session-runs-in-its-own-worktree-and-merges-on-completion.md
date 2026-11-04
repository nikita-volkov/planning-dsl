# Every session runs in its own worktree and merges on completion

Tags: sessions, worktrees, git

## Rationale

Two grounds, both confirmed by the maintainer: (a) parallel sessions must not share one checkout — their uncommitted state collides; (b) unfinished work stays off the main line — master advances only by completed, merged sessions. The worktree is created as soon as the session knows it will write anything — code, knowledge, docs, tasks alike; a read-only session never creates one. Knowledge-only sessions are not exempt: a code/knowledge split would need a second rule for exactly the boundary where sessions actually mix both.

On completion the session merges into master itself, after its tests pass (`cargo test` when it touched code). No per-merge human approval: with one maintainer and several parallel sessions, master would hang on open branches; human acceptance stays where the process already requires it — decisions and specs. Merges never squash: the session boundary and the individual commits are the record. When the branch's history is linear against master, the session rebases onto master and fast-forwards; when the branch is large, or its boundary should stay visible, it merges with `--no-ff`. The session resolves its own merge conflicts and completes the merge; escalate to the human only when the conflict is in the sense of a decision or a spec, not the mechanics. The worktree and the branch are deleted right after the merge — git history is the record.

Mechanics a session needs: it cannot merge into master from its own worktree — master is checked out in the primary worktree, so git refuses. The merge runs from the primary checkout: `git rebase master` in the worktree, then `git merge --ff-only session-<topic>` (linear case) or `git merge --no-ff -m "Merge <topic>"`; then `git worktree remove` before `git branch -d` — the branch delete fails while the worktree exists.

## Rejected alternatives

- Knowledge-only sessions on master: one rule for all sessions; a split doubles the boundary for no gain.
- Squash merges: they lose the session boundary and the individual commits.
- Per-merge human approval: master hangs with open branches; acceptance already has its place — decisions and specs.
- AGENTS.md-only, without a decision file: system rules live in the knowledge base.
- `--no-ff` always: superseded — linear small edits rebase-then-ff; `--no-ff` is for large branches or when the boundary should stay visible.

## Evidence

Settled with the maintainer in the plutus project on 2026-11-04 — same-named decision at commit f962b9f, merged as 956ca51 — and adopted here by the maintainer's instruction in the adoption session of 2026-11-04.
