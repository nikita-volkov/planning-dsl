Settle the capacity-contention tie-break before the scheduler is designed: when two eligible tasks compete for one executor's limited capacity, what breaks the tie? No `priority:` field exists among the settled decisions; TaskJuggler's semantics include priorities.

Done looks like: one light grilling round; the rule recorded either as a scheduler default (e.g. critical-path length) or as a minimal authored field, in `decisions/` or inside the scheduler's own spec.

Drafted 2026-11-03: the rule (longest downstream path first, then declaration order) is recorded as an autodecision — [capacity-contention-goes-to-the-longest-downstream-path-then-declaration-order](../autodecisions/capacity-contention-goes-to-the-longest-downstream-path-then-declaration-order.md) — and applied in the draft [specs/scheduler.md](../specs/scheduler.md), whose Grilling section holds the five open questions. Remaining: the grilling round with the maintainer; on acceptance the autodecision moves to `decisions/` and the spec is marked accepted. (The task said `decisions/`, but the working system puts agent-chosen choices in `autodecisions/` until the human accepts.)

See: specs/plan-format.md (Layers → Scheduler), facts/taskjuggler-solves-deterministic-scheduling-and-has-been-dormant-since-2020.
