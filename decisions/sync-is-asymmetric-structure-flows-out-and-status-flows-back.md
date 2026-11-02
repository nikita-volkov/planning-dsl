# Sync is asymmetric: structure flows out, status flows back

Tags: sync

## Rationale

The YAML owns structure, the tracker owns status. Tasks flow out to Jira/Linear through the executors' tracker `ids`; status flows back and lands in the plan's `status:`/`blocked-reason:` fields. A structural conflict (task renamed, dependency changed in the tracker) resolves in YAML's favour; a status conflict resolves in the tracker's favour. One direction of authority per concern, so round-trips converge instead of oscillating, and the plan stays the single structural source of truth while never pretending to be a live status board.

## Rejected alternatives

- Bidirectional full sync: two writers for every field; conflicts need merge policy per field, which is where sync tools go to die.
- Tracker as full source of truth (plan generated from the tracker): loses the open, diffable, version-controlled plan file — the whole positioning.
- YAML as full source of truth (status pushed out): the plan would fight the tracker over who observed reality last; humans move cards in the tracker.

## Evidence

Grilling sessions of 2026-11-02 (three rounds); recorded in the session handoff of 2026-11-03.
