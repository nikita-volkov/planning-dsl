# Executors are humans and agents uniformly, with optional availability and concurrency

Tags: executors, capacity

## Rationale

One `executors:` block holds humans and AI agents in the same schema: a name, the roles they fill, optional tracker `ids`. Two optional capacity fields cover the two kinds of being: `availability` — a calendar fraction, as a scalar shorthand for one whole-project period or a list of `{from, until, fraction}` periods — which humans wear (part-time, vacations); and `concurrency` — a number of parallel slots, default 1 — which agents wear (four review agents in flight). Nothing marks an executor as human or agent: the capacity fields they carry are the only difference, and the scheduler consumes both uniformly.

## Rejected alternatives

- Separate `humans:` and `agents:` blocks: the scheduler would need two code paths for the same question (how much work fits this being this week); the schema would grow a type field that gates nothing.
- Modelling agents with an inflated availability fraction: a 4-slot agent is not a 400 %-available human — concurrency is parallelism, not more calendar.
- Mandatory availability/concurrency on every executor: most executors are simply full-time, single-slot; defaults keep young plans small.

## Evidence

Grilling sessions of 2026-11-02 (three rounds); recorded in the session handoff of 2026-11-03.
