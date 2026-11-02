# Status is a four-value enum, and blocked-reason is written by tooling

Tags: status

## Rationale

A task's `status:` is exactly one of `todo | in-progress | blocked | done`, plus an optional free-text `blocked-reason:` when blocked. Both fields live in the plan file but are written by sync tooling as status flows back from trackers — they are never hand-typed during authoring. The enum is small enough to round-trip cleanly through every tracker's workflow states.

## Rejected alternatives

- A richer or configurable status workflow: every added state must map onto Jira/Linear round-trips; four values already cover what flows back from a tracker.
- Hand-maintained status: the plan would disagree with the trackers within a week — see [sync-is-asymmetric-structure-flows-out-and-status-flows-back](sync-is-asymmetric-structure-flows-out-and-status-flows-back.md).
- Percent-complete fields: a progress percentage is a lie people negotiate; status plus tracker history carries the truth.

## Evidence

Grilling sessions of 2026-11-02 (three rounds); recorded in the session handoff of 2026-11-03.
