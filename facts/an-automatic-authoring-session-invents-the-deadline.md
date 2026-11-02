# An automatic authoring session must invent the deadline

Tags: authoring-skill, dogfood

## Rationale

The author-plan skill's stage 1 (Frame) requires the single plan-level `deadline:` before anything is emitted, and itself warns that "a placeholder deadline is a lie the scheduler will treat as real". But the deadline is an act of authority — deciding when the work must land — and an automatic session has none. Running the interview without a human (the wedge dogfood run, 2026-11-03), the session had exactly two moves: refuse to emit, or invent a date. It invented 2026-12-01 (four weeks out) and could only flag the invention as an inline comment — the format has no way to distinguish a confirmed deadline from a proposed one. The skill's stages assume a human is present to answer; it has no defined automatic mode, and where it meets one, its own anti-lie rule is violated by construction.

## Evidence

- `.agents/skills/author-plan/SKILL.md`, stage 1: "do not emit a file until both are known; a placeholder deadline is a lie the scheduler will treat as real."
- `plan.yaml` line 5: `deadline: 2026-12-01  # proposed by the authoring session; maintainer to confirm` — invented at stage 1 of the dogfood run on 2026-11-03, pending maintainer correction.
- The wedge task (`tasks/write-the-grilling-authoring-skill.md`, git history) ordered the run to be automatic with self-proposed answers.
