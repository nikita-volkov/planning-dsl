Settle the capacity-contention tie-break before the scheduler is designed: when two eligible tasks compete for one executor's limited capacity, what breaks the tie? No `priority:` field exists among the settled decisions; TaskJuggler's semantics include priorities.

Done looks like: one light grilling round; the rule recorded either as a scheduler default (e.g. critical-path length) or as a minimal authored field, in `decisions/` or inside the scheduler's own spec.

See: specs/plan-format.md (Layers → Scheduler), facts/taskjuggler-solves-deterministic-scheduling-and-has-been-dormant-since-2020.
